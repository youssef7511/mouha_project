export type Mode = 'unpriced' | 'fixed' | 'per_box' | 'per_unit';
export interface Client {
  id: string;
  name: string;
  contact_name: string;
  phone: string;
  email: string;
  address: string;
  notes: string;
  archived: number;
  version: number;
}
export interface Template {
  id: string;
  name: string;
  notes: string;
  position: number;
  archived: number;
  version: number;
}
export interface Box {
  id: string;
  description: string;
  dimensions: string;
  quantity: number;
}
export interface StagePrice {
  template_id: string;
  pricing_mode: Mode;
  fixed_total_fils: string | null;
  quantity_scaled: string | null;
  unit_price_fils: string | null;
  item_prices: { item_id: string; unit_price_fils: string }[];
}
export interface OrderInput {
  client_id: string;
  name: string;
  due_date: string | null;
  payment_due_date: string | null;
  notes: string;
  status: 'draft' | 'confirmed';
  items: Box[];
  stages: StagePrice[];
}
export interface OrderDocument {
  input: OrderInput;
  stage_names: string[];
  stage_totals: (string | null)[];
}
export interface Order {
  id: string;
  number: string;
  client_id: string;
  client_name: string;
  name: string;
  status: 'draft' | 'confirmed';
  document: OrderDocument;
  total_fils: string | null;
  known_total_fils: string;
  received_fils: string;
  balance_fils: string | null;
  archived: number;
  version: number;
  created_at: string;
  updated_at: string;
}
export interface Payment {
  id: string;
  order_id: string;
  number: string;
  order_name: string;
  client_name: string;
  kind: 'receipt' | 'correction';
  corrected: number;
  original_id: string | null;
  amount_fils: string;
  signed_fils: string;
  business_date: string;
  method: string;
  reference: string;
  notes: string;
  reason: string;
  created_at: string;
}
export interface Snapshot {
  clients: Client[];
  orders: Order[];
  payments: Payment[];
  templates: Template[];
  profile: { name: string; device_id: string; workspace_id: string };
  today: string;
  currency: 'KWD';
  schema: number;
}
export interface ReceiptInput {
  amount_fils: string;
  business_date: string;
  method: 'cash' | 'bank' | 'card' | 'other';
  reference: string;
  notes: string;
}
export interface Revision {
  version: number;
  document: string;
  total_fils: string | null;
  known_total_fils: string;
  reason: string;
  created_at: string;
}
export type Action =
  | {
      kind: 'save_client';
      id: string | null;
      expected_version: number | null;
      name: string;
      contact_name: string;
      phone: string;
      email: string;
      address: string;
      notes: string;
      archived: boolean;
    }
  | {
      kind: 'save_template';
      id: string | null;
      expected_version: number | null;
      name: string;
      notes: string;
      archived: boolean;
    }
  | { kind: 'reorder_templates'; ids: string[]; versions: number[] }
  | {
      kind: 'save_order';
      id: string | null;
      expected_version: number | null;
      input: OrderInput;
      reason: string;
      deposit: ReceiptInput | null;
    }
  | { kind: 'archive_order'; id: string; expected_version: number; archived: boolean }
  | { kind: 'receive'; order_id: string; receipt: ReceiptInput }
  | { kind: 'correct_receipt'; id: string; reason: string; business_date: string }
  | { kind: 'profile'; name: string };
export interface Operation {
  request_id: string;
  action: Action;
}
export interface SaveResult {
  id?: string;
  version?: number;
  saved?: boolean;
}
