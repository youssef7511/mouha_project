import { test, expect } from '@playwright/test';
test('stage costs stay open for repeated unit and total entries across every stage', async ({
  page,
  request,
}) => {
  const boot = await (await request.post('/api/query', { data: { kind: 'bootstrap' } })).json();
  const name = `Repeated stage costs ${Date.now()}`;
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
    name,
    contact_name: '',
    email: '',
    phone: '',
    address: '',
    archived: false,
  });
  const order = await create({
    kind: 'create_order',
    client_id: client.id,
    name,
    due_date: '2026-12-01',
    notes: '',
    pricing_mode: 'fixed',
    fixed_total_fils: '45000',
    items: [
      { description: 'Small', dimensions_text: '', ordered_boxes: 100, unit_price_fils: null },
      { description: 'Large', dimensions_text: '', ordered_boxes: 50, unit_price_fils: null },
    ],
    template_ids: boot.templates
      .filter((t: { archived: number }) => !t.archived)
      .map((t: { id: string }) => t.id),
  });
  const detail = await (
    await request.post('/api/query', { data: { kind: 'order', id: order.id } })
  ).json();
  const errors: string[] = [];
  page.on('pageerror', (e) => errors.push(e.message));
  await page.goto('/');
  await expect(page.getByText('Local database connected')).toBeVisible();
  await page.getByRole('button', { name: 'Orders', exact: true }).click();
  await page.getByRole('button', { name: new RegExp(name) }).click();
  const stageButtons = page.locator('.detail .stage-row');
  await stageButtons.first().click();
  await page.getByRole('tab', { name: 'Costs', exact: true }).click();
  const dialog = page.getByRole('dialog');
  await dialog.getByLabel('Business date').fill('2026-10-01');
  await dialog.getByLabel('Expense description').fill('First direct purchase');
  await dialog.getByLabel('Supplier / provider').fill('Repeat supplier');
  await dialog.getByLabel('Reference', { exact: true }).fill('FIRST');
  await dialog
    .getByRole('combobox', { name: 'Box type', exact: true })
    .selectOption(detail.items[1].id);
  await dialog.getByRole('combobox', { name: 'Cost entry', exact: true }).selectOption('unit');
  await dialog.getByLabel('Quantity · Unit').fill('1.125');
  await dialog.getByLabel('Purchase cost per Unit').fill('0.333');
  await dialog.getByRole('button', { name: 'Save & add another', exact: true }).click();
  await expect(dialog).toBeVisible();
  await expect(dialog.getByRole('status')).toContainText('1 cost saved');
  await expect(dialog.getByLabel('Business date')).toHaveValue('2026-10-01');
  await expect(dialog.getByRole('combobox', { name: 'Cost entry', exact: true })).toHaveValue(
    'unit',
  );
  await expect(dialog.getByRole('combobox', { name: 'Box type', exact: true })).toHaveValue(
    detail.items[1].id,
  );
  await expect(dialog.getByLabel('Supplier / provider')).toHaveValue('Repeat supplier');
  for (const label of [
    'Expense description',
    'Reference',
    'Quantity · Unit',
    'Purchase cost per Unit',
  ])
    await expect(dialog.getByLabel(label, { exact: label === 'Reference' })).toHaveValue('');
  await dialog.getByRole('combobox', { name: 'Category', exact: true }).selectOption('machine');
  await dialog.getByRole('combobox', { name: 'Cost entry', exact: true }).selectOption('total');
  await dialog.getByLabel('Expense description').fill('Second machine expense');
  await dialog.getByLabel('Total cost').fill('5.125');
  await dialog.getByRole('button', { name: 'Save & add another', exact: true }).click();
  await expect(dialog.getByRole('status')).toContainText('2 costs saved');
  await expect(dialog.locator('.quote-summary').first()).toContainText('KWD 5.500');
  await expect(dialog.locator('.ledger-row')).toHaveCount(2);
  await expect(dialog.getByRole('combobox', { name: 'Category', exact: true })).toHaveValue(
    'machine',
  );
  await expect(dialog.getByLabel('Total cost')).toHaveValue('');
  await page.screenshot({ path: 'test-results/stage-repeat-costs.png' });
  // Failed validation does not close the form or add a row.
  await dialog.getByRole('button', { name: 'Save & add another', exact: true }).click();
  await expect(dialog.getByText('Describe this expense.', { exact: true })).toBeVisible();
  await expect(dialog.getByRole('status')).toContainText('2 costs saved');
  // Discarding only the next unsaved entry must preserve both committed expenses.
  await dialog.getByLabel('Total cost').fill('0.100');
  page.once('dialog', async (prompt) => {
    await prompt.dismiss();
  });
  await dialog.getByRole('button', { name: 'Done', exact: true }).click();
  await expect(dialog).toBeVisible();
  page.once('dialog', async (prompt) => {
    await prompt.accept();
  });
  await dialog.getByRole('button', { name: 'Done', exact: true }).click();
  await expect(dialog).not.toBeVisible();
  for (let index = 1; index < detail.stages.length; index++) {
    await stageButtons.nth(index).click();
    await page.getByRole('tab', { name: 'Costs', exact: true }).click();
    await dialog.getByLabel('Expense description').fill(`Cost for stage ${index + 1}`);
    await dialog.getByLabel('Total cost').fill(String(index));
    await dialog.getByRole('button', { name: 'Save & add another', exact: true }).click();
    await expect(dialog.getByRole('status')).toContainText('1 cost saved');
    await expect(dialog).toBeVisible();
    await dialog.getByRole('button', { name: 'Done', exact: true }).click();
    await expect(dialog).not.toBeVisible();
  }
  await page.reload();
  await page.getByRole('button', { name: 'Orders', exact: true }).click();
  await page.getByRole('button', { name: new RegExp(name) }).click();
  await stageButtons.first().click();
  await page.getByRole('tab', { name: 'Costs', exact: true }).click();
  await expect(dialog.locator('.ledger-row')).toHaveCount(2);
  await expect(dialog.locator('.quote-summary').first()).toContainText('KWD 5.500');
  await dialog.getByRole('button', { name: 'Done', exact: true }).click();
  const saved = await (
    await request.post('/api/query', { data: { kind: 'order', id: order.id } })
  ).json();
  expect(saved.cost_fils).toBe('11500');
  expect(saved.costs).toHaveLength(5);
  expect(saved.order.quote_fils).toBe('45000');
  expect(saved.order.balance_fils).toBe('45000');
  expect(saved.payments).toEqual([]);
  expect(saved.stages.every((s: { state: string }) => s.state === 'not_started')).toBeTruthy();
  expect(errors).toEqual([]);
});
