ALTER TABLE orders ADD COLUMN payment_due_date TEXT
 CHECK(payment_due_date IS NULL OR (length(payment_due_date)=10 AND payment_due_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'));
CREATE INDEX orders_payment_due ON orders(client_id,payment_due_date);
