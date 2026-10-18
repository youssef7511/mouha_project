import { test, expect } from '@playwright/test';
import process from 'node:process';

test('real preview account: setup, protected API, remember, logout and recovery', async ({
  page,
}) => {
  test.skip(
    process.env.PACKFLOW_AUTH_E2E !== '1',
    'Requires a fresh isolated preview with authentication enabled.',
  );
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Create your account' })).toBeVisible();
  expect((await page.request.post('/api/export', { data: { page: 'orders' } })).status()).toBe(401);
  expect((await page.request.post('/api/query', { data: { kind: 'bootstrap' } })).status()).toBe(
    401,
  );
  await page.getByLabel('Your name').fill('Preview Owner');
  await page.getByLabel('Username', { exact: true }).fill('preview-owner');
  await page
    .getByLabel('New password (at least 10 characters)', { exact: true })
    .fill('Preview fixture password');
  await page.getByLabel('Confirm password', { exact: true }).fill('Preview fixture password');
  await page.getByRole('button', { name: 'Create account', exact: true }).click();
  const recovery = await page.getByLabel('Recovery code').inputValue();
  expect(recovery).toHaveLength(64);
  await page.getByRole('button', { name: 'I saved it — continue to sign in' }).click();
  await page.getByLabel('Password', { exact: true }).fill('Wrong password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('Credentials were not accepted');
  await page.getByLabel('Password', { exact: true }).fill('Preview fixture password');
  await page.getByLabel('Remember me on this browser').check();
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await expect(page.getByText('Signed in as')).toContainText('Preview Owner');
  expect((await page.request.post('/api/query', { data: { kind: 'bootstrap' } })).status()).toBe(
    200,
  );
  const cookie = (await page.context().cookies()).find((c) => c.name === 'packflow_preview')!;
  expect(cookie.httpOnly).toBeTruthy();
  expect(cookie.sameSite).toBe('Strict');
  expect(cookie.expires).toBeGreaterThan(Date.now() / 1000);
  await page.reload();
  await page.getByRole('button', { name: 'Log out', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Welcome back' })).toBeVisible();
  expect((await page.request.post('/api/query', { data: { kind: 'bootstrap' } })).status()).toBe(
    401,
  );
  await page.getByRole('button', { name: 'Forgot password?' }).click();
  await page.getByLabel('Recovery code').fill(recovery);
  await page
    .getByLabel('New password (at least 10 characters)', { exact: true })
    .fill('Recovered fixture password');
  await page.getByLabel('Confirm password', { exact: true }).fill('Recovered fixture password');
  await page.getByRole('button', { name: 'Reset password', exact: true }).click();
  await expect(page.getByLabel('Recovery code')).not.toHaveValue(recovery);
  await page.getByRole('button', { name: 'I saved it — continue to sign in' }).click();
  await page.getByLabel('Username', { exact: true }).fill('preview-owner');
  await page.getByLabel('Password', { exact: true }).fill('Recovered fixture password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByLabel('Current password', { exact: true }).fill('Recovered fixture password');
  await page.getByLabel('New password', { exact: true }).fill('Changed fixture password');
  await page.getByLabel('Confirm new password', { exact: true }).fill('Changed fixture password');
  await page.getByRole('button', { name: 'Change password and sign out' }).click();
  await expect(page.getByRole('heading', { name: 'Welcome back' })).toBeVisible();
  await page.screenshot({ path: 'test-results/preview-login.png' });
});
