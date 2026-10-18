import { useState, type ReactNode, type FormEvent } from 'react';
import { Plus, Trash2 } from 'lucide-react';
import type {
  Action,
  Client,
  Template,
  Snapshot,
  SaveResult,
  Order,
  OrderInput,
  StagePrice,
  Mode,
  Payment,
  ReceiptInput,
} from './types';
import { decimal, money, parseScaled } from './money';
import { quote } from './pricing';
import { Sheet, ErrorLine } from './Sheet';
export interface Store {
  data: Snapshot;
  busy: boolean;
  write: (action: Action) => Promise<SaveResult>;
  restore: (
    file: File,
    confirmation: string,
  ) => Promise<{ safety_backup: string; message: string }>;
}
function Form({
  title,
  children,
  busy,
  dirty,
  onClose,
  onSubmit,
  error,
  label = 'Save',
}: {
  title: string;
  children: ReactNode;
  busy: boolean;
  dirty: boolean;
  onClose: () => void;
  onSubmit: (e: FormEvent) => void;
  error: unknown;
  label?: string;
}) {
  return (
    <Sheet title={title} onClose={onClose} busy={busy} dirty={dirty}>
      <form onSubmit={onSubmit}>
        <fieldset disabled={busy}>
          <div className="sheet-body">
            <ErrorLine error={error} />
            {children}
          </div>
          <footer className="sheet-foot">
            <button
              type="button"
              className="button"
              onClick={(e) =>
                e.currentTarget
                  .closest('dialog')
                  ?.dispatchEvent(new Event('cancel', { cancelable: true }))
              }
              disabled={busy}
            >
              Cancel
            </button>
            <button className="button primary" type="submit">
              {busy ? 'Saving…' : label}
            </button>
          </footer>
        </fieldset>
      </form>
    </Sheet>
  );
}
export function ClientEditor({
  store,
  record,
  onClose,
  onSaved,
}: {
  store: Store;
  record?: Client;
  onClose: () => void;
  onSaved?: (id: string) => void;
}) {
  const [value, set] = useState({
    name: record?.name ?? '',
    contact_name: record?.contact_name ?? '',
    phone: record?.phone ?? '',
    email: record?.email ?? '',
    address: record?.address ?? '',
    notes: record?.notes ?? '',
    archived: !!record?.archived,
  });
  const [dirty, mark] = useState(false);
  const [error, setError] = useState<unknown>(null);
  const field = (key: keyof typeof value, v: string | boolean) => {
    mark(true);
    set((p) => ({ ...p, [key]: v }));
  };
  return (
    <Form
      title={record ? 'Edit client' : 'New client'}
      busy={store.busy}
      dirty={dirty}
      onClose={onClose}
      error={error}
      onSubmit={async (e) => {
        e.preventDefault();
        try {
          const r = await store.write({
            kind: 'save_client',
            id: record?.id ?? null,
            expected_version: record?.version ?? null,
            ...value,
          });
          if (r.id) onSaved?.(r.id);
          onClose();
        } catch (e) {
          setError(e);
        }
      }}
    >
      <label>
        Client name
        <input
          required
          maxLength={120}
          value={value.name}
          onChange={(e) => field('name', e.target.value)}
          autoComplete="organization"
        />
      </label>
      <label>
        Contact person
        <input
          maxLength={120}
          value={value.contact_name}
          onChange={(e) => field('contact_name', e.target.value)}
          autoComplete="name"
        />
      </label>
      <div className="field-grid">
        <label>
          Phone
          <input
            type="tel"
            maxLength={100}
            value={value.phone}
            onChange={(e) => field('phone', e.target.value)}
            autoComplete="tel"
          />
        </label>
        <label>
          Email
          <input
            type="email"
            maxLength={200}
            value={value.email}
            onChange={(e) => field('email', e.target.value)}
            autoComplete="email"
          />
        </label>
      </div>
      <label>
        Address
        <textarea
          maxLength={1000}
          value={value.address}
          onChange={(e) => field('address', e.target.value)}
        />
      </label>
      <label>
        Client notes
        <textarea
          maxLength={4000}
          value={value.notes}
          onChange={(e) => field('notes', e.target.value)}
        />
      </label>
      {record && (
        <label className="check">
          <input
            type="checkbox"
            checked={value.archived}
            onChange={(e) => field('archived', e.target.checked)}
          />
          Archive client (keep orders and payments)
        </label>
      )}
    </Form>
  );
}
export function TemplateEditor({
  store,
  record,
  onClose,
}: {
  store: Store;
  record?: Template;
  onClose: () => void;
}) {
  const [name, setName] = useState(record?.name ?? '');
  const [notes, setNotes] = useState(record?.notes ?? '');
  const [archived, setArchived] = useState(!!record?.archived);
  const [dirty, mark] = useState(false);
  const [error, setError] = useState<unknown>(null);
  return (
    <Form
      title={record ? 'Edit stage' : 'New stage'}
      busy={store.busy}
      dirty={dirty}
      onClose={onClose}
      error={error}
      onSubmit={async (e) => {
        e.preventDefault();
        try {
          await store.write({
            kind: 'save_template',
            id: record?.id ?? null,
            expected_version: record?.version ?? null,
            name,
            notes,
            archived,
          });
          onClose();
        } catch (e) {
          setError(e);
        }
      }}
    >
      <label>
        Stage name
        <input
          required
          maxLength={80}
          value={name}
          onChange={(e) => {
            mark(true);
            setName(e.target.value);
          }}
        />
      </label>
      <label>
        Stage notes
        <textarea
          maxLength={1000}
          value={notes}
          onChange={(e) => {
            mark(true);
            setNotes(e.target.value);
          }}
        />
      </label>
      {record && (
        <label className="check">
          <input
            type="checkbox"
            checked={archived}
            onChange={(e) => {
              mark(true);
              setArchived(e.target.checked);
            }}
          />
          Archive stage
        </label>
      )}
      <p className="hint">
        Renaming or archiving a template preserves the names and prices saved on existing orders.
      </p>
    </Form>
  );
}
type ReceiptForm = {
  amount: string;
  business_date: string;
  method: ReceiptInput['method'];
  reference: string;
  notes: string;
};
const newReceipt = (today: string): ReceiptForm => ({
  amount: '',
  business_date: today,
  method: 'cash',
  reference: '',
  notes: '',
});
function asReceipt(r: ReceiptForm): ReceiptInput {
  return {
    amount_fils: parseScaled(r.amount),
    business_date: r.business_date,
    method: r.method,
    reference: r.reference,
    notes: r.notes,
  };
}
function ReceiptFields({
  value,
  onChange,
}: {
  value: ReceiptForm;
  onChange: (v: ReceiptForm) => void;
}) {
  return (
    <>
      <div className="field-grid">
        <label>
          Amount · KWD
          <input
            required
            inputMode="decimal"
            value={value.amount}
            placeholder="0.000"
            onChange={(e) => onChange({ ...value, amount: e.target.value })}
          />
        </label>
        <label>
          Payment date
          <input
            required
            type="date"
            value={value.business_date}
            onChange={(e) => onChange({ ...value, business_date: e.target.value })}
          />
        </label>
      </div>
      <label>
        Method
        <select
          value={value.method}
          onChange={(e) => onChange({ ...value, method: e.target.value as ReceiptInput['method'] })}
        >
          <option value="cash">Cash</option>
          <option value="bank">Bank transfer</option>
          <option value="card">Card</option>
          <option value="other">Other</option>
        </select>
      </label>
      <label>
        Payment reference
        <input
          maxLength={200}
          value={value.reference}
          onChange={(e) => onChange({ ...value, reference: e.target.value })}
        />
      </label>
      <label>
        Payment notes
        <textarea
          maxLength={1000}
          value={value.notes}
          onChange={(e) => onChange({ ...value, notes: e.target.value })}
        />
      </label>
    </>
  );
}
export function ReceiptEditor({
  store,
  orderId,
  onClose,
}: {
  store: Store;
  orderId?: string;
  onClose: () => void;
}) {
  const [selected, setSelected] = useState(orderId ?? '');
  const [value, set] = useState(newReceipt(store.data.today));
  const [dirty, mark] = useState(false);
  const [error, setError] = useState<unknown>(null);
  const order = store.data.orders.find((o) => o.id === selected);
  return (
    <Form
      title="Record payment"
      busy={store.busy}
      dirty={dirty}
      onClose={onClose}
      error={error}
      label="Record payment"
      onSubmit={async (e) => {
        e.preventDefault();
        try {
          await store.write({ kind: 'receive', order_id: selected, receipt: asReceipt(value) });
          onClose();
        } catch (e) {
          setError(e);
        }
      }}
    >
      <label>
        Order
        <select
          required
          value={selected}
          onChange={(e) => {
            mark(true);
            setSelected(e.target.value);
          }}
        >
          <option value="">Choose an order</option>
          {store.data.orders
            .filter((o) => !o.archived)
            .map((o) => (
              <option value={o.id} key={o.id}>
                {o.number} · {o.name} · {o.client_name}
              </option>
            ))}
        </select>
      </label>
      {order && (
        <p className="balance-note">
          Current balance: <strong>{money(order.balance_fils)}</strong>. Overpayments remain
          customer credit.
        </p>
      )}
      <ReceiptFields
        value={value}
        onChange={(v) => {
          mark(true);
          set(v);
        }}
      />
    </Form>
  );
}
export function CorrectionEditor({
  store,
  record,
  onClose,
}: {
  store: Store;
  record: Payment;
  onClose: () => void;
}) {
  const [reason, setReason] = useState('');
  const [date, setDate] = useState(store.data.today);
  const [error, setError] = useState<unknown>(null);
  return (
    <Form
      title="Correct receipt"
      busy={store.busy}
      dirty={!!reason}
      onClose={onClose}
      error={error}
      label="Reverse receipt"
      onSubmit={async (e) => {
        e.preventDefault();
        try {
          await store.write({
            kind: 'correct_receipt',
            id: record.id,
            reason,
            business_date: date,
          });
          onClose();
        } catch (e) {
          setError(e);
        }
      }}
    >
      <p>
        Reverse <strong>{money(record.amount_fils)}</strong> for {record.order_name}. The original
        receipt remains visible. Record a replacement payment separately if needed.
      </p>
      <label>
        Correction date
        <input
          required
          type="date"
          min={record.business_date}
          value={date}
          onChange={(e) => setDate(e.target.value)}
        />
      </label>
      <label>
        Reason
        <input
          required
          maxLength={500}
          value={reason}
          onChange={(e) => setReason(e.target.value)}
        />
      </label>
      <p className="hint">
        This corrects the recorded balance; it does not transfer money to the customer.
      </p>
    </Form>
  );
}
type ItemForm = { id: string; description: string; dimensions: string; quantity: string };
type PriceForm = {
  selected: boolean;
  mode: Mode;
  fixed: string;
  quantity: string;
  rate: string;
  items: Record<string, string>;
};
const freshItem = (): ItemForm => ({
  id: crypto.randomUUID(),
  description: '',
  dimensions: '',
  quantity: '1',
});
export function OrderEditor({
  store,
  record,
  onClose,
  onSaved,
}: {
  store: Store;
  record?: Order;
  onClose: () => void;
  onSaved: (id: string) => void;
}) {
  const initial = record?.document.input;
  const [name, setName] = useState(initial?.name ?? '');
  const [client, setClient] = useState(initial?.client_id ?? '');
  const [due, setDue] = useState(initial?.due_date ?? '');
  const [paymentDue, setPaymentDue] = useState(initial?.payment_due_date ?? '');
  const [notes, setNotes] = useState(initial?.notes ?? '');
  const [status, setStatus] = useState(initial?.status ?? 'draft');
  const [reason, setReason] = useState('');
  const [items, setItems] = useState<ItemForm[]>(
    initial?.items.map((i) => ({ ...i, quantity: String(i.quantity) })) ?? [freshItem()],
  );
  const [prices, setPrices] = useState<Record<string, PriceForm>>(() =>
    Object.fromEntries(
      store.data.templates.map((t) => {
        const s = initial?.stages.find((s) => s.template_id === t.id);
        return [
          t.id,
          {
            selected: initial ? !!s : !t.archived,
            mode: s?.pricing_mode ?? 'unpriced',
            fixed: s?.fixed_total_fils ? decimal(s.fixed_total_fils) : '',
            quantity: s?.quantity_scaled ? decimal(s.quantity_scaled) : '',
            rate: s?.unit_price_fils ? decimal(s.unit_price_fils) : '',
            items: Object.fromEntries(
              s?.item_prices.map((p) => [p.item_id, decimal(p.unit_price_fils)]) ?? [],
            ),
          },
        ];
      }),
    ),
  );
  const [deposit, setDeposit] = useState(false);
  const [receipt, setReceipt] = useState(newReceipt(store.data.today));
  const [dirty, mark] = useState(false);
  const [error, setError] = useState<unknown>(null);
  const [addingClient, setAddingClient] = useState(false);
  const updatePrice = (id: string, patch: Partial<PriceForm>) => {
    mark(true);
    setPrices((p) => ({ ...p, [id]: { ...p[id], ...patch } }));
  };
  const available = store.data.templates
    .filter((t) => !t.archived || prices[t.id]?.selected)
    .sort((a, b) => {
      const ai = initial?.stages.findIndex((s) => s.template_id === a.id) ?? -1;
      const bi = initial?.stages.findIndex((s) => s.template_id === b.id) ?? -1;
      return ai >= 0 || bi >= 0
        ? (ai < 0 ? 999 : ai) - (bi < 0 ? 999 : bi)
        : a.position - b.position;
    });
  const input = (): OrderInput => {
    const boxes = items.map((i) => {
      if (!/^\d+$/.test(i.quantity) || Number(i.quantity) < 1 || Number(i.quantity) > 1_000_000)
        throw new Error('Enter 1–1,000,000 whole boxes per type.');
      return { ...i, quantity: Number(i.quantity) };
    });
    const stages = available
      .filter((t) => prices[t.id]?.selected)
      .map((t) => {
        const p = prices[t.id];
        const s: StagePrice = {
          template_id: t.id,
          pricing_mode: p.mode,
          fixed_total_fils: null,
          quantity_scaled: null,
          unit_price_fils: null,
          item_prices: [],
        };
        if (p.mode === 'fixed') s.fixed_total_fils = parseScaled(p.fixed);
        if (p.mode === 'per_unit') {
          s.quantity_scaled = parseScaled(p.quantity, 1_000_000_000n);
          s.unit_price_fils = parseScaled(p.rate);
        }
        if (p.mode === 'per_box')
          s.item_prices = items.map((i) => ({
            item_id: i.id,
            unit_price_fils: parseScaled(p.items[i.id] ?? ''),
          }));
        return s;
      });
    return {
      client_id: client,
      name,
      due_date: due || null,
      payment_due_date: paymentDue || null,
      notes,
      status,
      items: boxes,
      stages,
    };
  };
  let preview: ReturnType<typeof quote> | undefined;
  try {
    const v = input();
    preview = quote(v.items, v.stages);
  } catch {
    /* Incomplete price fields are explained by the form on save. */
  }
  if (addingClient)
    return (
      <ClientEditor
        store={store}
        onClose={() => setAddingClient(false)}
        onSaved={(id) => {
          setClient(id);
          mark(true);
        }}
      />
    );
  return (
    <Form
      title={record ? 'Revise order' : 'New order'}
      busy={store.busy}
      dirty={dirty}
      onClose={onClose}
      error={error}
      label={record ? 'Save revision' : 'Save order'}
      onSubmit={async (e) => {
        e.preventDefault();
        try {
          const v = input();
          if (!v.stages.length) throw new Error('Select at least one production stage.');
          quote(v.items, v.stages);
          const result = await store.write({
            kind: 'save_order',
            id: record?.id ?? null,
            expected_version: record?.version ?? null,
            input: v,
            reason,
            deposit: deposit ? asReceipt(receipt) : null,
          });
          if (result.id) onSaved(result.id);
          onClose();
        } catch (e) {
          setError(e);
        }
      }}
    >
      <label>
        Order name
        <input
          required
          maxLength={120}
          value={name}
          placeholder="e.g. Gift boxes"
          onChange={(e) => {
            mark(true);
            setName(e.target.value);
          }}
        />
      </label>
      <div className="label-row">
        <label className="grow">
          Client
          <select
            required
            value={client}
            onChange={(e) => {
              mark(true);
              setClient(e.target.value);
            }}
          >
            <option value="">Choose a client</option>
            {store.data.clients
              .filter((c) => !c.archived || c.id === client)
              .map((c) => (
                <option key={c.id} value={c.id}>
                  {c.name}
                </option>
              ))}
          </select>
        </label>
        <button
          type="button"
          className="button"
          onClick={() => setAddingClient(true)}
          aria-label="Add client from order"
        >
          <Plus size={18} />
        </button>
      </div>
      <div className="field-grid">
        <label>
          Production due date
          <input
            type="date"
            value={due}
            onChange={(e) => {
              mark(true);
              setDue(e.target.value);
            }}
          />
        </label>
        <label>
          Payment due date
          <input
            type="date"
            value={paymentDue}
            onChange={(e) => {
              mark(true);
              setPaymentDue(e.target.value);
            }}
          />
        </label>
      </div>
      <div className="section-title">
        <h3>Box types</h3>
        <button
          type="button"
          className="text-btn"
          disabled={items.length >= 100}
          onClick={() => {
            mark(true);
            setItems((p) => [...p, freshItem()]);
          }}
        >
          <Plus size={16} /> Add box type
        </button>
      </div>
      {items.map((item, index) => (
        <section className="form-card" key={item.id}>
          <div className="between">
            <strong>Box type {index + 1}</strong>
            <button
              type="button"
              className="icon-btn"
              aria-label={`Remove box type ${index + 1}`}
              disabled={items.length === 1}
              onClick={() => {
                mark(true);
                setItems((p) => p.filter((i) => i.id !== item.id));
              }}
            >
              <Trash2 size={17} />
            </button>
          </div>
          {(
            [
              ['description', 'Description'],
              ['dimensions', 'Dimensions / sheet description'],
              ['quantity', 'Ordered boxes'],
            ] as const
          ).map(([key, label]) => (
            <label key={key}>
              {label}
              <input
                required={key !== 'dimensions'}
                maxLength={key === 'description' ? 160 : key === 'dimensions' ? 200 : 7}
                inputMode={key === 'quantity' ? 'numeric' : undefined}
                value={item[key]}
                onChange={(e) => {
                  mark(true);
                  setItems((p) =>
                    p.map((i) => (i.id === item.id ? { ...i, [key]: e.target.value } : i)),
                  );
                }}
              />
            </label>
          ))}
        </section>
      ))}
      <h3>Production stages & customer pricing</h3>
      <p className="hint">Select the stages for this order. Their customer prices add up below.</p>
      {available.map((t) => {
        const p = prices[t.id] ?? {
          selected: false,
          mode: 'unpriced' as Mode,
          fixed: '',
          quantity: '',
          rate: '',
          items: {},
        };
        const previous = initial?.stages.findIndex((s) => s.template_id === t.id) ?? -1;
        const stageName = previous >= 0 ? record!.document.stage_names[previous] : t.name;
        return (
          <section className={`form-card stage-card ${p.selected ? 'selected' : ''}`} key={t.id}>
            <label className="check stage-select">
              <input
                type="checkbox"
                checked={p.selected}
                onChange={(e) => updatePrice(t.id, { selected: e.target.checked })}
              />
              {stageName}
            </label>
            {p.selected && (
              <>
                <label>
                  Customer pricing · {stageName}
                  <select
                    aria-label={`Customer pricing · ${stageName}`}
                    value={p.mode}
                    onChange={(e) => updatePrice(t.id, { mode: e.target.value as Mode })}
                  >
                    <option value="unpriced">Not priced yet</option>
                    <option value="fixed">Fixed stage total</option>
                    <option value="per_box">Price per finished box</option>
                    <option value="per_unit">Quantity × price per Unit</option>
                  </select>
                </label>
                {p.mode === 'fixed' && (
                  <label>
                    Fixed total · KWD
                    <input
                      required
                      inputMode="decimal"
                      value={p.fixed}
                      onChange={(e) => updatePrice(t.id, { fixed: e.target.value })}
                    />
                  </label>
                )}
                {p.mode === 'per_unit' && (
                  <div className="field-grid">
                    <label>
                      Quantity · Unit
                      <input
                        required
                        inputMode="decimal"
                        value={p.quantity}
                        onChange={(e) => updatePrice(t.id, { quantity: e.target.value })}
                      />
                    </label>
                    <label>
                      Price per Unit · KWD
                      <input
                        required
                        inputMode="decimal"
                        value={p.rate}
                        onChange={(e) => updatePrice(t.id, { rate: e.target.value })}
                      />
                    </label>
                  </div>
                )}
                {p.mode === 'per_box' &&
                  items.map((i, index) => (
                    <label key={i.id}>
                      {i.description || `Box type ${index + 1}`} · KWD / box
                      <input
                        required
                        inputMode="decimal"
                        value={p.items[i.id] ?? ''}
                        onChange={(e) =>
                          updatePrice(t.id, { items: { ...p.items, [i.id]: e.target.value } })
                        }
                      />
                    </label>
                  ))}
              </>
            )}
          </section>
        );
      })}
      <section className="quote-summary">
        <span>Customer quote</span>
        <strong>{preview ? money(preview.total) : 'Complete pricing inputs'}</strong>
        {preview?.total === null && (
          <small>Known subtotal: {money(preview.known)} · some stages are unpriced</small>
        )}
      </section>
      <label>
        Order status
        <select
          value={status}
          onChange={(e) => {
            mark(true);
            setStatus(e.target.value as 'draft' | 'confirmed');
          }}
        >
          <option value="draft">Draft</option>
          <option value="confirmed">Confirmed</option>
        </select>
      </label>
      <label>
        Order notes / sheet quantities
        <textarea
          maxLength={4000}
          value={notes}
          onChange={(e) => {
            mark(true);
            setNotes(e.target.value);
          }}
        />
      </label>
      {record ? (
        <label>
          Reason for revision
          <input
            required
            maxLength={500}
            value={reason}
            onChange={(e) => {
              mark(true);
              setReason(e.target.value);
            }}
          />
        </label>
      ) : (
        <>
          <label className="check">
            <input
              type="checkbox"
              checked={deposit}
              onChange={(e) => {
                mark(true);
                setDeposit(e.target.checked);
              }}
            />
            Record a deposit with this order
          </label>
          {deposit && (
            <ReceiptFields
              value={receipt}
              onChange={(v) => {
                mark(true);
                setReceipt(v);
              }}
            />
          )}
        </>
      )}
    </Form>
  );
}
