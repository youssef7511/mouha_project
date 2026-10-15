import type { OrderDetail } from '../contracts';
import { decimal, money } from '../contracts/money';
export function StageQuoteBreakdown({ detail }: { detail: OrderDetail }) {
  if (!detail.stage_prices.length) return null;
  return (
    <div className="stage-quote-breakdown">
      <h3>Customer prices by stage</h3>
      <div className="table-wrap">
        <table>
          <thead>
            <tr>
              <th>STAGE</th>
              <th>PRICING</th>
              <th>CUSTOMER AMOUNT</th>
            </tr>
          </thead>
          <tbody>
            {detail.stage_prices.map((price) => (
              <tr key={price.stage_id}>
                <td>{price.name_snapshot}</td>
                <td>
                  {price.pricing_mode === 'fixed'
                    ? 'Fixed total quote'
                    : price.pricing_mode === 'per_unit'
                      ? `${decimal(price.quantity_scaled!)} Unit × ${money(price.unit_price_fils)}`
                      : price.pricing_mode === 'unpriced'
                        ? 'Not priced yet'
                        : 'Price per box type'}
                  {price.pricing_mode === 'per_box' &&
                    detail.items.map((item) => {
                      const rate = detail.stage_item_prices.find(
                        (p) => p.stage_id === price.stage_id && p.item_id === item.id,
                      );
                      return rate ? (
                        <small key={item.id}>
                          {item.description}: {item.ordered_boxes} boxes ×{' '}
                          {money(rate.unit_price_fils)}
                        </small>
                      ) : null;
                    })}
                </td>
                <td>{money(price.total_fils)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
