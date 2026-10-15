CREATE TABLE service_voids (
 original_id TEXT PRIMARY KEY REFERENCES outside_services(id), business_date TEXT NOT NULL,
 reason TEXT NOT NULL, created_at TEXT NOT NULL, actor_id TEXT NOT NULL REFERENCES actors(id)
) STRICT;
CREATE TABLE separate_cost_voids (
 original_id TEXT PRIMARY KEY REFERENCES separate_costs(id), business_date TEXT NOT NULL,
 reason TEXT NOT NULL, created_at TEXT NOT NULL, actor_id TEXT NOT NULL REFERENCES actors(id)
) STRICT;
DROP VIEW order_financials;
CREATE VIEW order_financials AS
 WITH amounts AS (
 SELECT o.id AS order_id, q.derived_total_fils AS base_quote_fils,
   coalesce((SELECT sum(charge_fils) FROM outside_services s WHERE order_id=o.id AND NOT EXISTS(SELECT 1 FROM service_voids v WHERE v.original_id=s.id)),0) AS service_charge_fils,
   coalesce((SELECT sum(CASE kind WHEN 'receipt' THEN amount_fils ELSE -amount_fils END) FROM payment_events WHERE order_id=o.id),0) AS paid_fils,
   coalesce((SELECT sum(CASE kind WHEN 'expense' THEN amount_fils ELSE -amount_fils END) FROM expense_entries WHERE order_id=o.id),0) AS cost_fils
 FROM orders o LEFT JOIN quote_revisions q ON q.id=o.current_quote_revision_id
 )
 SELECT *, CASE WHEN base_quote_fils IS NULL THEN NULL ELSE base_quote_fils+service_charge_fils END AS total_due_fils,
 CASE WHEN base_quote_fils IS NULL THEN NULL ELSE base_quote_fils+service_charge_fils-paid_fils END AS balance_fils
 FROM amounts;
