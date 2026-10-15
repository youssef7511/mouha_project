import { test, expect } from '@playwright/test';

test('client rankings, separate payment deadlines and partial-payment follow-up stay connected', async ({
  page,
  request,
}) => {
  const boot = await (await request.post('/api/query', { data: { kind: 'bootstrap' } })).json();
  const stamp = Date.now();
  const prefix = `Client insights ${stamp}`;
  const yesterday = new Date(`${boot.business_today}T12:00:00Z`);
  yesterday.setUTCDate(yesterday.getUTCDate() - 1);
  const due = yesterday.toISOString().slice(0, 10);
  const create = async (action: unknown) => {
    const response = await request.post('/api/mutate', {
      data: { request_id: crypto.randomUUID(), actor_id: boot.actors[0].id, action },
    });
    expect(response.ok()).toBeTruthy();
    return (await response.json()).data;
  };
  const client = async (name: string) =>
    create({
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
  const a = await client(`${prefix} A`),
    b = await client(`${prefix} B`);
  const order = async (cid: string, name: string) =>
    create({
      kind: 'create_order',
      client_id: cid,
      name,
      due_date: '2026-12-20',
      notes: '',
      pricing_mode: 'fixed',
      fixed_total_fils: '45000',
      items: [
        { description: 'Test box', dimensions_text: '', ordered_boxes: 100, unit_price_fils: null },
      ],
      template_ids: boot.templates
        .filter((t: { archived: number }) => !t.archived)
        .map((t: { id: string }) => t.id),
    });
  const first = await order(a.id, `${prefix} first`),
    second = await order(a.id, `${prefix} second`),
    third = await order(b.id, `${prefix} third`);
  const receipt = async (oid: string, amount: string) =>
    create({
      kind: 'receive_payment',
      order_id: oid,
      amount_fils: amount,
      business_date: boot.business_today,
      method: 'cash',
      reference: '',
      notes: '',
    });
  await receipt(first.id, '10000');
  await receipt(second.id, '5000');
  await receipt(third.id, '50000');
  await page.goto('/');
  await expect(page.getByText('Local database connected')).toBeVisible();
  await page.getByRole('button', { name: 'Payments & balances', exact: true }).click();
  await page.getByRole('combobox', { name: 'Order', exact: true }).selectOption(first.id);
  await page.getByRole('button', { name: 'Set payment due date', exact: true }).click();
  await page.getByRole('dialog').getByLabel('Agreed payment due date').fill(due);
  await page
    .getByRole('dialog')
    .getByRole('button', { name: 'Save payment due date', exact: true })
    .click();
  await expect(page.getByRole('dialog')).not.toBeVisible();
  await expect(page.getByText('Payment overdue', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Clients', exact: true }).click();
  await page.getByLabel('Search clients', { exact: true }).fill(prefix);
  await expect(page.locator('.client-leader').first().getByRole('heading')).toHaveText(
    `${prefix} A`,
  );
  await page.getByRole('combobox', { name: 'Rank clients by' }).selectOption('received');
  await expect(page.locator('.client-leader').first().getByRole('heading')).toHaveText(
    `${prefix} B`,
  );
  const row = page.locator('.client-review tbody tr').filter({ hasText: `${prefix} A` });
  await expect(row.getByText('2 partially paid', { exact: true })).toBeVisible();
  await expect(row.getByText('KWD 35.000', { exact: true })).toBeVisible();
  await expect(row.getByRole('cell').nth(4)).toContainText('1 order');
  await page.getByRole('button', { name: `Review ${prefix} A`, exact: true }).click();
  await expect(
    page
      .locator('.client-history')
      .getByRole('heading', { name: `${prefix} A · order & payment history` }),
  ).toBeVisible();
  await page
    .locator('.client-history')
    .getByRole('button', { name: new RegExp(`${prefix} first`) })
    .first()
    .click();
  await expect(page.getByRole('combobox', { name: 'Order', exact: true })).toHaveValue(first.id);
  await page.getByRole('button', { name: 'Record payment', exact: true }).click();
  await page.getByRole('dialog').getByLabel('Payment amount').fill('35.000');
  await page.getByRole('dialog').getByRole('button', { name: 'Save payment', exact: true }).click();
  await expect(page.getByRole('dialog')).not.toBeVisible();
  await page.getByRole('button', { name: 'Clients', exact: true }).click();
  await page.getByLabel('Search clients', { exact: true }).fill(prefix);
  await expect(
    page
      .locator('.client-review tbody tr')
      .filter({ hasText: `${prefix} A` })
      .getByText('1 partially paid', { exact: true }),
  ).toBeVisible();
  await expect(
    page
      .locator('.client-review tbody tr')
      .filter({ hasText: `${prefix} A` })
      .getByText('KWD 0.000', { exact: true }),
  ).toBeVisible();
});
