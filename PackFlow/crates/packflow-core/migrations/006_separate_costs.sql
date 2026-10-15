CREATE TABLE separate_costs (
 id TEXT PRIMARY KEY,
 category TEXT NOT NULL CHECK(category IN ('machine','finish','other')),
 amount_fils INTEGER NOT NULL CHECK(amount_fils BETWEEN 0 AND 9000000000000),
 description TEXT NOT NULL CHECK(length(trim(description)) BETWEEN 1 AND 1000),
 business_date TEXT NOT NULL, created_at TEXT NOT NULL, actor_id TEXT NOT NULL REFERENCES actors(id)
) STRICT;
CREATE INDEX separate_costs_date ON separate_costs(business_date,created_at);
