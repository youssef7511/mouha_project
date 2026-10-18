import { test, expect } from '@playwright/test';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
const name = `Phone ${Date.now()}`;
test('phone workflow: client, mixed boxes/stage pricing, deposit, revisions and receipt correction', async ({
  page,
}) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'New order', exact: true }).click();
  await page.getByRole('button', { name: 'Add client from order' }).click();
  await page.getByLabel('Client name', { exact: true }).fill(name);
  await page.getByLabel('Phone', { exact: true }).fill('0012345');
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(page.getByRole('dialog', { name: 'New order' })).toBeVisible();
  await page.getByLabel('Order name', { exact: true }).fill('Mobile gift boxes');
  const boxes = page
    .locator('.form-card')
    .filter({ has: page.getByRole('button', { name: /Remove box type/ }) });
  await boxes.nth(0).getByLabel('Description', { exact: true }).fill('Small');
  await boxes.nth(0).getByLabel('Ordered boxes', { exact: true }).fill('100');
  await page.getByRole('button', { name: 'Add box type', exact: true }).click();
  await boxes.nth(1).getByLabel('Description', { exact: true }).fill('Large');
  await boxes.nth(1).getByLabel('Ordered boxes', { exact: true }).fill('50');
  const lam = page
    .locator('.stage-card')
    .filter({ has: page.getByLabel('Customer pricing · Lamination', { exact: true }) });
  await lam.getByRole('combobox').selectOption('fixed');
  await lam.getByLabel('Fixed total · KWD', { exact: true }).fill('12.500');
  const impose = page
    .locator('.stage-card')
    .filter({ has: page.getByLabel('Customer pricing · Impose', { exact: true }) });
  await impose.getByRole('combobox').selectOption('per_box');
  await impose.getByLabel('Small · KWD / box', { exact: true }).fill('0.250');
  await impose.getByLabel('Large · KWD / box', { exact: true }).fill('0.400');
  const diecut = page
    .locator('.stage-card')
    .filter({ has: page.getByLabel('Customer pricing · Diecut', { exact: true }) });
  await diecut.getByRole('combobox').selectOption('per_unit');
  await diecut.getByLabel('Quantity · Unit', { exact: true }).fill('3.333');
  await diecut.getByLabel('Price per Unit · KWD', { exact: true }).fill('0.125');
  const finish = page
    .locator('.stage-card')
    .filter({ has: page.getByLabel('Customer pricing · Finish', { exact: true }) });
  await finish.getByRole('combobox').selectOption('fixed');
  await finish.getByLabel('Fixed total · KWD', { exact: true }).fill('0.000');
  await expect(page.locator('.quote-summary')).toContainText('KWD 57.917');
  await page.getByLabel('Record a deposit with this order').check();
  await page.getByLabel('Amount · KWD', { exact: true }).fill('10.000');
  await page.getByRole('button', { name: 'Save order', exact: true }).click();
  await expect(page.locator('.page-title')).toContainText('Mobile gift boxes');
  await expect(page.locator('.metrics')).toContainText('KWD 47.917');
  await page.reload();
  await page.getByRole('button', { name: /MOB-.*Mobile gift boxes/ }).click();
  await expect(page.locator('.metrics')).toContainText('KWD 47.917');
  await page.getByRole('button', { name: 'Payment', exact: true }).click();
  await page.getByLabel('Amount · KWD', { exact: true }).fill('5.000');
  await page.getByRole('button', { name: 'Record payment', exact: true }).click();
  await expect(page.locator('.metrics')).toContainText('KWD 42.917');
  const row = page
    .locator('.payment-card')
    .filter({ has: page.getByRole('heading', { name: 'KWD 5.000', exact: true }) });
  await row.getByRole('button', { name: 'Correct', exact: true }).click();
  await page.getByLabel('Reason', { exact: true }).fill('Duplicate entry');
  await page.getByRole('button', { name: 'Reverse receipt', exact: true }).click();
  await expect(page.locator('.metrics')).toContainText('KWD 47.917');
  await page.getByRole('button', { name: 'Revise order', exact: true }).click();
  await boxes.nth(0).getByLabel('Ordered boxes', { exact: true }).fill('200');
  await page.getByLabel('Reason for revision').fill('Customer added more small boxes');
  await page.getByRole('button', { name: 'Save revision', exact: true }).click();
  await expect(page.locator('.quote-summary')).toContainText('KWD 82.917');
  await expect(page.locator('.metrics')).toContainText('KWD 72.917');
  await mkdir('test-results', { recursive: true });
  await page.screenshot({ path: 'test-results/phone-order.png', fullPage: true });
  expect(
    await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth),
  ).toBeTruthy();
  await page.getByRole('button', { name: 'Quote history', exact: true }).click();
  await expect(page.getByRole('dialog')).toContainText('Revision 1 · KWD 57.917');
  await expect(page.getByRole('dialog')).toContainText('Revision 2 · KWD 82.917');
});
test('native phone UI performs saves without browser network access (real core through IPC fixture)', async ({
  page,
  request,
}) => {
  await page.exposeFunction(
    'mobileNativeInvoke',
    async (name: string, args: { request: unknown }) =>
      (await request.post(`/api/${name.replace('mobile_', '')}`, { data: args.request })).json(),
  );
  await page.addInitScript(() =>
    Object.assign(window, {
      isTauri: true,
      __TAURI_INTERNALS__: {
        invoke: (name: string, args: unknown) =>
          (
            window as unknown as { mobileNativeInvoke: (n: string, a: unknown) => Promise<unknown> }
          ).mobileNativeInvoke(name, args),
      },
    }),
  );
  await page.goto('/');
  await expect(page.getByText('On this phone', { exact: true })).toBeVisible();
  await page.context().setOffline(true);
  await page.getByRole('navigation').getByRole('button', { name: 'Clients', exact: true }).click();
  await page.getByRole('button', { name: 'New client', exact: true }).click();
  await page.getByLabel('Client name', { exact: true }).fill('Native offline client');
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Native offline client', exact: true }),
  ).toBeVisible();
});
test('mobile backup restores records, provides safety undo and rejects damaged files', async ({
  page,
}) => {
  await page.goto('/');
  const settings = () =>
    page.getByRole('navigation').getByRole('button', { name: 'Settings', exact: true }).click();
  await settings();
  const downloadPromise = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Save mobile backup', exact: true }).click();
  const download = await downloadPromise;
  await mkdir('test-results', { recursive: true });
  const path = 'test-results/workspace.pfmobile';
  await download.saveAs(path);
  expect((await readFile(path)).subarray(0, 18).toString()).toBe('PACKFLOW-MOBILE-1\n');
  await page.getByRole('navigation').getByRole('button', { name: 'Clients', exact: true }).click();
  await page.getByRole('button', { name: 'New client', exact: true }).click();
  await page.getByLabel('Client name', { exact: true }).fill('Added after backup');
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  await settings();
  await page.getByText('Restore a mobile backup', { exact: true }).click();
  await page.getByLabel('Mobile backup file', { exact: true }).setInputFiles(path);
  await page.getByLabel('Type RESTORE', { exact: true }).fill('RESTORE');
  await page.getByRole('button', { name: 'Restore records', exact: true }).click();
  await expect(page.getByRole('status')).toContainText('Restored');
  await page.getByText(/^Saved backups/).click();
  const safety = page.locator('.backup-row').filter({ hasText: 'before-restore-' }).last();
  const undoPromise = page.waitForEvent('download');
  await safety.getByRole('button', { name: 'Download', exact: true }).click();
  await (await undoPromise).saveAs('test-results/safety.pfmobile');
  await page.getByRole('navigation').getByRole('button', { name: 'Clients', exact: true }).click();
  await page.getByLabel('Search clients').fill('Added after backup');
  await expect(page.getByRole('heading', { name: 'No matching clients' })).toBeVisible();
  await settings();
  await page.getByText('Restore a mobile backup', { exact: true }).click();
  await page
    .getByLabel('Mobile backup file', { exact: true })
    .setInputFiles('test-results/safety.pfmobile');
  await page.getByLabel('Type RESTORE', { exact: true }).fill('RESTORE');
  await page.getByRole('button', { name: 'Restore records', exact: true }).click();
  await expect(page.getByRole('status')).toContainText('Restored');
  const corrupt = await readFile(path);
  corrupt[corrupt.length - 1] ^= 1;
  await writeFile('test-results/damaged.pfmobile', corrupt);
  await page
    .getByLabel('Mobile backup file', { exact: true })
    .setInputFiles('test-results/damaged.pfmobile');
  await page.getByLabel('Type RESTORE', { exact: true }).fill('RESTORE');
  await page.getByRole('button', { name: 'Restore records', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('checksum');
  await page.getByRole('navigation').getByRole('button', { name: 'Clients', exact: true }).click();
  await page.getByLabel('Search clients').fill('Added after backup');
  await expect(page.locator('.client-card')).toContainText('Added after backup');
});
test('export date-range transfer workbook from phone settings', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('navigation').getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByLabel('Changed from', { exact: true }).fill('2026-01-01');
  await page.getByLabel('Changed through', { exact: true }).fill('2027-01-01');
  const [download, response] = await Promise.all([
    page.waitForEvent('download'),
    page.waitForResponse((r) => r.url().endsWith('/api/transfer')),
    page.getByRole('button', { name: 'Export transfer Excel', exact: true }).click(),
  ]);
  expect(response.ok()).toBeTruthy();
  expect(response.request().postDataJSON()).toEqual({
    from_date: '2026-01-01',
    to_date: '2027-01-01',
  });
  expect(download.suggestedFilename()).toMatch(/PackFlow-transfer.*\.xlsx$/);
  const bytes = await readFile((await download.path())!);
  expect(bytes.subarray(0, 2).toString()).toBe('PK');
  expect(bytes.length).toBeGreaterThan(1000);
  await expect(page.getByRole('status')).toContainText('Transfer Excel downloaded');
});

test('cancel offers discard and stage template CRUD/reordering does not rename saved orders', async ({
  page,
}) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'New order', exact: true }).click();
  await page.getByLabel('Order name', { exact: true }).fill('Unsaved');
  await page.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Discard unsaved changes?' })).toBeVisible();
  await page.getByRole('button', { name: 'Discard changes', exact: true }).click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await page.getByRole('navigation').getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByRole('button', { name: 'Lamination', exact: true }).click();
  await page.getByLabel('Stage name', { exact: true }).fill('Lamination renamed');
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Lamination renamed', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Move Lamination renamed down' }).click();
  await expect(page.locator('.template-row').nth(1)).toContainText('Lamination renamed');
  await page.getByRole('navigation').getByRole('button', { name: 'Orders', exact: true }).click();
  await page.getByRole('button', { name: /MOB-.*Mobile gift boxes/ }).click();
  await expect(page.locator('.stage-detail').first()).toContainText('1. Lamination');
});
