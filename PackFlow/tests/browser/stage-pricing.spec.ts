import { test, expect } from '@playwright/test';

test('mixed stage prices sum, deselection and box removal stay aligned, and copied prices remain connected', async ({
  page,
  request,
}) => {
  const boot = await (await request.post('/api/query', { data: { kind: 'bootstrap' } })).json();
  const clientResponse = await request.post('/api/mutate', {
    data: {
      request_id: crypto.randomUUID(),
      actor_id: boot.actors[0].id,
      action: {
        kind: 'save_client',
        id: null,
        expected_version: null,
        name: `Stage quote client ${Date.now()}`,
        contact_name: '',
        email: '',
        phone: '',
        address: '',
        archived: false,
      },
    },
  });
  expect(clientResponse.ok()).toBeTruthy();
  const client = (await clientResponse.json()).data;
  const name = `Mixed stage quote ${Date.now()}`;
  await page.goto('/');
  await expect(page.getByText('Local database connected')).toBeVisible();
  await page.getByRole('button', { name: 'New order', exact: true }).click();
  const dialog = page.getByRole('dialog');
  await dialog.getByLabel('Order name').fill(name);
  await dialog.getByRole('combobox', { name: 'Client', exact: true }).selectOption(client.id);
  await dialog.getByLabel('Due date').fill('2026-12-01');
  await dialog.getByLabel('Description', { exact: true }).fill('Small');
  await dialog.getByLabel('Ordered boxes').fill('100');
  await dialog.getByRole('button', { name: 'Add box type' }).click();
  await dialog.getByLabel('Description', { exact: true }).nth(1).fill('Large');
  await dialog.getByLabel('Ordered boxes').nth(1).fill('50');
  const cards = dialog.locator('.stage-price-card');
  await cards.nth(0).getByRole('combobox').selectOption('fixed');
  await cards.nth(0).getByLabel('Stage fixed total').fill('10.000');
  await cards.nth(1).getByRole('combobox').selectOption('per_box');
  await cards.nth(1).getByLabel('Selling price per box 1').fill('0.250');
  await cards.nth(1).getByLabel('Selling price per box 2').fill('0.400');
  await cards.nth(2).getByRole('combobox').selectOption('per_unit');
  await cards.nth(2).getByLabel('Quantity').fill('1.125');
  await cards.nth(2).getByLabel('Selling price per Unit').fill('0.333');
  await expect(dialog.locator('.quote-summary')).toContainText('Not priced');
  await expect(dialog.getByRole('status')).toContainText('KWD 55.375');
  await cards.nth(3).getByRole('combobox').selectOption('fixed');
  await cards.nth(3).getByLabel('Stage fixed total').fill('5.000');
  await expect(dialog.locator('.quote-summary')).toContainText('KWD 60.375');
  await cards.nth(3).getByRole('checkbox').uncheck();
  await expect(cards.nth(3).getByRole('combobox')).toHaveCount(0);
  await expect(dialog.locator('.quote-summary')).toContainText('KWD 55.375');
  await cards.nth(3).getByRole('checkbox').check();
  await expect(dialog.locator('.quote-summary')).toContainText('KWD 60.375');
  // Removing a middle item must keep the surviving box type's stage rate.
  await dialog.getByRole('button', { name: 'Add box type' }).click();
  await dialog.getByLabel('Description', { exact: true }).nth(2).fill('Extra');
  await dialog.getByLabel('Ordered boxes').nth(2).fill('10');
  await cards.nth(1).getByLabel('Selling price per box 3').fill('0.100');
  await dialog.getByRole('button', { name: 'Remove box type 2', exact: true }).click();
  await expect(cards.nth(1).getByLabel('Selling price per box 2')).toHaveValue('0.100');
  await expect(dialog.locator('.quote-summary')).toContainText('KWD 41.375');
  await dialog.getByRole('button', { name: 'Create order', exact: true }).click();
  await expect(dialog).not.toBeVisible();
  const orders = await (await request.post('/api/query', { data: { kind: 'list_orders' } })).json();
  const order = orders.find((o: { name: string }) => o.name === name);
  expect(order.quote_fils).toBe('41375');
  const detail = await (
    await request.post('/api/query', { data: { kind: 'order', id: order.id } })
  ).json();
  expect(detail.stage_prices.map((p: { total_fils: string }) => p.total_fils)).toEqual([
    '10000',
    '26000',
    '375',
    '5000',
  ]);
  expect(detail.cost_fils).toBe('0');
  await page.getByRole('button', { name: 'Payments & balances', exact: true }).click();
  await page.getByRole('combobox', { name: 'Order', exact: true }).selectOption(order.id);
  await expect(page.locator('.stage-quote-breakdown')).toContainText('1.125 Unit × KWD 0.333');
  await page.reload();
  await page.getByRole('button', { name: 'New order', exact: true }).click();
  const copy = page.getByRole('dialog');
  await copy.getByLabel('Order name').fill(`${order.number} · ${name}`);
  await expect(copy.getByLabel('Order name')).toHaveValue(name);
  await expect(copy.locator('.quote-summary')).toContainText('KWD 41.375');
  await expect(
    copy.locator('.stage-price-card').nth(1).getByLabel('Selling price per box 2'),
  ).toHaveValue('0.100');
  const clonedName = `Copied ${name}`;
  await copy.getByLabel('Order name').fill(clonedName);
  await copy.getByRole('button', { name: 'Create order', exact: true }).click();
  await expect(copy).not.toBeVisible();
  const clonedOrders = await (
    await request.post('/api/query', { data: { kind: 'list_orders' } })
  ).json();
  const clone = clonedOrders.find((o: { name: string }) => o.name === clonedName);
  const clonedDetail = await (
    await request.post('/api/query', { data: { kind: 'order', id: clone.id } })
  ).json();
  expect(clonedDetail.order.quote_fils).toBe('41375');
  expect(clonedDetail.stage_prices.map((p: { total_fils: string }) => p.total_fils)).toEqual([
    '10000',
    '26000',
    '375',
    '5000',
  ]);
  expect(clonedDetail.payments).toEqual([]);
  expect(clonedDetail.services).toEqual([]);
  expect(clonedDetail.cost_fils).toBe('0');
});
