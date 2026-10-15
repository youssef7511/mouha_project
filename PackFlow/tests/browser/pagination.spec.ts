import { test, expect } from '@playwright/test';
test('search and pagination reach older orders in Orders, Payments and Reports without changing report totals', async ({
  page,
  request,
}) => {
  test.setTimeout(120000);
  const boot = await (await request.post('/api/query', { data: { kind: 'bootstrap' } })).json();
  const stamp = Date.now();
  const prefix = `Paged fixture ${stamp}`;
  const create = async (action: unknown) => {
    const response = await request.post('/api/mutate', {
      data: { request_id: crypto.randomUUID(), actor_id: boot.actors[0].id, action },
    });
    expect(response.ok()).toBeTruthy();
    return (await response.json()).data;
  };
  const clientName = `Paged client ${stamp}`;
  const client = await create({
    kind: 'save_client',
    id: null,
    expected_version: null,
    name: clientName,
    contact_name: '',
    email: '',
    phone: '',
    address: '',
    archived: false,
  });
  let oldest: { id: string; number: string } | undefined;
  for (let index = 0; index < 203; index++) {
    const order = await create({
      kind: 'create_order',
      client_id: client.id,
      name: `${prefix} ${String(index).padStart(3, '0')}`,
      due_date: '2026-12-01',
      notes: '',
      pricing_mode: 'fixed',
      fixed_total_fils: '1000',
      items: [
        { description: 'Test box', dimensions_text: '', ordered_boxes: 1, unit_price_fils: null },
      ],
      template_ids: boot.templates
        .filter((t: { archived: number }) => !t.archived)
        .map((t: { id: string }) => t.id),
    });
    if (!index) oldest = order;
  }
  expect(oldest).toBeTruthy();
  const newest = await (await request.post('/api/query', { data: { kind: 'list_orders' } })).json();
  expect(newest.some((o: { id: string }) => o.id === oldest!.id)).toBeFalsy();
  const errors: string[] = [];
  page.on('pageerror', (e) => errors.push(e.message));
  await page.goto('/');
  await expect(page.getByText('Local database connected')).toBeVisible();
  await page.getByRole('button', { name: 'Orders', exact: true }).click();
  await page.getByLabel('Search orders', { exact: true }).fill(prefix);
  const cards = page.locator('.order-card');
  const orderNav = page.getByRole('navigation', { name: 'Orders pagination', exact: true });
  await expect(orderNav).toContainText('of 203 orders');
  await expect(cards).toHaveCount(25);
  await page.getByLabel('Orders rows per page', { exact: true }).selectOption('10');
  await expect(cards).toHaveCount(10);
  await expect(orderNav).toContainText('Page 1 of 21');
  await page.getByRole('button', { name: 'Orders next page', exact: true }).click();
  await expect(orderNav).toContainText('Page 2 of 21');
  await expect(cards.first()).toContainText(`${prefix} 192`);
  await page.getByLabel('Search orders', { exact: true }).fill(`${prefix} 000`);
  await expect(cards).toHaveCount(1);
  await expect(orderNav).toContainText('Page 1 of 1');
  await cards.first().click();
  await expect(
    page.locator('.detail').getByRole('heading', { name: `${prefix} 000`, exact: true }),
  ).toBeVisible();
  await page.getByRole('button', { name: 'Payments & balances', exact: true }).click();
  const balanceSearch = page.getByLabel('Search order balances', { exact: true });
  await balanceSearch.fill(clientName);
  const balanceNav = page.getByRole('navigation', {
    name: 'Order balances pagination',
    exact: true,
  });
  await expect(balanceNav).toContainText('of 203 orders');
  await page.getByLabel('Order balances rows per page', { exact: true }).selectOption('10');
  await page.getByRole('button', { name: 'Order balances next page', exact: true }).click();
  await expect(balanceNav).toContainText('Page 2 of 21');
  await balanceSearch.fill(oldest!.number);
  await expect(balanceNav).toContainText('Showing 1–1 of 1 orders');
  await page.getByRole('button', { name: new RegExp(`${prefix} 000`) }).click();
  await expect(page.getByRole('combobox', { name: 'Order', exact: true })).toHaveValue(oldest!.id);
  await expect(page.locator('.payment-summary')).toContainText(`${prefix} 000`);
  await balanceSearch.fill('no matching order at all');
  await expect(page.getByText('No matching order balances', { exact: true })).toBeVisible();
  await expect(balanceNav).toContainText('Showing 0–0 of 0 orders');
  await page.getByRole('button', { name: 'Reports', exact: true }).click();
  const metric = page
    .locator('.report-metric')
    .filter({ has: page.getByText('Workspace order value', { exact: true }) })
    .locator('strong');
  await expect(metric).toBeVisible();
  const fullValue = await metric.innerText();
  const performanceSearch = page.getByLabel('Search order performance', { exact: true });
  await performanceSearch.fill(prefix);
  const performanceNav = page.getByRole('navigation', {
    name: 'Order performance pagination',
    exact: true,
  });
  await expect(performanceNav).toContainText('of 203 orders');
  await page.getByRole('button', { name: 'Order performance next page', exact: true }).click();
  await expect(performanceNav).toContainText('Page 2 of 9');
  await expect(metric).toHaveText(fullValue);
  await performanceSearch.fill(`${prefix} 000`);
  await expect(performanceNav).toContainText('Showing 1–1 of 1 orders');
  await expect(metric).toHaveText(fullValue);
  await page.screenshot({ path: 'test-results/report-search-pagination.png', fullPage: true });
  await page.getByRole('button', { name: new RegExp(`${prefix} 000`) }).click();
  await expect(
    page.locator('.detail').getByRole('heading', { name: `${prefix} 000`, exact: true }),
  ).toBeVisible();
  expect(errors).toEqual([]);
});
