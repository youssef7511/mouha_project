import { test, expect } from '@playwright/test';
test('save machine, finish and other costs in one dialog and retain independent order totals', async ({
  page,
  request,
}) => {
  const boot = await (await request.post('/api/query', { data: { kind: 'bootstrap' } })).json();
  const prefix = `Separate cost ${Date.now()}`;
  const reports = async () =>
    (
      await request.post('/api/query', {
        data: { kind: 'reports', from_date: boot.business_today, to_date: boot.business_today },
      })
    ).json();
  const before = await reports();
  const errors: string[] = [];
  page.on('pageerror', (e) => errors.push(e.message));
  await page.goto('/');
  await expect(page.getByText('Local database connected')).toBeVisible();
  await page.getByRole('button', { name: 'Costs', exact: true }).click();
  await page.getByRole('button', { name: 'Add separate costs', exact: true }).click();
  const dialog = page.getByRole('dialog');
  for (const [index, category, amount] of [
    [0, 'machine', '12.345'],
    [1, 'finish', '5.000'],
    [2, 'other', '1.250'],
  ] as const) {
    await dialog
      .getByRole('combobox', { name: 'Cost category', exact: true })
      .selectOption(category);
    await dialog.getByLabel('Cost amount').fill(amount);
    await dialog.getByLabel('Cost description').fill(`${prefix} ${category}`);
    await dialog.getByRole('button', { name: 'Save & add another', exact: true }).click();
    await expect(dialog).toBeVisible();
    await expect(dialog.locator('.separate-cost-saved li')).toHaveCount(index + 1);
    await expect(dialog.getByLabel('Cost amount')).toHaveValue('');
    await expect(dialog.getByLabel('Cost description')).toHaveValue('');
    await expect(dialog.getByRole('combobox', { name: 'Cost category', exact: true })).toHaveValue(
      category,
    );
  }
  await expect(dialog.getByRole('status')).toContainText('3 entries saved');
  await page.screenshot({ path: 'test-results/separate-costs-dialog.png' });
  await dialog.getByRole('button', { name: 'Save & add another', exact: true }).click();
  await expect(dialog.getByText('Describe this cost.', { exact: true })).toBeVisible();
  await expect(dialog.locator('.separate-cost-saved li')).toHaveCount(3);
  await dialog.getByRole('button', { name: 'Done', exact: true }).click();
  await expect(dialog).not.toBeVisible();
  const rows = page.locator('.separate-costs tbody tr').filter({ hasText: prefix });
  await expect(rows).toHaveCount(3);
  await expect(rows.filter({ hasText: `${prefix} machine` })).toContainText('KWD 12.345');
  const costs = await (
    await request.post('/api/query', { data: { kind: 'separate_costs' } })
  ).json();
  expect(
    costs.records.filter((c: { description: string }) => c.description.startsWith(prefix)),
  ).toHaveLength(3);
  const after = await reports();
  const sum = (activity: { cost_fils: string }[]) =>
    activity.reduce((total, r) => total + BigInt(r.cost_fils), 0n);
  expect(sum(after.activity) - sum(before.activity)).toBe(18595n);
  expect(after.portfolio).toEqual(before.portfolio);
  await page.reload();
  await page.getByRole('button', { name: 'Costs', exact: true }).click();
  await expect(rows).toHaveCount(3);
  await page.screenshot({ path: 'test-results/separate-costs.png', fullPage: true });
  await page.getByRole('button', { name: 'Reports', exact: true }).click();
  await expect(page.getByText('Separate costs · Machine', { exact: true })).toBeVisible();
  await expect(page.getByText('Separate costs · Finish', { exact: true })).toBeVisible();
  await expect(page.getByText('Separate costs · Others', { exact: true })).toBeVisible();
  expect(errors).toEqual([]);
});
