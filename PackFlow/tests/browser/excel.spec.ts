import { test, expect } from '@playwright/test';
import { readFile } from 'node:fs/promises';

test('every page downloads a real Excel workbook, including the selected report dates', async ({
  page,
}) => {
  await page.goto('/');
  for (const name of [
    'Dashboard',
    'Orders',
    'Clients',
    'Workers',
    'Costs',
    'Payments & balances',
    'Reports',
    'Settings',
    'Stock',
  ]) {
    await page.getByRole('navigation').getByRole('button', { name, exact: true }).click();
    if (name === 'Reports') {
      await page.getByLabel('From', { exact: true }).fill('2026-10-12');
      await page.getByLabel('To', { exact: true }).fill('2026-10-13');
    }
    const [download, response] = await Promise.all([
      page.waitForEvent('download'),
      page.waitForResponse((r) => r.url().endsWith('/api/export')),
      page.getByRole('button', { name: 'Export Excel', exact: true }).click(),
    ]);
    expect(response.ok()).toBeTruthy();
    expect(download.suggestedFilename()).toMatch(/^PackFlow-.+\.xlsx$/);
    const buffer = await readFile((await download.path())!);
    expect(buffer.subarray(0, 2).toString()).toBe('PK');
    expect(buffer.length).toBeGreaterThan(1000);
    if (name === 'Reports')
      expect(response.request().postDataJSON()).toMatchObject({
        from_date: '2026-10-12',
        to_date: '2026-10-13',
      });
  }
});

test('desktop export passes current filters and handles a cancelled save or write failure', async ({
  page,
  request,
}) => {
  const exports: Record<string, unknown>[] = [];
  await page.exposeFunction(
    'nativeExportTest',
    async (name: string, args: { request: Record<string, unknown> }) => {
      if (name === 'auth')
        return { configured: true, unlocked: true, display_name: 'Export Owner' };
      if (name === 'workspace_export') {
        exports.push(args.request);
        if (exports.length === 1) return { cancelled: true };
        throw new Error('Export folder is not writable.');
      }
      return (await request.post(`/api/${name}`, { data: args.request })).json();
    },
  );
  await page.addInitScript(() => {
    Object.assign(window, {
      isTauri: true,
      __TAURI_INTERNALS__: {
        invoke: (name: string, args: unknown) =>
          (
            window as unknown as {
              nativeExportTest: (name: string, args: unknown) => Promise<unknown>;
            }
          ).nativeExportTest(name, args),
      },
    });
  });
  await page.goto('/');
  await page.getByRole('navigation').getByRole('button', { name: 'Reports', exact: true }).click();
  await page.getByLabel('From', { exact: true }).fill('2026-10-12');
  await page.getByLabel('To', { exact: true }).fill('2026-10-13');
  const button = page.getByRole('button', { name: 'Export Excel', exact: true });
  await button.click();
  await expect(button).toBeEnabled();
  expect(exports[0]).toMatchObject({
    page: 'reports',
    from_date: '2026-10-12',
    to_date: '2026-10-13',
  });
  await expect(page.getByText('Excel saved:', { exact: false })).toHaveCount(0);
  await button.click();
  await expect(page.getByRole('alert')).toContainText('Export folder is not writable.');
  await expect(button).toBeEnabled();
});
