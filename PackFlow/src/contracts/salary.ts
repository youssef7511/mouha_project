import { MAX_MONEY } from './money';
export function salaryAmounts(
  basic: string,
  days: string,
  hours: string,
  fridays: number,
  overtime: string,
  divisor: number,
) {
  const b = BigInt(basic),
    d = BigInt(days),
    h = BigInt(hours),
    o = BigInt(overtime);
  if (
    b < 0n ||
    b > MAX_MONEY ||
    d < 0n ||
    d > 31000n ||
    h <= 0n ||
    h > 24000n ||
    o < 0n ||
    o > 744000n ||
    !Number.isInteger(fridays) ||
    fridays < 0 ||
    fridays > 31 ||
    !Number.isInteger(divisor) ||
    divisor < 1 ||
    divisor > 31
  )
    throw new Error('Enter valid days, hours and salary amounts.');
  const basis = BigInt(divisor);
  const round = (numerator: bigint, denominator: bigint) =>
    (numerator + denominator / 2n) / denominator;
  const normal = round(b * d, basis * 1000n),
    friday = round(b * BigInt(fridays) * 3n, basis * 2n),
    ot = round(b * o * 5n, basis * h * 4n);
  const total = normal + friday + ot;
  if (total > MAX_MONEY) throw new Error('Calculated salary exceeds the supported range.');
  return {
    normal_fils: normal.toString(),
    friday_fils: friday.toString(),
    overtime_fils: ot.toString(),
    total_fils: total.toString(),
  };
}
