CREATE TABLE outside_services (
 id TEXT PRIMARY KEY, order_id TEXT NOT NULL REFERENCES orders(id),
 description TEXT NOT NULL CHECK(length(trim(description))>0),
 charge_fils INTEGER NOT NULL CHECK(charge_fils BETWEEN 0 AND 9000000000000),
 expense_id TEXT NOT NULL UNIQUE, business_date TEXT NOT NULL,
 created_at TEXT NOT NULL, actor_id TEXT NOT NULL REFERENCES actors(id),
 UNIQUE(order_id,id),
 FOREIGN KEY(order_id,expense_id) REFERENCES expense_entries(order_id,id)
) STRICT;
CREATE INDEX outside_service_order ON outside_services(order_id,business_date);

-- All views derive from canonical entries; no independently editable balances.
CREATE VIEW order_financials AS
 WITH amounts AS (
 SELECT o.id AS order_id, q.derived_total_fils AS base_quote_fils,
   coalesce((SELECT sum(charge_fils) FROM outside_services WHERE order_id=o.id),0) AS service_charge_fils,
   coalesce((SELECT sum(CASE kind WHEN 'receipt' THEN amount_fils ELSE -amount_fils END) FROM payment_events WHERE order_id=o.id),0) AS paid_fils,
   coalesce((SELECT sum(CASE kind WHEN 'expense' THEN amount_fils ELSE -amount_fils END) FROM expense_entries WHERE order_id=o.id),0) AS cost_fils
 FROM orders o LEFT JOIN quote_revisions q ON q.id=o.current_quote_revision_id
 )
 SELECT *, CASE WHEN base_quote_fils IS NULL THEN NULL ELSE base_quote_fils+service_charge_fils END AS total_due_fils,
 CASE WHEN base_quote_fils IS NULL THEN NULL ELSE base_quote_fils+service_charge_fils-paid_fils END AS balance_fils
 FROM amounts;
