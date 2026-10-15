import { describe, it, expect } from 'vitest';
import { salaryAmounts } from './salary';
describe('salary preview arithmetic', () => {
  it('matches the workbook example and editable-divisor totals', () => {
    expect(salaryAmounts('125000', '26000', '6000', 5, '65000', 26)).toEqual({
      normal_fils: '125000',
      friday_fils: '36058',
      overtime_fils: '65104',
      total_fils: '226162',
    });
    expect(salaryAmounts('125000', '26000', '6000', 5, '65000', 25).total_fils).toBe('235208');
  });
  it('rounds individual payable components rather than intermediate hourly rates', () => {
    expect(salaryAmounts('1', '1000', '1000', 1, '800', 2)).toEqual({
      normal_fils: '1',
      friday_fils: '1',
      overtime_fils: '1',
      total_fils: '3',
    });
  });
  it('rejects invalid divisors, hours, fractional Friday counts and overflow', () => {
    expect(() => salaryAmounts('125000', '26000', '0', 5, '65000', 26)).toThrow();
    expect(() => salaryAmounts('125000', '26000', '6000', 5, '65000', 0)).toThrow();
    expect(() => salaryAmounts('125000', '26000', '6000', 1.5, '65000', 26)).toThrow();
    expect(() => salaryAmounts('9000000000000', '31000', '6000', 0, '0', 1)).toThrow();
  });
});
