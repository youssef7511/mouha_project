import { test, expect } from '@playwright/test';
import { existsSync } from 'node:fs';
const fixture = process.env.PACKFLOW_TRANSFER_TEST_OUTPUT;
test('phone workbook: match client, preview, import, restart-safe duplicate detection', async ({
  page,
}) => {
  test.skip(
    !fixture || !existsSync(fixture),
    'Set PACKFLOW_TRANSFER_TEST_OUTPUT to the real-core generated test workbook.',
  );
  await page.goto('/');
  await page.getByRole('navigation').getByRole('button', { name: 'Orders', exact: true }).click();
  await page.getByRole('button', { name: 'Import from phone', exact: true }).click();
  let dialog = page.getByRole('dialog');
  await dialog.getByLabel('Phone transfer workbook').setInputFiles(fixture!);
  await expect(dialog.getByRole('heading', { name: 'Match phone clients' })).toBeVisible();
  await expect(
    dialog.getByRole('button', { name: 'Import ready records', exact: true }),
  ).toBeDisabled();
  await dialog.getByLabel('Match عميل الكويت', { exact: true }).selectOption('new');
  await dialog.getByRole('button', { name: 'Preview import', exact: true }).click();
  await expect(dialog.getByText(/Create PF-.*phone MOB-/)).toBeVisible();
  await dialog.getByRole('checkbox').check();
  await dialog.getByRole('button', { name: 'Import ready records', exact: true }).click();
  await expect(dialog.getByRole('status')).toContainText('Import completed.');
  await expect(dialog.getByRole('status')).toContainText('before-phone-import-');
  await dialog.getByRole('button', { name: 'Done', exact: true }).click();
  await page.getByLabel('Search orders', { exact: true }).fill('Phone two box types');
  await expect(page.getByText('Phone two box types', { exact: true })).toBeVisible();
  await page.reload();
  await page
    .getByRole('navigation')
    .getByRole('button', { name: 'Payments & balances', exact: true })
    .click();
  await page.getByRole('button', { name: 'Import from phone', exact: true }).click();
  dialog = page.getByRole('dialog');
  await dialog.getByLabel('Phone transfer workbook').setInputFiles(fixture!);
  await expect(dialog.getByText('0 ready records.', { exact: false })).toBeVisible();
  await expect(
    dialog.getByRole('button', { name: 'Import ready records', exact: true }),
  ).toBeDisabled();
  await expect(dialog.getByRole('cell', { name: 'duplicate', exact: true })).toHaveCount(3);
  await page.screenshot({ path: 'test-results/phone-import-preview.png', fullPage: true });
});
