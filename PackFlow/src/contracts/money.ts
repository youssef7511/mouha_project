export const MAX_MONEY = 9_000_000_000_000n;
export function parseScaled(value: string, max = MAX_MONEY): string {
  if (!/^\d+(\.\d{1,3})?$/.test(value.trim()))
    throw new Error('Use a nonnegative amount with up to three decimal places.');
  const [whole, fraction = ''] = value.trim().split('.');
  const scaled = BigInt(whole) * 1000n + BigInt(fraction.padEnd(3, '0'));
  if (scaled > max) throw new Error('Amount exceeds the supported range.');
  return scaled.toString();
}
export function decimal(scaled: string): string {
  const amount = BigInt(scaled);
  const sign = amount < 0n ? '-' : '';
  const abs = amount < 0n ? -amount : amount;
  return `${sign}${abs / 1000n}.${(abs % 1000n).toString().padStart(3, '0')}`;
}
export function money(fils: string | null): string {
  return fils === null ? 'Not priced' : `KWD ${decimal(fils)}`;
}
export function lineTotal(quantity: string, unitPrice: string): string {
  const q = BigInt(quantity),
    p = BigInt(unitPrice),
    total = (q * p + 500n) / 1000n;
  if (q <= 0n || q > 1_000_000_000n || p < 0n || p > MAX_MONEY || total > MAX_MONEY)
    throw new Error('Line total exceeds the supported range.');
  return total.toString();
}
