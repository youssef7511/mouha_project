import { test, expect } from '@playwright/test';

test('real core: client → mixed order → stage work and cost → reload', async ({ page }) => {
  const name = `Browser verification ${Date.now()}`;
  const pageErrors: string[] = [];
  page.on('pageerror', (e) => pageErrors.push(e.message));
  await page.goto('/');
  await expect(page.getByText('Local database connected')).toBeVisible();
  await page.getByRole('button', { name: 'Clients', exact: true }).click();
  await page.getByRole('button', { name: 'Add client', exact: true }).click();
  let dialog = page.getByRole('dialog');
  await dialog.getByLabel('Company name').fill(name);
  await dialog.getByLabel('Contact name').fill('Amal');
  await dialog.getByRole('button', { name: 'Save client' }).click();
  await expect(dialog).not.toBeVisible();
  await page.getByRole('button', { name: 'New order', exact: true }).click();
  dialog = page.getByRole('dialog');
  await dialog.getByLabel('Order name').fill(name);
  await dialog.getByRole('combobox', { name: 'Client', exact: true }).selectOption({ label: name });
  await dialog.getByLabel('Due date').fill('2026-11-20');
  await dialog.getByRole('button', { name: 'Add box type' }).click();
  await dialog.getByLabel('Description', { exact: true }).nth(0).fill('Small box');
  await dialog.getByLabel('Ordered boxes').nth(0).fill('100');
  await dialog.getByLabel('Description', { exact: true }).nth(1).fill('Large box');
  await dialog.getByLabel('Ordered boxes').nth(1).fill('50');
  await dialog
    .getByRole('combobox', { name: /Customer pricing/ })
    .nth(0)
    .selectOption('per_box');
  await dialog.getByLabel('Selling price per box').nth(0).fill('0.250');
  await dialog.getByLabel('Selling price per box').nth(1).fill('0.400');
  for (let index = 1; index < 4; index++) {
    const card = dialog.locator('.stage-price-card').nth(index);
    await card.getByRole('combobox').selectOption('fixed');
    await card.getByLabel('Stage fixed total').fill('0.000');
  }
  await expect(
    dialog.locator('.quote-summary').getByText('KWD 45.000', { exact: true }),
  ).toBeVisible();
  await dialog.getByRole('button', { name: 'Create order', exact: true }).click();
  await expect(dialog).not.toBeVisible();
  const detail = page.locator('.detail');
  await expect(detail.getByRole('heading', { name })).toBeVisible();
  await expect(detail.getByText('KWD 45.000', { exact: true })).toBeVisible();
  await expect(detail.getByText('100 boxes')).toBeVisible();
  await detail.getByRole('button', { name: /Lamination/ }).click();
  dialog = page.getByRole('dialog');
  await dialog.getByRole('combobox', { name: 'Status', exact: true }).selectOption('completed');
  await dialog.getByRole('button', { name: 'Save stage status' }).click();
  await expect(dialog).not.toBeVisible();
  await detail.getByRole('button', { name: /Lamination/ }).click();
  await page.getByRole('tab', { name: 'Costs', exact: true }).click();
  dialog = page.getByRole('dialog');
  await dialog.getByLabel('Expense description').fill('Direct purchase batch');
  await dialog.getByRole('combobox', { name: 'Cost entry' }).selectOption('unit');
  await dialog.getByLabel('Quantity · Unit').fill('1.125');
  await dialog.getByLabel('Purchase cost per Unit').fill('0.333');
  await expect(dialog.getByText('KWD 0.375')).toBeVisible();
  await dialog.getByRole('button', { name: 'Save & add another', exact: true }).click();
  await expect(dialog).toBeVisible();
  await expect(dialog.getByRole('status')).toContainText('1 cost saved');
  await page.getByRole('tab', { name: 'Work', exact: true }).click();
  dialog = page.getByRole('dialog');
  await dialog
    .getByLabel('Work / sheets description')
    .fill('120 sheets laminated; sheet count is descriptive');
  await dialog.getByLabel('Input · Unit').fill('120');
  await dialog.getByLabel('Worker time').fill('40');
  await dialog.getByLabel('Machine work', { exact: true }).fill('Press');
  await dialog.getByLabel('Machine time').fill('20');
  await dialog.getByRole('button', { name: 'Record work', exact: true }).click();
  await expect(dialog).not.toBeVisible();
  await page.reload();
  await page.getByRole('button', { name: 'Orders', exact: true }).click();
  await page.getByRole('button', { name: new RegExp(name) }).click();
  await expect(page.locator('.detail').getByText('KWD 45.000', { exact: true })).toBeVisible();
  await expect(page.locator('.detail').getByText('KWD 0.375', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Costs', exact: true }).click();
  await expect(page.getByText('KWD 44.625')).toBeVisible();
  await expect(page.getByText('Direct purchase batch')).toBeVisible();
  await page.screenshot({ path: 'test-results/costs.png', fullPage: true });
  await page.getByRole('button', { name: 'Stock', exact: true }).click();
  await expect(page.getByText('Stock management is deferred')).toBeVisible();
  expect(pageErrors).toEqual([]);
});

test('template rename preserves existing order snapshots and local-only fonts', async ({
  page,
  request,
}) => {
  const boot = await (await request.post('/api/query', { data: { kind: 'bootstrap' } })).json();
  const original = boot.templates.find((t: { position: number }) => t.position === 1);
  const stamp = Date.now();
  const orderName = `Snapshot verification ${stamp}`;
  const create = async (action: unknown) => {
    const response = await request.post('/api/mutate', {
      data: { request_id: crypto.randomUUID(), actor_id: boot.actors[0].id, action },
    });
    expect(response.ok()).toBeTruthy();
    return response.json();
  };
  const customer = await create({
    kind: 'save_client',
    id: null,
    expected_version: null,
    name: orderName,
    contact_name: '',
    email: '',
    phone: '',
    address: '',
    archived: false,
  });
  await create({
    kind: 'create_order',
    client_id: customer.data.id,
    name: orderName,
    due_date: '2026-11-20',
    notes: '',
    pricing_mode: 'unpriced',
    fixed_total_fils: null,
    items: [
      { description: 'Test box', dimensions_text: '', ordered_boxes: 5, unit_price_fils: null },
    ],
    template_ids: boot.templates
      .filter((t: { archived: number }) => !t.archived)
      .map((t: { id: string }) => t.id),
  });
  const updatedName = `Printing verification ${stamp}`;
  const external: string[] = [];
  page.on('request', (r) => {
    if (/^https?:/.test(r.url()) && !r.url().startsWith('http://127.0.0.1')) external.push(r.url());
  });
  await page.goto('/');
  await expect(page.getByText('Local database connected')).toBeVisible();
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page
    .locator('.template-row')
    .filter({ hasText: original.name })
    .getByRole('button', { name: 'Edit', exact: true })
    .click();
  const dialog = page.getByRole('dialog');
  await dialog.getByLabel('Stage name').fill(updatedName);
  await dialog.getByRole('button', { name: 'Save stage', exact: true }).click();
  await expect(dialog).not.toBeVisible();
  await expect(page.locator('.template-row').filter({ hasText: updatedName })).toBeVisible();
  await page.getByRole('button', { name: 'Orders', exact: true }).click();
  await page.locator('.order-card').filter({ hasText: orderName }).click();
  await expect(page.locator('.detail .stage-row').filter({ hasText: original.name })).toBeVisible();
  await page.setViewportSize({ width: 1024, height: 700 });
  await page.screenshot({ path: 'test-results/orders-1024.png', fullPage: true });
  expect(external).toEqual([]);
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page
    .locator('.template-row')
    .filter({ hasText: updatedName })
    .getByRole('button', { name: 'Edit', exact: true })
    .click();
  await page.getByRole('dialog').getByLabel('Stage name').fill(original.name);
  await page.getByRole('dialog').getByRole('button', { name: 'Save stage', exact: true }).click();
  await expect(page.getByRole('dialog')).not.toBeVisible();
});
