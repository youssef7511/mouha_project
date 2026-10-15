import { test, expect } from '@playwright/test';

test('native startup displays text IPC errors so a failed installer can be diagnosed', async ({
  page,
}) => {
  await page.addInitScript(() => {
    Object.assign(window, {
      isTauri: true,
      __TAURI_INTERNALS__: {
        invoke: () => Promise.reject('auth not allowed by the main window capability'),
      },
    });
  });
  await page.goto('/');
  await expect(page.getByRole('alert')).toContainText(
    'auth not allowed by the main window capability',
  );
  await expect(page.getByRole('button', { name: 'Retry', exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Orders', exact: true })).toHaveCount(0);
});
