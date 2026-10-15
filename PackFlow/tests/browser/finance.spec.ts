import { test, expect } from '@playwright/test';

test('copy order details, then connect outside services, payment balances and reports', async ({
  page,
  request,
}) => {
  const boot = await (await request.post('/api/query', { data: { kind: 'bootstrap' } })).json();
  const stamp = Date.now();
  const create = async (action: unknown) => {
    const response = await request.post('/api/mutate', {
      data: { request_id: crypto.randomUUID(), actor_id: boot.actors[0].id, action },
    });
    expect(response.ok()).toBeTruthy();
    return (await response.json()).data;
  };
  const client = await create({
    kind: 'save_client',
    id: null,
    expected_version: null,
    name: `Finance client ${stamp}`,
    contact_name: '',
    email: '',
    phone: '',
    address: '',
    archived: false,
  });
  const source = await create({
    kind: 'create_order',
    client_id: client.id,
    name: `Original order ${stamp}`,
    due_date: '2026-11-20',
    notes: 'Repeat order instructions',
    pricing_mode: 'fixed',
    fixed_total_fils: '45000',
    items: [
      {
        description: 'Small box',
        dimensions_text: '10 x 8 cm',
        ordered_boxes: 100,
        unit_price_fils: null,
      },
      {
        description: 'Large box',
        dimensions_text: '20 x 16 cm',
        ordered_boxes: 50,
        unit_price_fils: null,
      },
    ],
    template_ids: boot.templates
      .filter((t: { archived: number }) => !t.archived)
      .map((t: { id: string }) => t.id),
  });
  await create({
    kind: 'add_outside_service',
    order_id: source.id,
    expected_version: 1,
    description: 'Original service',
    supplier_text: 'Original supplier',
    charge_fils: '18000',
    supplier_cost_fils: '12000',
    business_date: boot.business_today,
    reference: '',
  });
  await create({
    kind: 'receive_payment',
    order_id: source.id,
    amount_fils: '10000',
    business_date: boot.business_today,
    method: 'cash',
    reference: '',
    notes: '',
  });
  const sourceDetail = await (
    await request.post('/api/query', { data: { kind: 'order', id: source.id } })
  ).json();
  await create({
    kind: 'update_stage',
    id: sourceDetail.stages[0].id,
    expected_version: 1,
    state: 'completed',
    notes: '',
    business_date: boot.business_today,
    reason: null,
  });
  await page.goto('/');
  await expect(page.getByText('Local database connected')).toBeVisible();
  await page.getByRole('button', { name: 'New order', exact: true }).click();
  let dialog = page.getByRole('dialog');
  const name = dialog.getByRole('combobox', { name: 'Order name', exact: true });
  await name.fill(`${source.number} · Original order ${stamp}`);
  await expect(name).toHaveValue(`Original order ${stamp}`);
  await expect(dialog.getByRole('combobox', { name: 'Client', exact: true })).toHaveValue(
    client.id,
  );
  await expect(dialog.getByLabel('Description', { exact: true })).toHaveCount(2);
  await expect(dialog.getByLabel('Ordered boxes').nth(1)).toHaveValue('50');
  await expect(dialog.getByLabel('Stage fixed total').first()).toHaveValue('45.000');
  await expect(dialog.getByLabel('Order notes')).toHaveValue('Repeat order instructions');
  const cloneName = `Copied order ${stamp}`;
  await name.fill(cloneName);
  await dialog.getByRole('button', { name: 'Create order', exact: true }).click();
  await expect(dialog).not.toBeVisible();
  const orders = await (await request.post('/api/query', { data: { kind: 'list_orders' } })).json();
  const clone = orders.find((o: { name: string }) => o.name === cloneName);
  expect(clone.id).not.toBe(source.id);
  let detail = await (
    await request.post('/api/query', { data: { kind: 'order', id: clone.id } })
  ).json();
  expect(detail.order.total_due_fils).toBe('45000');
  expect(detail.cost_fils).toBe('0');
  expect(detail.payments).toEqual([]);
  expect(detail.services).toEqual([]);
  expect(detail.stages.every((s: { state: string }) => s.state === 'not_started')).toBeTruthy();
  await page.getByRole('button', { name: 'Payments & balances', exact: true }).click();
  await page.getByRole('combobox', { name: 'Order', exact: true }).selectOption(clone.id);
  await page.getByRole('button', { name: 'Add outside service', exact: true }).click();
  dialog = page.getByRole('dialog');
  await dialog.getByLabel('Outside service description').fill('Custom finishing');
  await dialog.getByLabel('Supplier', { exact: true }).fill('Finishing partner');
  await dialog.getByLabel('Charge to customer').fill('18.000');
  await dialog.getByLabel('Supplier cost').fill('12.000');
  await expect(dialog.getByText('KWD 63.000')).toBeVisible();
  await dialog.getByRole('button', { name: 'Save outside service' }).click();
  await expect(dialog).not.toBeVisible();
  await page.getByRole('button', { name: 'Record payment', exact: true }).click();
  await page.getByRole('dialog').getByLabel('Payment amount').fill('20.000');
  await page.getByRole('dialog').getByRole('button', { name: 'Save payment' }).click();
  await expect(page.getByRole('dialog')).not.toBeVisible();
  await expect(page.locator('.payment-balance').getByText('KWD 43.000')).toBeVisible();
  await page.reload();
  await page.getByRole('button', { name: 'Payments & balances', exact: true }).click();
  await page.getByRole('combobox', { name: 'Order', exact: true }).selectOption(clone.id);
  await expect(page.locator('.payment-balance').getByText('Partially paid')).toBeVisible();
  detail = await (
    await request.post('/api/query', { data: { kind: 'order', id: clone.id } })
  ).json();
  expect(detail.costs).toHaveLength(1);
  expect(detail.order.balance_fils).toBe('43000');
  await page.getByRole('button', { name: 'Costs', exact: true }).click();
  await expect(page.getByText('KWD 51.000', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Reports', exact: true }).click();
  await expect(page.getByRole('img', { name: /Daily costs and net receipts/ })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Cost breakdown' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Production stage progress' })).toBeVisible();
  await expect(page.getByRole('button', { name: new RegExp(cloneName) })).toBeVisible();
});
