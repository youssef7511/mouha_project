import { describe, it, expect } from 'vitest';
import { parseScaled, decimal, lineTotal, money } from './money';
describe('KWD and Unit values', () => {
  it('preserves fils and distinguishes missing pricing from zero', () => {
    expect(parseScaled('0.001')).toBe('1');
    expect(decimal('45000')).toBe('45.000');
    expect(money(null)).toBe('Not priced');
    expect(money('0')).toBe('KWD 0.000');
  });
  it('rounds once at the line boundary with the Rust fixture', () => {
    expect(lineTotal(parseScaled('1.125'), parseScaled('0.333'))).toBe('375');
    expect(lineTotal('500', '1')).toBe('1');
    expect(lineTotal('499', '1')).toBe('0');
  });
  it('rejects excess precision, signs, nonfinite numbers and overflow', () => {
    for (const s of ['0.0001', '-1', 'NaN', 'Infinity', '1e3', ''])
      expect(() => parseScaled(s)).toThrow();
    expect(() => parseScaled('9000000000.001')).toThrow();
  });
});
