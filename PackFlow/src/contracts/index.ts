export type StageState = 'not_started' | 'in_progress' | 'completed';
export type PricingMode = 'unpriced' | 'fixed' | 'per_box';
export type StagePricingMode = PricingMode | 'per_unit';
export interface StagePriceInput {
  template_id: string;
  pricing_mode: StagePricingMode;
  fixed_total_fils: string | null;
  quantity_scaled: string | null;
  unit_price_fils: string | null;
  item_prices_fils: string[];
}
export interface StagePrice {
  stage_id: string;
  template_id: string | null;
  name_snapshot: string;
  pricing_mode: StagePricingMode;
  fixed_total_fils: string | null;
  quantity_scaled: string | null;
  unit_price_fils: string | null;
  total_fils: string | null;
}
export interface Actor {
  id: string;
  display_name: string;
  active: number;
}
export interface Client {
  id: string;
  name: string;
  contact_name: string;
  email: string;
  phone: string;
  address: string;
  archived: number;
  version: number;
  order_count: number;
}
export interface Worker {
  id: string;
  name: string;
  role: string;
  active: number;
  archived: number;
  version: number;
}
export interface SalaryRecord {
  id: string;
  month: string;
  worker_id: string;
  worker_name_snapshot: string;
  basic_fils: string;
  normal_days_scaled: string;
  hours_per_day_scaled: string;
  fridays: number;
  overtime_hours_scaled: string;
  normal_fils: string;
  friday_fils: string;
  overtime_fils: string;
  total_fils: string;
  notes: string;
  archived: number;
  version: number;
}
export interface SalariesData {
  month: string;
  divisor: number;
  version: number;
  workers: Pick<Worker, 'id' | 'name' | 'active' | 'archived'>[];
  records: SalaryRecord[];
  months: { month: string; divisor: number; version: number }[];
  totals: {
    workers: number;
    normal_fils: string;
    friday_fils: string;
    overtime_fils: string;
    total_fils: string;
  };
  history: {
    id: string;
    action: string;
    created_at: string;
    after_json: string;
    actor_name: string;
  }[];
}
export interface StageTemplate {
  id: string;
  name: string;
  default_notes: string;
  position: number;
  archived: number;
  version: number;
}
export interface Bootstrap {
  actors: Actor[];
  clients: Client[];
  workers: Worker[];
  templates: StageTemplate[];
  business_today: string;
  schema_version: number;
  settings: { key: string; value_json: string }[];
  stats: { active_orders: number; ordered_boxes: number; completed_stages: number };
}
export interface OrderSummary {
  id: string;
  number: string;
  name: string;
  client_id: string;
  client_name: string;
  due_date: string;
  payment_due_date: string | null;
  notes: string;
  lifecycle: 'open' | 'closed' | 'cancelled';
  archived: number;
  version: number;
  quote_fils: string | null;
  service_charge_fils: string;
  total_due_fils: string | null;
  paid_fils: string;
  balance_fils: string | null;
  pricing_mode: PricingMode;
  ordered_boxes: number;
  stage_count: number;
  completed_stages: number;
}
export interface OrderItem {
  id: string;
  order_id: string;
  description: string;
  dimensions_text: string;
  ordered_boxes: number;
  unit_price_fils: string | null;
}
export interface OrderPage {
  records: (OrderSummary & { cost_fils: string })[];
  total: number;
  page: number;
  page_size: number;
}
export interface OrderStage {
  id: string;
  order_id: string;
  template_id: string | null;
  name_snapshot: string;
  state: StageState;
  position: number;
  notes: string;
  retired: number;
  version: number;
  cost_fils: string;
}
export type ExpenseCategory =
  'material' | 'outsourced' | 'machine' | 'labor' | 'transport' | 'other';
