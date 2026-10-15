CREATE TABLE actors (
 id TEXT PRIMARY KEY, display_name TEXT NOT NULL CHECK(length(trim(display_name)) BETWEEN 1 AND 100),
 active INTEGER NOT NULL DEFAULT 1 CHECK(active IN (0,1))
) STRICT;
CREATE TABLE clients (
 id TEXT PRIMARY KEY, name TEXT NOT NULL CHECK(length(trim(name)) BETWEEN 1 AND 120), normalized_name TEXT NOT NULL,
 contact_name TEXT NOT NULL DEFAULT '', email TEXT NOT NULL DEFAULT '', phone TEXT NOT NULL DEFAULT '', address TEXT NOT NULL DEFAULT '',
 archived INTEGER NOT NULL DEFAULT 0 CHECK(archived IN (0,1)), version INTEGER NOT NULL DEFAULT 1 CHECK(version>0),
 created_at TEXT NOT NULL, updated_at TEXT NOT NULL
) STRICT;
CREATE INDEX clients_name ON clients(normalized_name);
CREATE TABLE workers (
 id TEXT PRIMARY KEY, name TEXT NOT NULL, role TEXT NOT NULL DEFAULT '', phone TEXT NOT NULL DEFAULT '', email TEXT NOT NULL DEFAULT '',
 active INTEGER NOT NULL DEFAULT 1 CHECK(active IN (0,1)), archived INTEGER NOT NULL DEFAULT 0 CHECK(archived IN (0,1)),
 version INTEGER NOT NULL DEFAULT 1 CHECK(version>0), created_at TEXT NOT NULL, updated_at TEXT NOT NULL
) STRICT;
CREATE TABLE stage_templates (
 id TEXT PRIMARY KEY, name TEXT NOT NULL CHECK(length(trim(name)) BETWEEN 1 AND 80), normalized_name TEXT NOT NULL,
 default_notes TEXT NOT NULL DEFAULT '', position INTEGER NOT NULL UNIQUE,
 archived INTEGER NOT NULL DEFAULT 0 CHECK(archived IN (0,1)), version INTEGER NOT NULL DEFAULT 1 CHECK(version>0),
 created_at TEXT NOT NULL, updated_at TEXT NOT NULL
) STRICT;
CREATE UNIQUE INDEX active_template_name ON stage_templates(normalized_name) WHERE archived=0;
CREATE TABLE orders (
 id TEXT PRIMARY KEY, number TEXT NOT NULL UNIQUE, client_id TEXT NOT NULL REFERENCES clients(id), name TEXT NOT NULL CHECK(length(trim(name)) BETWEEN 1 AND 120),
 due_date TEXT NOT NULL, notes TEXT NOT NULL DEFAULT '', lifecycle TEXT NOT NULL DEFAULT 'open' CHECK(lifecycle IN ('open','closed','cancelled')),
 archived INTEGER NOT NULL DEFAULT 0 CHECK(archived IN (0,1)), current_quote_revision_id TEXT,
 cancellation_reason TEXT, version INTEGER NOT NULL DEFAULT 1 CHECK(version>0), created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
 FOREIGN KEY(id,current_quote_revision_id) REFERENCES quote_revisions(order_id,id) DEFERRABLE INITIALLY DEFERRED
) STRICT;
CREATE INDEX orders_client ON orders(client_id,lifecycle,archived);
CREATE INDEX orders_due ON orders(due_date,lifecycle,archived);
CREATE TABLE order_items (
 id TEXT PRIMARY KEY, order_id TEXT NOT NULL REFERENCES orders(id), position INTEGER NOT NULL,
 description TEXT NOT NULL CHECK(length(trim(description)) BETWEEN 1 AND 160), dimensions_text TEXT NOT NULL DEFAULT '',
 ordered_boxes INTEGER NOT NULL CHECK(ordered_boxes BETWEEN 1 AND 1000000),
 UNIQUE(order_id,id), UNIQUE(order_id,position)
) STRICT;
CREATE TABLE order_stages (
 id TEXT PRIMARY KEY, order_id TEXT NOT NULL REFERENCES orders(id), template_id TEXT REFERENCES stage_templates(id),
 name_snapshot TEXT NOT NULL, position INTEGER NOT NULL, notes TEXT NOT NULL DEFAULT '',
 state TEXT NOT NULL DEFAULT 'not_started' CHECK(state IN ('not_started','in_progress','completed')),
 retired INTEGER NOT NULL DEFAULT 0 CHECK(retired IN (0,1)), retirement_reason TEXT, retained_work_acknowledgement TEXT,
 first_completed_date TEXT, version INTEGER NOT NULL DEFAULT 1 CHECK(version>0), UNIQUE(order_id,id), UNIQUE(order_id,position)
) STRICT;
CREATE TABLE stage_assignments (
 id TEXT PRIMARY KEY, stage_id TEXT NOT NULL REFERENCES order_stages(id), worker_id TEXT NOT NULL REFERENCES workers(id),
 assigned_at TEXT NOT NULL, assigned_actor_id TEXT NOT NULL REFERENCES actors(id), unassigned_at TEXT
) STRICT;
CREATE UNIQUE INDEX active_stage_worker ON stage_assignments(stage_id,worker_id) WHERE unassigned_at IS NULL;
CREATE TABLE quote_revisions (
 id TEXT PRIMARY KEY, order_id TEXT NOT NULL REFERENCES orders(id), revision_no INTEGER NOT NULL CHECK(revision_no>0),
 purpose TEXT NOT NULL DEFAULT 'normal' CHECK(purpose IN ('normal','cancellation')),
 pricing_mode TEXT NOT NULL CHECK(pricing_mode IN ('unpriced','fixed','per_box')),
 fixed_total_fils INTEGER CHECK(fixed_total_fils BETWEEN 0 AND 9000000000000),
 derived_total_fils INTEGER CHECK(derived_total_fils BETWEEN 0 AND 9000000000000),
 reason TEXT, business_date TEXT NOT NULL, created_at TEXT NOT NULL, actor_id TEXT NOT NULL REFERENCES actors(id),
 UNIQUE(order_id,id), UNIQUE(order_id,revision_no),
 CHECK((pricing_mode='unpriced' AND fixed_total_fils IS NULL AND derived_total_fils IS NULL) OR
       (pricing_mode='fixed' AND fixed_total_fils IS NOT NULL AND derived_total_fils=fixed_total_fils) OR
       (pricing_mode='per_box' AND fixed_total_fils IS NULL AND derived_total_fils IS NOT NULL))
) STRICT;
CREATE TABLE quote_items (
 quote_revision_id TEXT NOT NULL, order_id TEXT NOT NULL, item_id TEXT NOT NULL,
 description_snapshot TEXT NOT NULL, dimensions_snapshot TEXT NOT NULL,
 ordered_boxes_snapshot INTEGER NOT NULL CHECK(ordered_boxes_snapshot BETWEEN 1 AND 1000000),
 unit_price_fils INTEGER CHECK(unit_price_fils BETWEEN 0 AND 9000000000000),
 PRIMARY KEY(quote_revision_id,item_id),
 FOREIGN KEY(order_id,quote_revision_id) REFERENCES quote_revisions(order_id,id),
 FOREIGN KEY(order_id,item_id) REFERENCES order_items(order_id,id)
) STRICT;
CREATE TABLE work_events (
 id TEXT PRIMARY KEY, order_id TEXT NOT NULL REFERENCES orders(id), stage_id TEXT NOT NULL, item_id TEXT,
 kind TEXT NOT NULL CHECK(kind IN ('work','reversal')), original_id TEXT,
 business_date TEXT NOT NULL, description TEXT NOT NULL, dimensions_text TEXT NOT NULL DEFAULT '',
 input_scaled INTEGER CHECK(input_scaled BETWEEN 0 AND 1000000000), output_scaled INTEGER CHECK(output_scaled BETWEEN 0 AND 1000000000),
 waste_scaled INTEGER CHECK(waste_scaled BETWEEN 0 AND 1000000000), worker_id TEXT REFERENCES workers(id),
 worker_minutes INTEGER CHECK(worker_minutes BETWEEN 0 AND 1000000), machine_description TEXT NOT NULL DEFAULT '',
 machine_minutes INTEGER CHECK(machine_minutes BETWEEN 0 AND 1000000), reason TEXT,
 created_at TEXT NOT NULL, actor_id TEXT NOT NULL REFERENCES actors(id), UNIQUE(order_id,id),
 FOREIGN KEY(order_id,stage_id) REFERENCES order_stages(order_id,id),
 FOREIGN KEY(order_id,item_id) REFERENCES order_items(order_id,id),
 FOREIGN KEY(order_id,original_id) REFERENCES work_events(order_id,id),
 CHECK((kind='work' AND original_id IS NULL) OR (kind='reversal' AND original_id IS NOT NULL AND length(trim(reason))>0))
) STRICT;
CREATE UNIQUE INDEX reversed_work_once ON work_events(original_id) WHERE kind='reversal';
CREATE TABLE expense_entries (
 id TEXT PRIMARY KEY, order_id TEXT NOT NULL REFERENCES orders(id), stage_id TEXT, item_id TEXT, work_event_id TEXT,
 worker_id TEXT REFERENCES workers(id), kind TEXT NOT NULL CHECK(kind IN ('expense','reversal')), original_id TEXT,
 category TEXT NOT NULL CHECK(category IN ('material','outsourced','machine','labor','transport','other')),
 description TEXT NOT NULL CHECK(length(trim(description))>0), supplier_text TEXT NOT NULL DEFAULT '', reference TEXT NOT NULL DEFAULT '',
 pricing_basis TEXT NOT NULL CHECK(pricing_basis IN ('unit','total')),
 quantity_scaled INTEGER CHECK(quantity_scaled BETWEEN 1 AND 1000000000), unit_cost_fils INTEGER CHECK(unit_cost_fils BETWEEN 0 AND 9000000000000),
 amount_fils INTEGER NOT NULL CHECK(amount_fils BETWEEN 0 AND 9000000000000),
 business_date TEXT NOT NULL, reason TEXT, created_at TEXT NOT NULL, actor_id TEXT NOT NULL REFERENCES actors(id), UNIQUE(order_id,id),
 FOREIGN KEY(order_id,stage_id) REFERENCES order_stages(order_id,id),
 FOREIGN KEY(order_id,item_id) REFERENCES order_items(order_id,id),
 FOREIGN KEY(order_id,work_event_id) REFERENCES work_events(order_id,id),
 FOREIGN KEY(order_id,original_id) REFERENCES expense_entries(order_id,id),
 CHECK((pricing_basis='unit' AND quantity_scaled IS NOT NULL AND unit_cost_fils IS NOT NULL) OR (pricing_basis='total' AND unit_cost_fils IS NULL)),
 CHECK((kind='expense' AND original_id IS NULL) OR (kind='reversal' AND original_id IS NOT NULL AND length(trim(reason))>0))
) STRICT;
CREATE UNIQUE INDEX reversed_expense_once ON expense_entries(original_id) WHERE kind='reversal';
CREATE INDEX expense_order_date ON expense_entries(order_id,business_date,created_at);
CREATE INDEX expense_stage_date ON expense_entries(stage_id,business_date);
CREATE INDEX work_stage_date ON work_events(stage_id,business_date);
CREATE TABLE production_events (
 id TEXT PRIMARY KEY, order_id TEXT NOT NULL REFERENCES orders(id), stage_id TEXT, kind TEXT NOT NULL CHECK(kind IN ('production','reversal')),
 original_id TEXT, business_date TEXT NOT NULL, notes TEXT NOT NULL DEFAULT '', reason TEXT,
 created_at TEXT NOT NULL, actor_id TEXT NOT NULL REFERENCES actors(id), UNIQUE(order_id,id),
 FOREIGN KEY(order_id,stage_id) REFERENCES order_stages(order_id,id), FOREIGN KEY(order_id,original_id) REFERENCES production_events(order_id,id),
 CHECK((kind='production' AND original_id IS NULL) OR (kind='reversal' AND original_id IS NOT NULL AND length(trim(reason))>0))
) STRICT;
CREATE TABLE production_lines (
 event_id TEXT NOT NULL, order_id TEXT NOT NULL, item_id TEXT NOT NULL, quantity_boxes INTEGER NOT NULL CHECK(quantity_boxes BETWEEN 1 AND 1000000),
 PRIMARY KEY(event_id,item_id), FOREIGN KEY(order_id,event_id) REFERENCES production_events(order_id,id), FOREIGN KEY(order_id,item_id) REFERENCES order_items(order_id,id)
) STRICT;
CREATE TABLE dispatch_events (
 id TEXT PRIMARY KEY, order_id TEXT NOT NULL REFERENCES orders(id), kind TEXT NOT NULL CHECK(kind IN ('dispatch','reversal')), original_id TEXT,
 business_date TEXT NOT NULL, recipient_snapshot TEXT NOT NULL DEFAULT '', address_snapshot TEXT NOT NULL DEFAULT '', carrier TEXT NOT NULL DEFAULT '',
 reference TEXT NOT NULL DEFAULT '', notes TEXT NOT NULL DEFAULT '', reason TEXT, created_at TEXT NOT NULL, actor_id TEXT NOT NULL REFERENCES actors(id),
 UNIQUE(order_id,id), FOREIGN KEY(order_id,original_id) REFERENCES dispatch_events(order_id,id),
 CHECK((kind='dispatch' AND original_id IS NULL) OR (kind='reversal' AND original_id IS NOT NULL AND length(trim(reason))>0))
) STRICT;
CREATE TABLE dispatch_lines (
 event_id TEXT NOT NULL, order_id TEXT NOT NULL, item_id TEXT NOT NULL, quantity_boxes INTEGER NOT NULL CHECK(quantity_boxes BETWEEN 1 AND 1000000),
 PRIMARY KEY(event_id,item_id), FOREIGN KEY(order_id,event_id) REFERENCES dispatch_events(order_id,id), FOREIGN KEY(order_id,item_id) REFERENCES order_items(order_id,id)
) STRICT;
CREATE TABLE payment_events (
 id TEXT PRIMARY KEY, order_id TEXT NOT NULL REFERENCES orders(id), kind TEXT NOT NULL CHECK(kind IN ('receipt','refund')),
 amount_fils INTEGER NOT NULL CHECK(amount_fils BETWEEN 1 AND 9000000000000), business_date TEXT NOT NULL,
 method TEXT NOT NULL CHECK(method IN ('cash','bank','card','other')), reference TEXT NOT NULL DEFAULT '', notes TEXT NOT NULL DEFAULT '', reason TEXT,
 created_at TEXT NOT NULL, actor_id TEXT NOT NULL REFERENCES actors(id), UNIQUE(order_id,id)
) STRICT;
CREATE TABLE refund_allocations (
 refund_event_id TEXT NOT NULL, receipt_event_id TEXT NOT NULL, order_id TEXT NOT NULL,
 amount_fils INTEGER NOT NULL CHECK(amount_fils BETWEEN 1 AND 9000000000000), PRIMARY KEY(refund_event_id,receipt_event_id),
 FOREIGN KEY(order_id,refund_event_id) REFERENCES payment_events(order_id,id), FOREIGN KEY(order_id,receipt_event_id) REFERENCES payment_events(order_id,id),
 CHECK(refund_event_id<>receipt_event_id)
) STRICT;
CREATE INDEX production_order_date ON production_events(order_id,business_date,created_at);
CREATE INDEX dispatch_order_date ON dispatch_events(order_id,business_date,created_at);
CREATE INDEX payment_order_date ON payment_events(order_id,business_date,created_at);
CREATE INDEX production_item ON production_lines(item_id,event_id);
CREATE INDEX dispatch_item ON dispatch_lines(item_id,event_id);
CREATE INDEX refund_receipt ON refund_allocations(receipt_event_id);
CREATE TABLE fulfillment_milestones (
 order_id TEXT PRIMARY KEY REFERENCES orders(id), first_full_dispatch_at TEXT NOT NULL, business_date TEXT NOT NULL, due_date_snapshot TEXT NOT NULL
) STRICT;
CREATE TABLE audit_events (
 id TEXT PRIMARY KEY, actor_id TEXT NOT NULL REFERENCES actors(id), request_id TEXT NOT NULL, entity_type TEXT NOT NULL, entity_id TEXT NOT NULL,
 action TEXT NOT NULL, business_date TEXT NOT NULL, created_at TEXT NOT NULL, reason TEXT, before_json TEXT, after_json TEXT
) STRICT;
CREATE INDEX audit_entity ON audit_events(entity_type,entity_id,created_at);
CREATE TABLE mutation_requests (
 request_id TEXT PRIMARY KEY, command_name TEXT NOT NULL, payload_hash TEXT NOT NULL, result_json TEXT NOT NULL, committed_at TEXT NOT NULL
) STRICT;
CREATE TABLE import_batches (
 id TEXT PRIMARY KEY, dataset TEXT NOT NULL, content_hash TEXT NOT NULL, imported_at TEXT NOT NULL,
 actor_id TEXT NOT NULL REFERENCES actors(id), row_count INTEGER NOT NULL CHECK(row_count>0), request_id TEXT NOT NULL, override_reason TEXT
) STRICT;
CREATE TABLE sequences (name TEXT PRIMARY KEY, next_value INTEGER NOT NULL CHECK(next_value>0)) STRICT;
CREATE TABLE settings (key TEXT PRIMARY KEY, value_json TEXT NOT NULL, updated_at TEXT NOT NULL) STRICT;
CREATE TABLE backup_runs (
 id TEXT PRIMARY KEY, kind TEXT NOT NULL CHECK(kind IN ('daily','manual','pre_upgrade','safety')),
 target_path TEXT NOT NULL, started_at TEXT NOT NULL, completed_at TEXT, status TEXT NOT NULL CHECK(status IN ('running','success','failed')),
 schema_version INTEGER NOT NULL, checksum TEXT, error_code TEXT
) STRICT;
