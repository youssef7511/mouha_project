import { describe, it, expect } from 'vitest';
import { quote } from './pricing';
import { parseScaled, lineTotal, MAX_MONEY } from './money';
import type { StagePrice } from './types';
describe('exact phone pricing', () => {
  const items = [
    { id: 'a', description: 'Small', dimensions: '', quantity: 100 },
    { id: 'b', description: 'Large', dimensions: '', quantity: 50 },
  ];
  const stage = (mode: StagePrice['pricing_mode'], fields: Partial<StagePrice>): StagePrice => ({
    template_id: mode,
    pricing_mode: mode,
    fixed_total_fils: null,
    quantity_scaled: null,
    unit_price_fils: null,
    item_prices: [],
    ...fields,
  });
  it('matches the Mac arithmetic fixtures for mixed modes and rounding', () => {
    const r = quote(items, [
      stage('fixed', { fixed_total_fils: '12500' }),
      stage('per_box', {
        item_prices: [
          { item_id: 'b', unit_price_fils: '400' },
          { item_id: 'a', unit_price_fils: '250' },
        ],
      }),
      stage('per_unit', { quantity_scaled: '3333', unit_price_fils: '125' }),
    ]);
    expect(r.total).toBe('57917');
    expect(lineTotal('3333', '125')).toBe('417');
  });
  it('keeps a zero price distinct from unpriced', () => {
    expect(quote(items, [stage('fixed', { fixed_total_fils: '0' })]).total).toBe('0');
    expect(
      quote(items, [stage('unpriced', {}), stage('fixed', { fixed_total_fils: '12000' })]),
    ).toMatchObject({ total: null, known: '12000' });
  });
  it('rejects excess precision and quote overflow', () => {
    expect(() => parseScaled('1.0001')).toThrow();
    expect(() =>
      quote(items, [
        stage('fixed', { fixed_total_fils: MAX_MONEY.toString() }),
        stage('fixed', { fixed_total_fils: '1' }),
      ]),
    ).toThrow();
  });
});
