import { MAX_MONEY, lineTotal } from './money';
import type { Box, StagePrice } from './types';
export function quote(items: Box[], stages: StagePrice[]) {
  let known = 0n;
  let complete = stages.length > 0;
  const totals = stages.map((s) => {
    let total: bigint;
    switch (s.pricing_mode) {
      case 'unpriced':
        complete = false;
        return null;
      case 'fixed':
        if (s.fixed_total_fils === null) throw new Error('Enter the fixed stage total.');
        total = BigInt(s.fixed_total_fils);
        break;
      case 'per_unit':
        if (s.quantity_scaled === null || s.unit_price_fils === null)
          throw new Error('Enter a unit quantity and price.');
        total = BigInt(lineTotal(s.quantity_scaled, s.unit_price_fils));
        break;
      case 'per_box':
        total = items.reduce((sum, item) => {
          const rate = s.item_prices.find((p) => p.item_id === item.id);
          if (!rate) throw new Error('Price every box type.');
          return sum + BigInt(item.quantity) * BigInt(rate.unit_price_fils);
        }, 0n);
        break;
    }
    if (total < 0n || total > MAX_MONEY)
      throw new Error('Stage total exceeds the supported range.');
    known += total;
    if (known > MAX_MONEY) throw new Error('Order total exceeds the supported range.');
    return total.toString();
  });
  return { known: known.toString(), total: complete ? known.toString() : null, totals };
}
