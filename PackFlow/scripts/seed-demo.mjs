import { randomUUID } from 'node:crypto';
import { mkdir, readFile, rename, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';

// Explicitly opt-in: this only writes to the local development core.
const base = 'http://127.0.0.1:4318';
const journalPath = fileURLToPath(new URL('../.dev-data/demo-seed-v1.json', import.meta.url));
await mkdir(fileURLToPath(new URL('../.dev-data/', import.meta.url)), { recursive: true });
let previewCookie = "";
async function api(path, data) {
  const response = await fetch(`${base}/api/${path}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...(previewCookie ? { Cookie: previewCookie } : {}) },
    body: JSON.stringify(data),
    signal: AbortSignal.timeout(15_000),
  });
  const result = await response.json();
  const cookie = response.headers.get("set-cookie");
  if (cookie) previewCookie = cookie.split(";")[0];
  if (!response.ok) throw new Error(`${path}: ${JSON.stringify(result)}`);
  return result;
}
if (process.env.PACKFLOW_PREVIEW_USERNAME && process.env.PACKFLOW_PREVIEW_PASSWORD) {
  await api('auth', {kind:'login', username:process.env.PACKFLOW_PREVIEW_USERNAME, password:process.env.PACKFLOW_PREVIEW_PASSWORD, remember:false});
}
const bootstrap = await api('query', { kind: 'bootstrap' });
let journal;
try {
  journal = JSON.parse(await readFile(journalPath, 'utf8'));
} catch (error) {
  if (error.code !== 'ENOENT') throw error;
  journal = {
    business_date: bootstrap.business_today,
    actor_id: bootstrap.actors.find((actor) => actor.active)?.id,
    template_ids: bootstrap.templates
      .filter((stage) => !stage.archived)
      .sort((a, b) => a.position - b.position)
      .map((stage) => stage.id),
    steps: {},
  };
}
if (!journal.actor_id || journal.template_ids.length < 4)
  throw new Error('Demo scenarios need an active actor and at least four active stage templates.');
async function saveJournal() {
  await writeFile(`${journalPath}.tmp`, JSON.stringify(journal, null, 2));
  await rename(`${journalPath}.tmp`, journalPath);
}
async function seed(key, action) {
  let step = journal.steps[key];
  if (!step) {
    step = journal.steps[key] = {
      request: { request_id: randomUUID(), actor_id: journal.actor_id, action },
    };
    // Keep the exact request before sending: interrupted writes retry idempotently.
    await saveJournal();
  }
  if (!step.result) {
    step.result = await api('mutate', step.request);
    await saveJournal();
  }
  return step.result.data;
}
function dateOffset(days) {
  const date = new Date(`${journal.business_date}T12:00:00Z`);
  date.setUTCDate(date.getUTCDate() + days);
  return date.toISOString().slice(0, 10);
}

const clientNames = ['Cedar Gifts', 'Sunrise Bakery', 'Amber Perfumes', 'Harbor Coffee'];
const clientIds = [];
for (const [index, company] of clientNames.entries()) {
  const result = await seed(`client-${index}`, {
    kind: 'save_client',
    id: null,
    expected_version: null,
    name: `[Demo] ${company}`,
    contact_name: ['Amal', 'Omar', 'Mariam', 'Sami'][index],
    email: `demo${index + 1}@example.com`,
    phone: '',
    address: 'Fictional customer for development testing.',
    archived: false,
  });
  clientIds.push(result.id);
}
const workerIds = [];
for (const [index, [name, role]] of [
  ['Amal', 'Lamination operator'],
  ['Omar', 'Printing and diecut operator'],
  ['Mariam', 'Finishing and quality checks'],
].entries()) {
  const result = await seed(`worker-${index}`, {
    kind: 'save_worker',
    id: null,
    expected_version: null,
    name: `[Demo] ${name}`,
    role,
    active: true,
  });
  workerIds.push(result.id);
}
const item = (description, dimensions, quantity, price = null) => ({
  description,
  dimensions_text: dimensions,
  ordered_boxes: quantity,
  unit_price_fils: price,
});
const orders = [
  {
    key: 'gift',
    name: 'Gift boxes - mixed sizes',
    client: 0,
    due: 7,
    mode: 'per_box',
    total: null,
    items: [
      item('Small gift box', '12 x 8 x 5 cm', 500, '280'),
      item('Large gift box', '22 x 16 x 8 cm', 300, '420'),
    ],
    completed: 2,
    running: true,
  },
  {
    key: 'bakery',
    name: 'Bakery takeaway boxes',
    client: 1,
    due: 3,
    mode: 'per_box',
    total: null,
    items: [item('Pastry box', '18 x 12 x 6 cm', 1500, '120')],
    completed: 1,
    running: true,
  },
  {
    key: 'perfume',
    name: 'Perfume launch packaging',
    client: 2,
    due: 1,
    mode: 'fixed',
    total: '480000',
    items: [
      item('Single bottle box', '8 x 8 x 15 cm', 400),
      item('Gift set box', '24 x 18 x 8 cm', 200),
    ],
    completed: 4,
    running: false,
  },
  {
    key: 'coffee',
    name: 'Coffee subscription sleeves',
    client: 3,
    due: 10,
    mode: 'per_box',
    total: null,
    items: [item('Coffee sleeve', '10 x 7 x 20 cm', 800, '300')],
    completed: 0,
    running: true,
  },
  {
    key: 'retail',
    name: 'Retail assortment - awaiting quote',
    client: 0,
    due: 14,
    mode: 'unpriced',
    total: null,
    items: [
      item('Counter display box', '30 x 20 x 12 cm', 1000),
      item('Sample box', '9 x 9 x 4 cm', 500),
    ],
    completed: 0,
    running: false,
  },
  {
    key: 'rush',
    name: 'Rush replacement batch - cost overrun',
    client: 1,
    due: -2,
    mode: 'fixed',
    total: '35000',
    items: [item('Replacement pastry box', '18 x 12 x 6 cm', 120)],
    completed: 3,
    running: true,
  },
  {
    key: 'macaron',
    name: 'Macaron boxes - three assortments',
    client: 1,
    due: 5,
    mode: 'per_box',
    total: null,
    items: [
      item('Six-piece macaron box', '18 x 8 x 4 cm', 600, '180'),
      item('Twelve-piece macaron box', '24 x 16 x 4 cm', 400, '260'),
      item('Party assortment box', '32 x 24 x 5 cm', 200, '450'),
    ],
    completed: 0,
    running: false,
  },
  {
    key: 'candle',
    name: 'Candle gift packaging',
    client: 0,
    due: 6,
    mode: 'fixed',
    total: '325000',
    items: [
      item('Single candle box', '10 x 10 x 12 cm', 450),
      item('Candle duo box', '22 x 12 x 12 cm', 150),
    ],
    completed: 1,
    running: true,
  },
  {
    key: 'discovery',
    name: 'Perfume discovery sets',
    client: 2,
    due: 2,
    mode: 'per_box',
    total: null,
    items: [item('Five-vial discovery box', '16 x 10 x 3 cm', 1000, '390')],
    completed: 3,
    running: true,
  },
  {
    key: 'coffee-bulk',
    name: 'Wholesale coffee cartons',
    client: 3,
    due: 12,
    mode: 'fixed',
    total: '1250000',
    items: [
      item('Six-bag carton', '30 x 20 x 24 cm', 2000),
      item('Twelve-bag carton', '40 x 30 x 24 cm', 1000),
    ],
    completed: 2,
    running: true,
  },
  {
    key: 'samples',
    name: 'Complimentary sample boxes - zero quote',
    client: 2,
    due: 0,
    mode: 'fixed',
    total: '0',
    items: [item('Promotional sample box', '7 x 7 x 3 cm', 80)],
    completed: 4,
    running: false,
  },
  {
    key: 'seasonal',
    name: 'Seasonal gift collection - awaiting quote',
    client: 0,
    due: 21,
    mode: 'unpriced',
    total: null,
    items: [
      item('Gift hamper box', '36 x 28 x 18 cm', 250),
      item('Mini gift box', '10 x 10 x 6 cm', 750),
    ],
    completed: 0,
    running: false,
  },
];
function expensesFor(index, reference) {
  const total = (category, description, amount, supplier) => ({
    item_id: null,
    category,
    description,
    supplier_text: `[Demo] ${supplier}`,
    reference,
    pricing_basis: 'total',
    quantity_scaled: null,
    unit_cost_fils: null,
    amount_fils: amount,
  });
  return [
    [
      {
        item_id: null,
        category: 'material',
        description: 'Demo direct purchase of lamination film',
        supplier_text: '[Demo] Film supplier',
        reference,
        pricing_basis: 'unit',
        quantity_scaled: '125500',
        unit_cost_fils: '125',
        amount_fils: null,
      },
      total('machine', 'Demo laminator setup and use', '12500', 'Machine service'),
    ],
    [total('outsourced', 'Demo outsourced colour impression', '45750', 'Printing partner')],
    [total('machine', 'Demo diecut machine setup and run', '22500', 'Machine service')],
    [
      total('material', 'Demo glue and finishing supplies', '18900', 'Packaging supplier'),
      total('transport', 'Demo supplier collection cost', '8250', 'Local transport'),
    ],
  ][index];
}
for (const order of orders) {
  const created = await seed(`order-${order.key}`, {
    kind: 'create_order',
    client_id: clientIds[order.client],
    name: `[Demo] ${order.name}`,
    due_date: dateOffset(order.due),
    notes:
      'Fictional test order. Sheet quantities are descriptive; Unit values are stage work records. No dispatch or payment has been recorded.',
    pricing_mode: order.mode,
    fixed_total_fils: order.total,
    items: order.items,
    template_ids: journal.template_ids,
  });
  const detail = await api('query', { kind: 'order', id: created.id });
  const stages = detail.stages
    .filter((stage) => !stage.retired)
    .sort((a, b) => a.position - b.position);
  const worked = order.completed + (order.running ? 1 : 0);
  for (let index = 0; index < worked; index++) {
    const stage = stages[index];
    const input = 600 - index * 10;
    await seed(`${order.key}-entry-${index}`, {
      kind: 'post_stage_entry',
      order_id: created.id,
      stage_id: stage.id,
      business_date: dateOffset(-3 + index),
      work: {
        item_id: detail.items[0].id,
        description: `Demo ${stage.name_snapshot} work; ${input} sheets noted by the operator.`,
        dimensions_text: order.items[0].dimensions_text,
        input_scaled: String(input * 1000),
        output_scaled: String((input - 10) * 1000),
        waste_scaled: '10000',
        worker_id: workerIds[index === 3 ? 2 : index % 2],
        worker_minutes: [65, 90, 50, 120][index],
        machine_description: [
          'Laminator',
          'Colour impression press',
          'Diecut press',
          'Folder and gluer',
        ][index],
        machine_minutes: [45, 75, 40, 60][index],
      },
      expenses: expensesFor(index, `DEMO-${order.key.toUpperCase()}-${index + 1}`),
    });
    await seed(`${order.key}-status-${index}`, {
      kind: 'update_stage',
      id: stage.id,
      expected_version: stage.version,
      state: index < order.completed ? 'completed' : 'in_progress',
      notes:
        index < order.completed
          ? 'Demo stage completed and checked.'
          : 'Demo stage currently in progress.',
      business_date: dateOffset(-3 + index),
      reason: null,
    });
  }
  console.log(`${created.number}: [Demo] ${order.name}`);
}

// Financial examples: partial payment, fully paid, credit, and unknown balance.
for (const [key, description, charge, cost] of [
  ['gift', 'Demo custom ribbon and gift-box finishing', '18000', '12000'],
  ['perfume', 'Demo premium embossing service', '60000', '42000'],
  ['retail', 'Demo outsourced artwork preparation', '15000', '9000'],
]) {
  const oid = journal.steps[`order-${key}`].result.data.id;
  const detail = await api('query', { kind: 'order', id: oid });
  await seed(`${key}-outside-service`, {
    kind: 'add_outside_service',
    order_id: oid,
    expected_version: detail.order.version,
    description,
    supplier_text: '[Demo] Specialist finishing partner',
    charge_fils: charge,
    supplier_cost_fils: cost,
    business_date: journal.business_date,
    reference: `DEMO-SERVICE-${key}`,
  });
}
for (const [key, amount, method] of [
  ['gift', '125000', 'bank'],
  ['perfume', '540000', 'bank'],
  ['coffee', '250000', 'cash'],
  ['retail', '10000', 'cash'],
]) {
  await seed(`${key}-receipt`, {
    kind: 'receive_payment',
    order_id: journal.steps[`order-${key}`].result.data.id,
    amount_fils: amount,
    business_date: journal.business_date,
    method,
    reference: `DEMO-RECEIPT-${key}`,
    notes: 'Fictional customer receipt for testing balances and reports.',
  });
}

// Agreed payment deadlines are separate from production due dates.
for (const [key, offset] of [
  ['gift', -1],
  ['macaron', -1],
  ['perfume', -2],
  ['rush', -2],
  ['candle', 7],
  ['coffee', 7],
]) {
  const oid = journal.steps[`order-${key}`].result.data.id;
  const detail = await api('query', { kind: 'order', id: oid });
  await seed(`${key}-payment-deadline`, {
    kind: 'set_payment_due_date',
    order_id: oid,
    expected_version: detail.order.version,
    payment_due_date: dateOffset(offset),
  });
}
for (const [key, amount] of [
  ['macaron', '50000'],
  ['candle', '100000'],
]) {
  await seed(`${key}-demo-deposit`, {
    kind: 'receive_payment',
    order_id: journal.steps[`order-${key}`].result.data.id,
    amount_fils: amount,
    business_date: journal.business_date,
    method: 'bank',
    reference: `DEMO-DEPOSIT-${key}`,
    notes: 'Demo agreed deposit; balance remains payable on its separate due date.',
  });
}

// Fictional monthly salaries remain separate from production costs and customer payments.
const salaryMonth = journal.business_date.slice(0, 7);
for (const [index, basic, hours, fridays, overtime] of [
  [0, '125000', '6000', 1, '10000'],
  [1, '185000', '9000', 2, '15000'],
  [2, '90000', '6000', 0, '8000'],
]) {
  const month = await api('query', { kind: 'salaries', month: salaryMonth });
  await seed(`monthly-salary-${index}`, {
    kind: 'save_salary',
    id: null,
    expected_version: null,
    month: salaryMonth,
    expected_month_version: month.version,
    worker_id: workerIds[index],
    basic_fils: basic,
    normal_days_scaled: String(month.divisor * 1000),
    hours_per_day_scaled: hours,
    fridays,
    overtime_hours_scaled: overtime,
    notes: 'Fictional monthly salary for testing; not imported from the workbook.',
  });
}

// Verify saved records through the same read API used by the UI.
const saved = await api('query', { kind: 'list_orders' });
for (const order of orders) {
  const id = journal.steps[`order-${order.key}`].result.data.id;
  if (!saved.some((entry) => entry.id === id)) throw new Error(`Missing demo order: ${order.key}`);
}
const final = await api('query', { kind: 'bootstrap' });
console.log(
  `Demo data ready: ${clientNames.length} clients, ${workerIds.length} workers, ${orders.length} orders.`,
);
console.log(
  `Database totals: ${final.stats.ordered_boxes} boxes; ${final.stats.completed_stages} completed stages.`,
);
console.log('Refresh PackFlow at http://127.0.0.1:1420/ to see the samples.');
