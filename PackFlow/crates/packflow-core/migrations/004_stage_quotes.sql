-- New quotes retain each stage's commercial inputs independently of production costs.
-- Existing order-level quotes remain intact, without inventing historical stage allocations.
CREATE TABLE quote_stage_prices (
 quote_revision_id TEXT NOT NULL, order_id TEXT NOT NULL, stage_id TEXT NOT NULL,
 pricing_mode TEXT NOT NULL CHECK(pricing_mode IN ('unpriced','fixed','per_box','per_unit')),
 fixed_total_fils INTEGER CHECK(fixed_total_fils BETWEEN 0 AND 9000000000000),
 quantity_scaled INTEGER CHECK(quantity_scaled BETWEEN 1 AND 1000000000),
 unit_price_fils INTEGER CHECK(unit_price_fils BETWEEN 0 AND 9000000000000),
 derived_total_fils INTEGER CHECK(derived_total_fils BETWEEN 0 AND 9000000000000),
 PRIMARY KEY(quote_revision_id,stage_id),
 UNIQUE(quote_revision_id,order_id,stage_id),
 FOREIGN KEY(order_id,quote_revision_id) REFERENCES quote_revisions(order_id,id),
 FOREIGN KEY(order_id,stage_id) REFERENCES order_stages(order_id,id),
 CHECK((pricing_mode='unpriced' AND fixed_total_fils IS NULL AND quantity_scaled IS NULL AND unit_price_fils IS NULL AND derived_total_fils IS NULL) OR
       (pricing_mode='fixed' AND fixed_total_fils IS NOT NULL AND quantity_scaled IS NULL AND unit_price_fils IS NULL AND derived_total_fils=fixed_total_fils) OR
       (pricing_mode='per_box' AND fixed_total_fils IS NULL AND quantity_scaled IS NULL AND unit_price_fils IS NULL AND derived_total_fils IS NOT NULL) OR
       (pricing_mode='per_unit' AND fixed_total_fils IS NULL AND quantity_scaled IS NOT NULL AND unit_price_fils IS NOT NULL AND derived_total_fils IS NOT NULL))
) STRICT;
CREATE TABLE quote_stage_item_prices (
 quote_revision_id TEXT NOT NULL, order_id TEXT NOT NULL, stage_id TEXT NOT NULL, item_id TEXT NOT NULL,
 unit_price_fils INTEGER NOT NULL CHECK(unit_price_fils BETWEEN 0 AND 9000000000000),
 PRIMARY KEY(quote_revision_id,stage_id,item_id),
 FOREIGN KEY(quote_revision_id,order_id,stage_id) REFERENCES quote_stage_prices(quote_revision_id,order_id,stage_id),
 FOREIGN KEY(quote_revision_id,item_id) REFERENCES quote_items(quote_revision_id,item_id)
) STRICT;
