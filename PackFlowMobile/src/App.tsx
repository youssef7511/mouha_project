import { useEffect, useRef, useState } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import {
  Box,
  Boxes,
  Users,
  Wallet,
  Settings,
  Plus,
  Search,
  ArrowLeft,
  ArrowUp,
  ArrowDown,
  ChevronRight,
  Download,
  RotateCcw,
} from 'lucide-react';
import type {
  Action,
  Snapshot,
  Order,
  Payment,
  Template,
  Revision,
  SaveResult,
  Operation,
} from './types';
import { api, message, downloadBackup, restoreBackup } from './api';
import { money } from './money';
import { Sheet, Empty, ErrorLine } from './Sheet';
import {
  ClientEditor,
  TemplateEditor,
  OrderEditor,
  ReceiptEditor,
  CorrectionEditor,
  type Store,
} from './forms';
type Tab = 'orders' | 'clients' | 'payments' | 'settings';
type Modal =
  | { kind: 'client'; id?: string }
  | { kind: 'order'; id?: string }
  | { kind: 'receipt'; orderId?: string }
  | { kind: 'stage'; id?: string }
  | { kind: 'correction'; id: string }
  | { kind: 'history'; id: string };
const sum = (values: string[]) => values.reduce((s, v) => s + BigInt(v), 0n).toString();
function badge(order: Order) {
  if (order.total_fils === null) return 'Not priced';
  const b = BigInt(order.balance_fils!);
  return b < 0n
    ? 'Customer credit'
    : b === 0n
      ? 'Paid'
      : BigInt(order.received_fils) > 0n
        ? 'Partially paid'
        : 'Unpaid';
}
export function App() {
  const [data, setData] = useState<Snapshot | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const locked = useRef(false);
  const uncertain = useRef<{ payload: string; request: Operation } | null>(null);
  const [tab, setTab] = useState<Tab>('orders');
  const [selected, setSelected] = useState<string | null>(null);
  const [modal, setModal] = useState<Modal | null>(null);
  const [search, setSearch] = useState('');
  const [archived, setArchived] = useState(false);
  const [limit, setLimit] = useState(20);
  const [clientFilter, setClientFilter] = useState<string | null>(null);
  const refresh = async () => {
    try {
      const next = await api.snapshot();
      setData(next);
      setError(null);
    } catch (e) {
      setError(e);
    } finally {
      setLoading(false);
    }
  };
  useEffect(() => {
    void refresh();
  }, []);
  const write = async (action: Action): Promise<SaveResult> => {
    if (locked.current) throw new Error('Another save is still running.');
    locked.current = true;
    setBusy(true);
    setError(null);
    const payload = JSON.stringify(action);
    if (uncertain.current?.payload !== payload)
      uncertain.current = { payload, request: { request_id: crypto.randomUUID(), action } };
    try {
      const result = await api.mutate(uncertain.current.request);
      uncertain.current = null;
      await refresh();
      return result;
    } finally {
      locked.current = false;
      setBusy(false);
    }
  };
  const navigate = (next: Tab) => {
    setTab(next);
    setSelected(null);
    setSearch('');
    setArchived(false);
    setLimit(20);
    setClientFilter(null);
  };
  const perform = async (fn: () => Promise<unknown>) => {
    try {
      await fn();
    } catch (e) {
      setError(e);
    }
  };
  const restore = async (file: File, confirmation: string) => {
    if (locked.current) throw new Error('Another save is still running.');
    locked.current = true;
    setBusy(true);
    try {
      const result = await restoreBackup(file, confirmation);
      uncertain.current = null;
      await refresh();
      return result;
    } finally {
      locked.current = false;
      setBusy(false);
    }
  };
  const store: Store | undefined = data ? { data, busy, write, restore } : undefined;
  const current = data?.orders.find((o) => o.id === selected);
  const filteredOrders =
    data?.orders.filter(
      (o) =>
        (archived || !o.archived) &&
        (!clientFilter || o.client_id === clientFilter) &&
        `${o.number} ${o.name} ${o.client_name}`.toLowerCase().includes(search.toLowerCase()),
    ) ?? [];
  const filteredClients =
    data?.clients.filter(
      (c) =>
        (archived || !c.archived) &&
        `${c.name} ${c.contact_name} ${c.phone} ${c.email}`
          .toLowerCase()
          .includes(search.toLowerCase()),
    ) ?? [];
  const filteredPayments =
    data?.payments.filter((p) =>
      `${p.number} ${p.order_name} ${p.client_name} ${p.reference}`
        .toLowerCase()
        .includes(search.toLowerCase()),
    ) ?? [];
  const outstanding =
    data?.orders.flatMap((o) =>
      o.balance_fils !== null && BigInt(o.balance_fils) > 0n ? [o.balance_fils] : [],
    ) ?? [];
  const credits =
    data?.orders.flatMap((o) =>
      o.balance_fils !== null && BigInt(o.balance_fils) < 0n
        ? [(-BigInt(o.balance_fils)).toString()]
        : [],
    ) ?? [];
  return (
    <div className="phone-shell">
      <header className="app-header">
        <button className="brand" onClick={() => navigate('orders')} aria-label="PackFlow home">
          <span>
            <Box size={24} />
          </span>
          <div>
            PackFlow<small>MOBILE</small>
          </div>
        </button>
        <span className="device-badge">
          <i />
          {isTauri() ? 'On this phone' : 'Local preview'}
        </span>
      </header>
      <main className="main">
        <ErrorLine error={error} />
        {loading ? (
          <div className="loading" role="status">
            Opening your local workspace…
          </div>
        ) : !data ? (
          <Empty title="Workspace unavailable">
            <button className="button" onClick={() => void refresh()}>
              Retry connection
            </button>
          </Empty>
        ) : (
          <>
            {current ? (
              <OrderDetail
                store={store!}
                order={current}
                back={() => setSelected(null)}
                edit={() => setModal({ kind: 'order', id: current.id })}
                payment={() => setModal({ kind: 'receipt', orderId: current.id })}
                history={() => setModal({ kind: 'history', id: current.id })}
                correct={(id) => setModal({ kind: 'correction', id })}
                perform={perform}
              />
            ) : (
              <>
                {tab === 'orders' && (
                  <>
                    <div className="page-title">
                      <div>
                        <p className="eyebrow">{data.profile.name}</p>
                        <h1>A little more order.</h1>
                        <p>Every box, every stage, in your pocket.</p>
                      </div>
                    </div>
                    <div className="metrics">
                      <div>
                        <span>Outstanding · this phone</span>
                        <strong>{money(sum(outstanding))}</strong>
                      </div>
                      <div>
                        <span>Net received</span>
                        <strong>{money(sum(data.payments.map((p) => p.signed_fils)))}</strong>
                      </div>
                    </div>
                    {credits.length > 0 && (
                      <p className="hint">
                        Customer credit: {money(sum(credits))} · kept separate from outstanding
                        balances.
                      </p>
                    )}
                    <div className="section-title">
                      <h2>
                        Orders <span>{data.orders.filter((o) => !o.archived).length}</span>
                      </h2>
                      <button
                        className="button primary"
                        disabled={busy}
                        onClick={() => setModal({ kind: 'order' })}
                      >
                        <Plus size={18} />
                        New order
                      </button>
                    </div>
                    {clientFilter && (
                      <button className="filter-chip" onClick={() => setClientFilter(null)}>
                        {data.clients.find((c) => c.id === clientFilter)?.name} · Clear filter ×
                      </button>
                    )}
                    <SearchBox
                      value={search}
                      onChange={(v) => {
                        setSearch(v);
                        setLimit(20);
                      }}
                      label="Search orders"
                    />
                    <label className="check archive-toggle">
                      <input
                        type="checkbox"
                        checked={archived}
                        onChange={(e) => setArchived(e.target.checked)}
                      />
                      Include archived orders
                    </label>
                    {filteredOrders.length ? (
                      filteredOrders.slice(0, limit).map((o) => (
                        <button className="order-card" key={o.id} onClick={() => setSelected(o.id)}>
                          <div className="between">
                            <span className="reference">{o.number}</span>
                            <span className={`pill ${o.status === 'confirmed' ? 'green' : ''}`}>
                              {o.archived ? 'Archived' : o.status}
                            </span>
                          </div>
                          <h3>{o.name}</h3>
                          <p>{o.client_name}</p>
                          <div className="between card-bottom">
                            <div>
                              <strong>{money(o.total_fils)}</strong>
                              <small>
                                {o.document.input.items
                                  .reduce((sum, i) => sum + i.quantity, 0)
                                  .toLocaleString()}{' '}
                                boxes · {o.document.input.stages.length} stages
                              </small>
                            </div>
                            <span className="pill">{badge(o)}</span>
                            <ChevronRight size={17} />
                          </div>
                        </button>
                      ))
                    ) : (
                      <Empty title={search ? 'No matching orders' : 'Your next order starts here'}>
                        Add a client, choose box types and price each stage. Everything is saved
                        locally.
                      </Empty>
                    )}
                    <More
                      count={filteredOrders.length}
                      limit={limit}
                      onMore={() => setLimit((n) => n + 20)}
                    />
                  </>
                )}
                {tab === 'clients' && (
                  <>
                    <div className="page-title">
                      <p className="eyebrow">THE PEOPLE BEHIND THE BOXES</p>
                      <h1>Your clients.</h1>
                      <p>Contacts and balances from this phone’s records.</p>
                    </div>
                    <button
                      className="button primary full"
                      onClick={() => setModal({ kind: 'client' })}
                      disabled={busy}
                    >
                      <Plus size={18} />
                      New client
                    </button>
                    <SearchBox
                      value={search}
                      onChange={(v) => {
                        setSearch(v);
                        setLimit(20);
                      }}
                      label="Search clients"
                    />
                    <label className="check archive-toggle">
                      <input
                        type="checkbox"
                        checked={archived}
                        onChange={(e) => setArchived(e.target.checked)}
                      />
                      Include archived clients
                    </label>
                    {filteredClients.slice(0, limit).map((c) => {
                      const orders = data.orders.filter((o) => o.client_id === c.id);
                      return (
                        <section className="client-card" key={c.id}>
                          <div className="between">
                            <div className="client-heading">
                              <span className="avatar">{c.name.slice(0, 1).toUpperCase()}</span>
                              <div>
                                <h3>{c.name}</h3>
                                <p>
                                  {c.contact_name || 'Client'}
                                  {c.archived ? ' · Archived' : ''}
                                </p>
                              </div>
                            </div>
                            <button
                              className="text-btn"
                              onClick={() => setModal({ kind: 'client', id: c.id })}
                            >
                              Edit
                            </button>
                          </div>
                          {c.phone && (
                            <a
                              href={`tel:${c.phone.replace(/[^+\d]/g, '')}`}
                              className="contact-link"
                            >
                              {c.phone}
                            </a>
                          )}
                          <div className="between card-bottom">
                            <small>
                              {orders.length} orders ·{' '}
                              {money(sum(orders.map((o) => o.received_fils)))} received
                            </small>
                            <button
                              className="text-btn"
                              onClick={() => {
                                navigate('orders');
                                setClientFilter(c.id);
                              }}
                            >
                              Orders <ChevronRight size={15} />
                            </button>
                          </div>
                        </section>
                      );
                    })}
                    {!filteredClients.length && (
                      <Empty title={search ? 'No matching clients' : 'A good place to begin'}>
                        Save a client’s name and contact details, then create their first order.
                      </Empty>
                    )}
                    <More
                      count={filteredClients.length}
                      limit={limit}
                      onMore={() => setLimit((n) => n + 20)}
                    />
                  </>
                )}
                {tab === 'payments' && (
                  <>
                    <div className="page-title">
                      <p className="eyebrow">EVERY PAYMENT, CONNECTED</p>
                      <h1>Payments.</h1>
                      <p>Order receipts, corrections and customer credit.</p>
                    </div>
                    <div className="metrics">
                      <div>
                        <span>Net received · this phone</span>
                        <strong>{money(sum(data.payments.map((p) => p.signed_fils)))}</strong>
                      </div>
                      <div>
                        <span>Outstanding</span>
                        <strong>{money(sum(outstanding))}</strong>
                      </div>
                    </div>
                    <button
                      className="button primary full"
                      disabled={busy || !data.orders.some((o) => !o.archived)}
                      onClick={() => setModal({ kind: 'receipt' })}
                    >
                      <Plus size={18} />
                      Record payment
                    </button>
                    <SearchBox
                      value={search}
                      onChange={(v) => {
                        setSearch(v);
                        setLimit(20);
                      }}
                      label="Search payments"
                    />
                    {filteredPayments.slice(0, limit).map((p) => (
                      <PaymentCard
                        key={p.id}
                        payment={p}
                        onCorrect={() => setModal({ kind: 'correction', id: p.id })}
                        onOrder={() => {
                          setTab('orders');
                          setSelected(p.order_id);
                        }}
                      />
                    ))}
                    {!filteredPayments.length && (
                      <Empty
                        title={
                          search ? 'No matching payments' : 'Ready when the first deposit arrives'
                        }
                      >
                        Record a payment against an order to keep the balance connected.
                      </Empty>
                    )}
                    <More
                      count={filteredPayments.length}
                      limit={limit}
                      onMore={() => setLimit((n) => n + 20)}
                    />
                  </>
                )}
                {tab === 'settings' && (
                  <WorkspaceSettings
                    store={store!}
                    refresh={refresh}
                    stage={(id) => setModal({ kind: 'stage', id })}
                    perform={perform}
                  />
                )}
              </>
            )}
          </>
        )}
      </main>
      <nav className="bottom-nav" aria-label="Phone navigation">
        {(
          [
            ['orders', 'Orders', Boxes],
            ['clients', 'Clients', Users],
            ['payments', 'Payments', Wallet],
            ['settings', 'Settings', Settings],
          ] as const
        ).map(([key, label, Icon]) => (
          <button
            key={key}
            disabled={busy}
            aria-current={tab === key ? 'page' : undefined}
            onClick={() => navigate(key)}
          >
            <Icon size={22} />
            <span>{label}</span>
          </button>
        ))}
      </nav>
      {data && store && modal?.kind === 'client' && (
        <ClientEditor
          store={store}
          record={data.clients.find((c) => c.id === modal.id)}
          onClose={() => setModal(null)}
        />
      )}
      {data && store && modal?.kind === 'stage' && (
        <TemplateEditor
          store={store}
          record={data.templates.find((t) => t.id === modal.id)}
          onClose={() => setModal(null)}
        />
      )}
      {data && store && modal?.kind === 'order' && (
        <OrderEditor
          store={store}
          record={data.orders.find((o) => o.id === modal.id)}
          onClose={() => setModal(null)}
          onSaved={(id) => {
            setTab('orders');
            setSelected(id);
          }}
        />
      )}
      {store && modal?.kind === 'receipt' && (
        <ReceiptEditor store={store} orderId={modal.orderId} onClose={() => setModal(null)} />
      )}
      {store && modal?.kind === 'correction' && (
        <CorrectionEditor
          store={store}
          record={data!.payments.find((p) => p.id === modal.id)!}
          onClose={() => setModal(null)}
        />
      )}
      {data && modal?.kind === 'history' && (
        <History
          order={data.orders.find((o) => o.id === modal.id)!}
          onClose={() => setModal(null)}
        />
      )}
    </div>
  );
}
function SearchBox({
  value,
  onChange,
  label,
}: {
  value: string;
  onChange: (v: string) => void;
  label: string;
}) {
  return (
    <label className="search">
      <Search size={18} />
      <input
        type="search"
        aria-label={label}
        placeholder={label + '…'}
        value={value}
        onChange={(e) => onChange(e.target.value)}
      />
    </label>
  );
}
function More({ count, limit, onMore }: { count: number; limit: number; onMore: () => void }) {
  return count > limit ? (
    <button className="button full" onClick={onMore}>
      Show more · {Math.min(limit, count)} of {count}
    </button>
  ) : null;
}
function PaymentCard({
  payment: p,
  onCorrect,
  onOrder,
}: {
  payment: Payment;
  onCorrect: () => void;
  onOrder?: () => void;
}) {
  return (
    <section className="payment-card">
      <div className="between">
        <span className="reference">
          {p.business_date} · {p.method}
        </span>
        <span className={`pill ${p.kind === 'correction' ? 'orange' : p.corrected ? '' : 'green'}`}>
          {p.kind === 'correction' ? 'Correction' : p.corrected ? 'Corrected' : 'Receipt'}
        </span>
      </div>
      <div className="between">
        <div>
          <h3>{money(p.signed_fils)}</h3>
          <button className="link" onClick={onOrder}>
            {p.order_name}
          </button>
          <p>{p.client_name}</p>
        </div>
        {p.kind === 'receipt' && !p.corrected && (
          <button className="text-btn" onClick={onCorrect}>
            Correct
          </button>
        )}
      </div>
      {p.reference && <small>{p.reference}</small>}
      {p.reason && <p className="hint">{p.reason}</p>}
    </section>
  );
}
function OrderDetail({
  store,
  order: o,
  back,
  edit,
  payment,
  history,
  correct,
  perform,
}: {
  store: Store;
  order: Order;
  back: () => void;
  edit: () => void;
  payment: () => void;
  history: () => void;
  correct: (id: string) => void;
  perform: (f: () => Promise<unknown>) => Promise<void>;
}) {
  const d = o.document;
  const rows = store.data.payments.filter((p) => p.order_id === o.id);
  return (
    <>
      <button className="text-btn back" onClick={back}>
        <ArrowLeft size={18} />
        All orders
      </button>
      <div className="page-title">
        <p className="eyebrow">
          {o.number} · {o.status}
          {o.archived ? ' · archived' : ''}
        </p>
        <h1>{o.name}</h1>
        <p>{o.client_name}</p>
      </div>
      <div className="quote-summary">
        <span>Customer quote</span>
        <strong>{money(o.total_fils)}</strong>
        {o.total_fils === null && <small>Known subtotal {money(o.known_total_fils)}</small>}
        <div className="between">
          <span>{badge(o)}</span>
          <small>Revision {o.version}</small>
        </div>
      </div>
      <div className="metrics">
        <div>
          <span>Net received</span>
          <strong>{money(o.received_fils)}</strong>
        </div>
        <div>
          <span>
            {o.balance_fils !== null && BigInt(o.balance_fils) < 0n ? 'Customer credit' : 'Balance'}
          </span>
          <strong>
            {money(
              o.balance_fils === null
                ? null
                : (BigInt(o.balance_fils) < 0n
                    ? -BigInt(o.balance_fils)
                    : BigInt(o.balance_fils)
                  ).toString(),
            )}
          </strong>
        </div>
      </div>
      {d.input.payment_due_date && (
        <p className="balance-note">
          Payment due {d.input.payment_due_date}
          {d.input.payment_due_date < store.data.today &&
          o.balance_fils !== null &&
          BigInt(o.balance_fils) > 0n
            ? ' · Overdue'
            : ''}
        </p>
      )}
      <div className="actions">
        <button className="button" disabled={store.busy || !!o.archived} onClick={edit}>
          Revise order
        </button>
        <button className="button primary" disabled={store.busy || !!o.archived} onClick={payment}>
          <Plus size={17} />
          Payment
        </button>
      </div>
      <h2 className="section-label">Box types</h2>
      {d.input.items.map((i) => (
        <section className="detail-row" key={i.id}>
          <div>
            <h3>{i.description}</h3>
            <p>{i.dimensions || 'No dimensions entered'}</p>
          </div>
          <strong>{i.quantity.toLocaleString()} boxes</strong>
        </section>
      ))}
      <h2 className="section-label">Stage pricing</h2>
      {d.input.stages.map((s, index) => (
        <section className="stage-detail" key={s.template_id}>
          <div className="between">
            <h3>
              {index + 1}. {d.stage_names[index]}
            </h3>
            <strong>{money(d.stage_totals[index])}</strong>
          </div>
          <p>
            {
              {
                unpriced: 'Not priced yet',
                fixed: 'Fixed stage total',
                per_box: 'Price per finished box',
                per_unit: 'Quantity × price per Unit',
              }[s.pricing_mode]
            }
          </p>
          {s.pricing_mode === 'per_box' &&
            s.item_prices.map((p) => (
              <small key={p.item_id}>
                {d.input.items.find((i) => i.id === p.item_id)?.description}:{' '}
                {money(p.unit_price_fils)} / box
              </small>
            ))}
        </section>
      ))}
      {(d.input.due_date || d.input.notes) && (
        <section className="note-card">
          <h3>Order notes</h3>
          {d.input.due_date && <p>Production due {d.input.due_date}</p>}
          <p className="preserve-lines">{d.input.notes}</p>
        </section>
      )}
      <div className="section-title">
        <h2>Payment history</h2>
        <button className="text-btn" onClick={history}>
          Quote history
        </button>
      </div>
      {rows.map((p) => (
        <PaymentCard key={p.id} payment={p} onCorrect={() => correct(p.id)} />
      ))}
      {!rows.length && <p className="hint">No payments recorded yet.</p>}
      <button
        className="button full"
        disabled={store.busy}
        onClick={() =>
          void perform(() =>
            store.write({
              kind: 'archive_order',
              id: o.id,
              expected_version: o.version,
              archived: !o.archived,
            }),
          )
        }
      >
        {o.archived ? 'Restore order' : 'Archive order'}
      </button>
      <p className="hint">
        Archiving hides the order from the default list. Financial history and balances remain
        recorded.
      </p>
    </>
  );
}
function History({ order, onClose }: { order: Order; onClose: () => void }) {
  const [data, set] = useState<Revision[] | null>(null);
  const [error, setError] = useState<unknown>(null);
  useEffect(() => {
    api
      .query<{ revisions: Revision[] }>({ kind: 'history', order_id: order.id })
      .then((r) => set(r.revisions))
      .catch(setError);
  }, [order.id, order.version]);
  return (
    <Sheet title="Quote history" onClose={onClose}>
      <div className="sheet-body">
        <ErrorLine error={error} />
        {!data && !error && <p>Loading revisions…</p>}
        {data?.map((r) => {
          const doc = JSON.parse(r.document) as Order['document'];
          return (
            <details className="history-card" key={r.version}>
              <summary>
                <strong>
                  Revision {r.version} · {money(r.total_fils)}
                </strong>
                <small>{new Date(r.created_at).toLocaleString()}</small>
                <p>{r.reason || 'Order created'}</p>
              </summary>
              {doc.input.items.map((i) => (
                <p key={i.id}>
                  {i.description} · {i.quantity} boxes · {i.dimensions}
                </p>
              ))}
              {doc.stage_names.map((name, i) => (
                <p key={i}>
                  {name}: {money(doc.stage_totals[i])}
                </p>
              ))}
            </details>
          );
        })}
      </div>
    </Sheet>
  );
}
function WorkspaceSettings({
  store,
  refresh,
  stage,
  perform,
}: {
  store: Store;
  refresh: () => Promise<void>;
  stage: (id?: string) => void;
  perform: (f: () => Promise<unknown>) => Promise<void>;
}) {
  const [name, setName] = useState(store.data.profile.name);
  const [notice, setNotice] = useState('');
  const [file, setFile] = useState<File | null>(null);
  const [confirmation, setConfirmation] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<unknown>(null);
  const [backups, setBackups] = useState<string[]>([]);
  const loadBackups = () =>
    api.query<string[]>({ kind: 'backups' }).then(setBackups).catch(setError);
  useEffect(() => {
    void loadBackups();
  }, []);
  const task = async (fn: () => Promise<void>) => {
    if (busy) return;
    setBusy(true);
    setError(null);
    setNotice('');
    try {
      await fn();
    } catch (e) {
      setError(e);
    } finally {
      setBusy(false);
    }
  };
  const move = (id: string, direction: number) => {
    const templates = [...store.data.templates];
    const i = templates.findIndex((t) => t.id === id);
    if (i + direction < 0 || i + direction >= templates.length) return;
    [templates[i], templates[i + direction]] = [templates[i + direction], templates[i]];
    void perform(() =>
      store.write({
        kind: 'reorder_templates',
        ids: templates.map((t) => t.id),
        versions: templates.map((t) => t.version),
      }),
    );
  };
  return (
    <>
      <div className="page-title">
        <p className="eyebrow">YOUR LOCAL WORKSPACE</p>
        <h1>Make it yours.</h1>
        <p>Independent phone records. Currency: KWD.</p>
      </div>
      <section className="settings-card">
        <h2>Workspace</h2>
        <form
          onSubmit={(e) => {
            e.preventDefault();
            void task(async () => {
              await store.write({ kind: 'profile', name });
              setNotice('Workspace name saved.');
            });
          }}
        >
          <label>
            Workspace name
            <input
              required
              maxLength={100}
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
          </label>
          <button className="button" disabled={busy || store.busy}>
            Save name
          </button>
        </form>
      </section>
      <section className="settings-card">
        <div className="section-title">
          <h2>Production stages</h2>
          <button className="text-btn" disabled={store.busy} onClick={() => stage()}>
            <Plus size={16} />
            Add stage
          </button>
        </div>
        <p className="hint">
          Impose means printing/impression. Changes apply to future orders; saved order snapshots
          remain intact.
        </p>
        {store.data.templates.map((t, i) => (
          <div className="template-row" key={t.id}>
            <button className="link grow" onClick={() => stage(t.id)}>
              <strong>{t.name}</strong>
              {!!t.archived && <small>Archived</small>}
            </button>
            <button
              className="icon-btn"
              aria-label={`Move ${t.name} up`}
              disabled={store.busy || i === 0}
              onClick={() => move(t.id, -1)}
            >
              <ArrowUp size={17} />
            </button>
            <button
              className="icon-btn"
              aria-label={`Move ${t.name} down`}
              disabled={store.busy || i === store.data.templates.length - 1}
              onClick={() => move(t.id, 1)}
            >
              <ArrowDown size={17} />
            </button>
          </div>
        ))}
      </section>
      <section className="settings-card">
        <h2>Mobile backup & restore</h2>
        <p>
          Save a backup outside this phone. A mobile backup restores this app’s records; it does not
          merge with the Mac.
        </p>
        <button
          className="button primary full"
          disabled={busy || store.busy}
          onClick={() =>
            void task(async () => {
              setNotice(await downloadBackup());
              await loadBackups();
            })
          }
        >
          <Download size={18} />
          Save mobile backup
        </button>
        <details className="restore-panel">
          <summary>Restore a mobile backup</summary>
          <label>
            Mobile backup file
            <input
              type="file"
              accept=".pfmobile"
              onChange={(e) => setFile(e.target.files?.[0] ?? null)}
            />
          </label>
          <label>
            Type RESTORE
            <input
              value={confirmation}
              onChange={(e) => setConfirmation(e.target.value)}
              autoComplete="off"
            />
          </label>
          <p className="hint">
            Restore replaces this phone’s records. A safety copy is saved first.
          </p>
          <button
            className="button danger full"
            disabled={busy || store.busy || !file || confirmation !== 'RESTORE'}
            onClick={() =>
              void task(async () => {
                const result = await store.restore(file!, confirmation);
                setNotice(result.message);
                setConfirmation('');
                setFile(null);
                await refresh();
                await loadBackups();
              })
            }
          >
            <RotateCcw size={17} />
            Restore records
          </button>
        </details>
        {backups.length > 0 && (
          <details className="restore-panel">
            <summary>Saved backups ({backups.length})</summary>
            {backups.map((name) => (
              <div className="backup-row" key={name}>
                <small>{name}</small>
                {!isTauri() && (
                  <button
                    className="text-btn"
                    onClick={() =>
                      void task(async () => {
                        setNotice(await downloadBackup(name));
                      })
                    }
                  >
                    Download
                  </button>
                )}
              </div>
            ))}
            {isTauri() && (
              <p className="hint">
                Use Files → On My iPhone → PackFlow Mobile → PackFlow Mobile Backups to share or
                restore these files.
              </p>
            )}
          </details>
        )}
        <ErrorLine error={error} />
        {notice && (
          <p role="status" className="notice">
            {notice}
          </p>
        )}
      </section>
      <p className="privacy-note">
        Saved on this device. No account with a server, cloud sync or Mac connection. Protect access
        with your phone’s screen lock.
      </p>
      <p className="version">PackFlow Mobile · 0.1.0</p>
    </>
  );
}
