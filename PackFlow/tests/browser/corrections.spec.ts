import { test, expect } from '@playwright/test';

test('separate cost correction preserves the row and updates reports', async ({
  page,
  request,
}) => {
  const boot = await (await request.post('/api/query', { data: { kind: 'bootstrap' } })).json();
  const description = `Correct this cost ${Date.now()}`;
  const response = await request.post('/api/mutate', {
    data: {
      request_id: crypto.randomUUID(),
      actor_id: boot.actors[0].id,
      action: {
        kind: 'add_separate_cost',
        category: 'machine',
        amount_fils: '2345',
        description,
        business_date: boot.business_today,
      },
    },
  });
  expect(response.ok()).toBeTruthy();
  const total = async () =>
    (await (await request.post('/api/query', { data: { kind: 'separate_costs' } })).json())
      .total_fils;
  const before = BigInt(await total());
  await page.goto('/');
  await page.getByRole('button', { name: 'Costs', exact: true }).click();
  const row = page.getByRole('row').filter({ hasText: description });
  await row.getByRole('button', { name: 'Correct entry' }).click();
  await expect(row.getByRole('button', { name: 'Confirm reversal' })).toBeDisabled();
  await row.getByLabel('Reason').fill('Entered twice by mistake');
  await row.getByRole('button', { name: 'Confirm reversal' }).click();
  await expect(row).toContainText('Reversed · original retained');
  expect(BigInt(await total())).toBe(before - 2345n);
  await page.reload();
  await page.getByRole('button', { name: 'Costs', exact: true }).click();
  await expect(page.getByRole('row').filter({ hasText: description })).toContainText(
    'Entered twice by mistake',
  );
});

test('native account screens gate workspace access and clear it on logout (IPC UI fixture)', async ({
  page,
  request,
}) => {
  let configured = false;
  let unlocked = false;
  let businessReads = 0;
  const recovery = 'a'.repeat(64);
  await page.exposeFunction(
    'nativeTestInvoke',
    async (name: string, args: { request?: Record<string, unknown> }) => {
      const r = args.request ?? {};
      if (name === 'auth') {
        if (r.kind === 'setup') {
          configured = true;
          return { configured, unlocked, recovery_code: recovery };
        }
        if (r.kind === 'login') {
          if (r.password !== 'Fixture password 123')
            throw new Error('Credentials were not accepted.');
          unlocked = true;
        }
        if (r.kind === 'logout') unlocked = false;
        return { configured, unlocked, display_name: 'Fixture Owner' };
      }
      if (!unlocked) throw new Error('Sign in required');
      businessReads++;
      return (await request.post(`/api/${name}`, { data: r })).json();
    },
  );
  await page.addInitScript(() => {
    Object.assign(window, {
      isTauri: true,
      __TAURI_INTERNALS__: {
        invoke: (name: string, args: unknown) =>
          (
            window as unknown as { nativeTestInvoke: (n: string, a: unknown) => Promise<unknown> }
          ).nativeTestInvoke(name, args),
      },
    });
  });
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Create your account' })).toBeVisible();
  expect(businessReads).toBe(0);
  await page.getByLabel('Your name').fill('Fixture Owner');
  await page.getByLabel('Username', { exact: true }).fill('owner');
  await page
    .getByLabel('New password (at least 10 characters)', { exact: true })
    .fill('Fixture password 123');
  await page.getByLabel('Confirm password', { exact: true }).fill('Different password 123');
  await page.getByRole('button', { name: 'Create account', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('Passwords do not match');
  await page.getByLabel('Confirm password', { exact: true }).fill('Fixture password 123');
  await page.getByRole('button', { name: 'Create account', exact: true }).click();
  await expect(page.getByLabel('Recovery code')).toHaveValue(recovery);
  expect(businessReads).toBe(0);
  await page.getByRole('button', { name: 'I saved it — continue to sign in' }).click();
  await page.getByLabel('Password', { exact: true }).fill('Fixture password 123');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Save backup…' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Choose backup and restore…' })).toBeDisabled();
  await page.getByRole('button', { name: 'Log out', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Welcome back' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Orders', exact: true })).toHaveCount(0);
  await page.screenshot({ path: 'test-results/local-login.png' });
});