export interface Cost {
  service_linked: number;
  reversed: number;
  reason?: string | null;
  id: string;
  stage_id: string | null;
  kind: 'expense' | 'reversal';
  category: ExpenseCategory;
  description: string;
  supplier_text: string;
  reference: string;
  quantity_scaled: string | null;
  unit_cost_fils: string | null;
  amount_fils: string;
  business_date: string;
}
export type SeparateCostCategory = 'machine' | 'finish' | 'other';
export interface SeparateCost {
  reversed: number;
  reason?: string | null;
  id: string;
  category: SeparateCostCategory;
  amount_fils: string;
  description: string;
  business_date: string;
  created_at: string;
  actor_name: string;
}
export interface SeparateCostsData {
  records: SeparateCost[];
  categories: { category: SeparateCostCategory; amount_fils: string; count: number }[];
  total_fils: string;
  count: number;
}
export interface Work {
  kind: 'work' | 'reversal';
  reversed: number;
  reason?: string | null;
  id: string;
  stage_id: string;
  description: string;
  dimensions_text: string;
  input_scaled: string | null;
  output_scaled: string | null;
  waste_scaled: string | null;
  worker_id: string | null;
  worker_minutes: number | null;
  machine_description: string;
  machine_minutes: number | null;
  business_date: string;
}
export interface History {
  id: string;
  action: string;
  entity_type: string;
  entity_id: string;
  business_date: string;
  created_at: string;
  reason: string | null;
  actor_name: string;
}
export interface OrderDetail {
  mobile_source?: {
    number: string;
    source_version: number;
    source_id: string;
    workspace_id: string;
  } | null;
  stage_prices: StagePrice[];
  stage_item_prices: { stage_id: string; item_id: string; unit_price_fils: string }[];
  order: Omit<OrderSummary, 'ordered_boxes' | 'stage_count' | 'completed_stages'>;
  items: OrderItem[];
  stages: OrderStage[];
  costs: Cost[];
  work: Work[];
  cost_fils: string;
  history: History[];
  services: OutsideService[];
  payments: Payment[];
}
export interface OutsideService {
  reversed: number;
  reason?: string | null;
  id: string;
  description: string;
  charge_fils: string;
  supplier_cost_fils: string;
  supplier_text: string;
  reference: string;
  business_date: string;
}
export interface Payment {
  reversed: number;
  reason?: string | null;
  id: string;
  kind: 'receipt' | 'refund';
  amount_fils: string;
  business_date: string;
  method: string;
  reference: string;
  notes: string;
}
export interface Reports {
  portfolio_summary: {
    order_count: number;
    ordered_boxes: number;
    total_due_fils: string;
    unpriced_orders: number;
  };
  from_date: string;
  to_date: string;
  portfolio: {
    id: string;
    number: string;
    name: string;
    client_name: string;
    lifecycle: string;
    total_due_fils: string | null;
    cost_fils: string;
    paid_fils: string;
    balance_fils: string | null;
    ordered_boxes: number;
  }[];
  activity: { business_date: string; cost_fils: string; received_fils: string }[];
  categories: { category: ExpenseCategory; amount_fils: string }[];
  stages: { name: string; not_started: number; in_progress: number; completed: number }[];
}
export interface ClientInsight {
  id: string;
  name: string;
  contact_name: string;
  archived: number;
  order_count: number;
  quoted_fils: string;
  net_received_fils: string;
  outstanding_fils: string;
  credit_fils: string;
  overdue_fils: string;
  priced_orders: number;
  unpriced_orders: number;
  partial_orders: number;
  settled_orders: number;
  overdue_orders: number;
  late_receipt_orders: number;
  instalment_orders: number;
  missing_deadline_orders: number;
  dated_orders: number;
  oldest_overdue_days: number;
}
export interface ClientInsightsData {
  business_today: string;
  clients: ClientInsight[];
  orders: (Pick<
    OrderSummary,
    | 'id'
    | 'client_id'
    | 'number'
    | 'name'
    | 'archived'
    | 'due_date'
    | 'payment_due_date'
    | 'total_due_fils'
    | 'paid_fils'
    | 'balance_fils'
  > & { receipt_count: number; has_receipt_after_due: number })[];
  payments: (Pick<
    Payment,
    'id' | 'kind' | 'amount_fils' | 'business_date' | 'method' | 'reference'
  > & { order_id: string; client_id: string; number: string; name: string })[];
}
export interface NewItem {
  description: string;
  dimensions_text: string;
  ordered_boxes: number;
  unit_price_fils: string | null;
}
export interface WorkInput {
  item_id: string | null;
  description: string;
  dimensions_text: string;
  input_scaled: string | null;
  output_scaled: string | null;
  waste_scaled: string | null;
  worker_id: string | null;
  worker_minutes: number | null;
  machine_description: string;
  machine_minutes: number | null;
}
export interface ExpenseInput {
  item_id: string | null;
  category: ExpenseCategory;
  description: string;
  supplier_text: string;
  reference: string;
  pricing_basis: 'unit' | 'total';
  quantity_scaled: string | null;
  unit_cost_fils: string | null;
  amount_fils: string | null;
}
export type Action =
  | {
      kind: 'reverse_entry';
      entry_type: 'expense' | 'work' | 'payment' | 'service' | 'separate_cost';
      id: string;
      reason: string;
      business_date: string;
    }
  | {
      kind: 'add_separate_cost';
      category: SeparateCostCategory;
      amount_fils: string;
      description: string;
      business_date: string;
    }
  | { kind: 'set_salary_divisor'; month: string; expected_version: number; divisor: number }
  | {
      kind: 'save_salary';
      id: string | null;
      expected_version: number | null;
      month: string;
      expected_month_version: number;
      worker_id: string;
      basic_fils: string;
      normal_days_scaled: string;
      hours_per_day_scaled: string;
      fridays: number;
      overtime_hours_scaled: string;
      notes: string;
    }
  | {
      kind: 'archive_salary';
      id: string;
      expected_version: number;
      expected_month_version: number;
      archived: boolean;
    }
  | {
      kind: 'set_payment_due_date';
      order_id: string;
      expected_version: number;
      payment_due_date: string | null;
    }
  | {
      kind: 'add_outside_service';
      order_id: string;
      expected_version: number;
      description: string;
      supplier_text: string;
      charge_fils: string;
      supplier_cost_fils: string;
      business_date: string;
      reference: string;
    }
  | {
      kind: 'receive_payment';
      order_id: string;
      amount_fils: string;
      business_date: string;
      method: string;
      reference: string;
      notes: string;
    }
  | {
      kind: 'save_client';
      id: string | null;
      expected_version: number | null;
      name: string;
      contact_name: string;
      email: string;
      phone: string;
      address: string;
      archived: boolean;
    }
  | {
      kind: 'save_worker';
      id: string | null;
      expected_version: number | null;
      name: string;
      role: string;
      active: boolean;
    }
  | {
      kind: 'save_template';
      id: string | null;
      expected_version: number | null;
      name: string;
      default_notes: string;
      archived: boolean;
    }
  | { kind: 'reorder_templates'; ids: string[]; versions: number[] }
  | { kind: 'delete_template'; id: string; expected_version: number }
  | {
      kind: 'create_order';
      client_id: string;
      name: string;
      due_date: string;
      notes: string;
      pricing_mode: PricingMode;
      fixed_total_fils: string | null;
      items: NewItem[];
      template_ids: string[];
      stage_prices?: StagePriceInput[];
    }
  | {
      kind: 'update_stage';
      id: string;
      expected_version: number;
      state: StageState;
      notes: string;
      business_date: string;
      reason: string | null;
    }
  | {
      kind: 'post_stage_entry';
      order_id: string;
      stage_id: string;
      business_date: string;
      work: WorkInput | null;
      expenses: ExpenseInput[];
    };
export interface Mutation {
  request_id: string;
  actor_id: string;
  action: Action;
}
export interface MutationResult {
  data: { id?: string; order_id?: string; number?: string };
  changed_entities: string[];
  committed_at: string;
}
export type Query =
  | { kind: 'order_page'; search: string; page: number; page_size: number; scope: 'all' | 'active' }
  | { kind: 'separate_costs' }
  | { kind: 'salaries'; month: string }
  | { kind: 'client_insights' }
  | { kind: 'reports'; from_date: string; to_date: string; include_portfolio?: boolean }
  | { kind: 'bootstrap' }
  | { kind: 'list_orders' }
  | { kind: 'order'; id: string }
  | { kind: 'diagnostics' };
