import { test, expect } from '@playwright/test';
test('monthly divisor, salary calculation, editing, archive and history remain connected', async ({
  page,
  request,
}) => {
  const boot = await (await request.post('/api/query', { data: { kind: 'bootstrap' } })).json();
  const name = `Salary worker ${Date.now()}`;
  const workerResponse = await request.post('/api/mutate', {
    data: {
      request_id: crypto.randomUUID(),
      actor_id: boot.actors[0].id,
      action: {
        kind: 'save_worker',
        id: null,
        expected_version: null,
        name,
        role: 'Operator',
        active: true,
      },
    },
  });
  expect(workerResponse.ok()).toBeTruthy();
  const wid = (await workerResponse.json()).data.id;
  const errors: string[] = [];
  page.on('pageerror', (e) => errors.push(e.message));
  await page.goto('/');
  await expect(page.getByText('Local database connected')).toBeVisible();
  await page.getByRole('button', { name: 'Workers', exact: true }).click();
  await page.getByLabel('Salary month', { exact: true }).fill('2035-05');
  await expect(page.getByRole('heading', { name: 'Worker salaries · 2035-05' })).toBeVisible();
  const divisor = page.getByLabel('Monthly salary divisor');
  if ((await divisor.inputValue()) !== '26') {
    await divisor.fill('26');
    await page.getByRole('button', { name: 'Save divisor', exact: true }).click();
    await expect(divisor).toHaveValue('26');
  }
  await page.getByRole('button', { name: 'Add monthly salary', exact: true }).click();
  let dialog = page.getByRole('dialog');
  await dialog.getByRole('combobox', { name: 'Worker name', exact: true }).selectOption(wid);
  await dialog.getByLabel('Basic salary').fill('125.000');
  await dialog.getByLabel('Normal days worked').fill('26');
  await dialog.getByLabel('Working hours / day').fill('6');
  await dialog.getByLabel('Fridays worked').fill('5');
  await dialog.getByLabel('Overtime · hours').fill('65');
  await expect(dialog.locator('.quote-summary')).toContainText('KWD 226.162');
  await dialog.getByRole('button', { name: 'Save monthly salary', exact: true }).click();
  await expect(dialog).not.toBeVisible();
  const row = page.locator('.salary-records tbody tr').filter({ hasText: name });
  await expect(row.getByText('KWD 226.162', { exact: true })).toBeVisible();
  await divisor.fill('25');
  await page.getByRole('button', { name: 'Save divisor', exact: true }).click();
  await expect(row.getByText('KWD 235.208', { exact: true })).toBeVisible();
  await row.getByRole('button', { name: `Edit salary for ${name}`, exact: true }).click();
  dialog = page.getByRole('dialog');
  await expect(dialog.getByLabel('Basic salary')).toHaveValue('125.000');
  await dialog.getByLabel('Fridays worked').fill('2');
  await dialog.getByLabel('Overtime · hours').fill('10');
  await expect(dialog.locator('.quote-summary')).toContainText('KWD 155.417');
  await dialog.getByRole('button', { name: 'Save monthly salary', exact: true }).click();
  await expect(dialog).not.toBeVisible();
  await expect(row.getByText('KWD 155.417', { exact: true })).toBeVisible();
  await page.screenshot({ path: 'test-results/salaries.png', fullPage: true });
  await row.getByRole('button', { name: `Archive salary for ${name}`, exact: true }).click();
  await expect(row).toHaveCount(0);
  await page.getByLabel('Show archived salaries').check();
  await expect(row.getByText('Archived salary', { exact: true })).toBeVisible();
  await row.getByRole('button', { name: `Restore salary for ${name}`, exact: true }).click();
  await expect(
    row.getByRole('button', { name: `Edit salary for ${name}`, exact: true }),
  ).toBeVisible();
  await page.getByLabel('Salary month', { exact: true }).fill('2035-06');
  await expect(page.getByRole('heading', { name: 'Worker salaries · 2035-06' })).toBeVisible();
  await expect(divisor).toHaveValue('26');
  await expect(row).toHaveCount(0);
  await page.getByRole('combobox', { name: 'Saved months' }).selectOption('2035-05');
  await expect(row.getByText('KWD 155.417', { exact: true })).toBeVisible();
  await expect(divisor).toHaveValue('25');
  await page.reload();
  await page.getByRole('button', { name: 'Workers', exact: true }).click();
  await page.getByLabel('Salary month', { exact: true }).fill('2035-05');
  await expect(row.getByText('KWD 155.417', { exact: true })).toBeVisible();
  await expect(page.locator('.salary-history')).toContainText('Monthly divisor set to 25 days');
  const data = await (
    await request.post('/api/query', { data: { kind: 'salaries', month: '2035-05' } })
  ).json();
  const record = data.records.find((r: { worker_id: string }) => r.worker_id === wid);
  expect(record.normal_fils).toBe('130000');
  expect(record.friday_fils).toBe('15000');
  expect(record.overtime_fils).toBe('10417');
  expect(record.archived).toBe(0);
  expect(errors).toEqual([]);
});
