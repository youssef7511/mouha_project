import { WorkspaceSafety } from '../features/LocalAccount';
import { PhoneImportButton } from '../features/PhoneImport';
import {
  ExcelExportButton,
  ExcelFilters,
  type ExportRequest,
  type ExportPage,
} from '../features/ExcelExport';
import { useEffect, useState } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import {
  Activity,
  ArrowDown,
  ArrowRight,
  ArrowUp,
  Box,
  ChartNoAxesCombined,
  Check,
  ChevronRight,
  FolderOpen,
  Grid2X2,
  Layers,
  Package,
  Plus,
  Search,
  Settings,
  Trash2,
  Users,
  Wallet,
  Warehouse,
  type LucideIcon,
} from 'lucide-react';
import type {
  Bootstrap,
  Client,
  OrderDetail,
  OrderStage,
  OrderSummary,
  StageTemplate,
  Worker,
} from '../contracts';
import { useBootstrap, useOrder, useOrders, useOrderPage, useWrite } from '../data/hooks';
import { Pagination } from '../components/Pagination';
import { money } from '../contracts/money';
import { Empty, ErrorNotice } from '../components/Dialog';
import { ClientDialog, NewOrderDialog, TemplateDialog, WorkerDialog } from '../features/forms';
import { StageDialog } from '../features/StageDialog';
import { Payments } from '../features/Payments';
import { Reports } from '../features/Reports';
import { ClientInsights } from '../features/ClientInsights';
import { Salaries } from '../features/Salaries';
import { SeparateCosts } from '../features/SeparateCosts';

const navigation: [string, string, LucideIcon][] = [
  ['dashboard', 'Dashboard', Grid2X2],
  ['orders', 'Orders', FolderOpen],
  ['clients', 'Clients', Users],
  ['stock', 'Stock', Package],
  ['reports', 'Reports', ChartNoAxesCombined],
  ['payments', 'Payments & balances', Wallet],
  ['costs', 'Costs', ChartNoAxesCombined],
  ['workers', 'Workers', Users],
  ['settings', 'Settings', Settings],
];
const stateName = {
  not_started: 'Not started',
  in_progress: 'In progress',
  completed: 'Completed',
};
const actionName: Record<string, string> = {
  order_created: 'Order created',
  stage_updated: 'Stage status updated',
  stage_entry_posted: 'Stage work or cost recorded',
  outside_service_added: 'Outside service added',
  payment_received: 'Customer payment recorded',
  payment_due_date_updated: 'Payment due date updated',
};
function Badge({ state }: { state: OrderStage['state'] }) {
  return (
    <span className={`badge ${state}`}>
      {state === 'completed' && <Check size={12} />} {stateName[state]}
    </span>
  );
}
function Progress({ done, total }: { done: number; total: number }) {
  return (
    <div className="progress">
      <div className="between">
        <span>
          {done} of {total} stages
        </span>
        <strong>{total ? Math.round((done / total) * 100) : 0}%</strong>
      </div>
      <div className="progress-track">
        <span style={{ width: `${total ? (done / total) * 100 : 0}%` }} />
      </div>
    </div>
  );
}
function Heading({
  title,
  subtitle,
  children,
}: {
  title: string;
  subtitle: string;
  children?: React.ReactNode;
}) {
  return (
    <div className="page-heading">
      <div>
        <h1>{title}</h1>
        <p>{subtitle}</p>
      </div>
      {children}
    </div>
  );
}
function Stat({
  label,
  value,
  icon: Icon,
  note,
}: {
  label: string;
  value: string | number;
  icon: LucideIcon;
  note: string;
}) {
  return (
    <div className="stat">
      <div className="between">
        <span>{label}</span>
        <div className="stat-icon">
          <Icon size={17} />
        </div>
      </div>
      <strong className="stat-value">{value}</strong>
      <small>{note}</small>
    </div>
  );
}

type Modal =
  | { kind: 'client'; value?: Client }
  | { kind: 'worker'; value?: Worker }
  | { kind: 'template'; value?: StageTemplate }
  | { kind: 'new' }
  | { kind: 'stage'; stageId: string };
export function App() {
  const [exportFilters, setExportFilters] = useState<ExportRequest | null>(null);
  return (
    <ExcelFilters.Provider value={setExportFilters}>
      <Workspace exportFilters={exportFilters} />
    </ExcelFilters.Provider>
  );
}
function Workspace({ exportFilters }: { exportFilters: ExportRequest | null }) {
  const bootstrap = useBootstrap();
  const orders = useOrders();
  const [page, setPage] = useState('dashboard');
  const [selected, setSelected] = useState<string | null>(null);
  const detail = useOrder(selected);
  const [modal, setModal] = useState<Modal | null>(null);
  const [search, setSearch] = useState('');
  const [orderPage, setOrderPage] = useState(1);
  const [orderPageSize, setOrderPageSize] = useState(25);
  const pagedOrders = useOrderPage(
    page === 'orders' ? search : '',
    orderPage,
    orderPageSize,
    'all',
  );
  const [actor, setActor] = useState('');
  const data = bootstrap.data;
  useEffect(() => {
    if (data && !actor) setActor(data.actors[0]?.id ?? '');
  }, [data, actor]);
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'n') {
        e.preventDefault();
        if (data) setModal({ kind: 'new' });
      }
      if ((e.metaKey || e.ctrlKey) && e.key === 'f') {
        e.preventDefault();
        document.getElementById('screen-search')?.focus();
      }
      if (
        (e.metaKey || e.ctrlKey) &&
        e.key === 's' &&
        document.querySelector('dialog[open] form')
      ) {
        e.preventDefault();
        (document.querySelector('dialog[open] form') as HTMLFormElement).requestSubmit();
      }
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [data]);
  const navigate = (next: string) => {
    setPage(next);
    setSearch('');
    setOrderPage(1);
  };
  const pick = (id: string) => {
    setSelected(id);
    navigate('orders');
  };
  const all = orders.data ?? [];
  const filtered = pagedOrders.data?.records ?? [];
  const open = all.filter((o) => o.lifecycle === 'open' && !o.archived);
  return (
    <div className="app">
      <aside className="sidebar">
        <button className="brand" onClick={() => navigate('dashboard')}>
          <span>
            <Box size={25} />
          </span>
          PackFlow<b>.</b>
        </button>
        <div className="workspace">
          <Warehouse size={21} />
          <div>
            <strong>Production workspace</strong>
            <small>LOCAL · KUWAIT</small>
          </div>
        </div>
        <p className="eyebrow">WORKSPACE</p>
        <nav aria-label="Main navigation">
          {navigation.map(([key, label, Icon], i) => (
            <div key={key}>
              {i === 5 && <p className="eyebrow nav-label">BUSINESS</p>}
              <button
                className={`nav-item ${page === key ? 'active' : ''}`}
                aria-label={label}
                aria-current={page === key ? 'page' : undefined}
                onClick={() => navigate(key)}
              >
                <Icon size={18} />
                <span>{label}</span>
                {key === 'orders' && <b>{data?.stats.active_orders ?? 0}</b>}
              </button>
            </div>
          ))}
        </nav>
        <div className="sidebar-bottom">
          <span className="avatar">M</span>
          <div>
            <strong>{data?.actors.find((a) => a.id === actor)?.display_name ?? 'Manager'}</strong>
            <small>Shared local workspace</small>
          </div>
        </div>
      </aside>
      <div className="shell">
        <header className="topbar">
          <div className="breadcrumb">
            <Warehouse size={16} />
            <span>Workspace</span>
            <ChevronRight size={14} />
            <strong>{navigation.find((n) => n[0] === page)?.[1]}</strong>
          </div>
          <div className="top-actions">
            {['clients', 'orders', 'payments', 'settings'].includes(page) && (
              <PhoneImportButton key={`import-${page}`} disabled={!data} />
            )}
            <ExcelExportButton
              key={page}
              disabled={!data}
              request={
                exportFilters?.page === page
                  ? exportFilters
                  : { page: page as ExportPage, search: page === 'orders' ? search : '' }
              }
            />
            <span className="local-badge">
              {isTauri() ? 'Offline desktop' : 'Development preview'}
            </span>
            {data && data.actors.length > 1 && (
              <select
                aria-label="Current actor"
                value={actor}
                onChange={(e) => setActor(e.target.value)}
              >
                {data.actors.map((a) => (
                  <option key={a.id} value={a.id}>
                    {a.display_name}
                  </option>
                ))}
              </select>
            )}
            <button
              className="btn primary"
              disabled={!data}
              onClick={() => setModal({ kind: 'new' })}
            >
              <Plus size={16} />
              New order
            </button>
          </div>
        </header>
        <main>
          <ErrorNotice error={bootstrap.error ?? orders.error} />
          {bootstrap.isPending && (
            <div className="panel loading" role="status">
              Connecting to your local workspace…
            </div>
          )}
          {bootstrap.isError && (
            <div className="panel">
              <Empty title="Local workspace is unavailable">
                Start the development core with <code>npm run dev:core</code> and reload. Existing
                records stay in your data directory.
              </Empty>
              <button
                className="btn"
                onClick={() => {
                  void bootstrap.refetch();
                  void orders.refetch();
                }}
              >
                Retry connection
              </button>
            </div>
          )}
          {data && (
            <>
              {page === 'dashboard' && (
                <>
                  <Heading
                    title="A clearer production day."
                    subtitle="From the first material purchase to the final box."
                  />
                  <div className="stats">
                    <Stat
                      label="Active orders"
                      value={data.stats.active_orders.toString().padStart(2, '0')}
                      icon={FolderOpen}
                      note="Open orders in your workspace"
                    />
                    <Stat
                      label="Stages completed"
                      value={data.stats.completed_stages}
                      icon={Check}
                      note="Recorded manual stage updates"
                    />
                    <Stat
                      label="Ordered boxes"
                      value={data.stats.ordered_boxes.toLocaleString()}
                      icon={Box}
                      note="Across your active orders"
                    />
                    <Stat
                      label="Production stages"
                      value={data.templates.filter((t) => !t.archived).length}
                      icon={Layers}
                      note="Editable templates for new orders"
                    />
                  </div>
                  <div className="dashboard-grid">
                    <section className="panel">
                      <div className="panel-heading">
                        <div>
                          <h2>Orders moving through production</h2>
                          <p>A clear view of the work ahead.</p>
                        </div>
                        <button className="btn subtle" onClick={() => navigate('orders')}>
                          View all
                          <ArrowRight size={15} />
                        </button>
                      </div>
                      {open.length ? (
                        <OrderTable orders={open.slice(0, 5)} onPick={pick} />
                      ) : (
                        <Empty title="Your next order starts here">
                          Add a client, then create an order with box types and production stages.
                        </Empty>
                      )}
                    </section>
                    <aside className="flow-card">
                      <span className="eyebrow">YOUR PRODUCTION FLOW</span>
                      <h2>
                        Every stage.
                        <br />
                        One connected order.
                      </h2>
                      <div className="flow-list">
                        {data.templates
                          .filter((t) => !t.archived)
                          .map((t, i) => (
                            <div key={t.id}>
                              <span>{i + 1}</span>
                              <strong>{t.name}</strong>
                            </div>
                          ))}
                      </div>
                      <button className="btn" onClick={() => navigate('settings')}>
                        Manage stages
                        <ArrowRight size={14} />
                      </button>
                    </aside>
                  </div>
                  <section className="panel welcome-panel">
                    <div className="welcome-icon">
                      <Activity size={26} />
                    </div>
                    <div>
                      <h2>Costs with a clear source.</h2>
                      <p>
                        Record materials, printing, machine work and labor inside each stage. The
                        customer quote stays independent.
                      </p>
                    </div>
                    <button className="btn" onClick={() => navigate('orders')}>
                      Open orders
                      <ArrowRight size={14} />
                    </button>
                  </section>
                </>
              )}
              {page === 'orders' && (
                <>
                  <Heading
                    title="Every order. One clear flow."
                    subtitle="Manage your box types, production stages and direct costs."
                  />
                  <div className="toolbar">
                    <label className="search">
                      <Search size={16} />
                      <input
                        id="screen-search"
                        value={search}
                        onChange={(e) => {
                          setSearch(e.target.value);
                          setOrderPage(1);
                        }}
                        maxLength={200}
                        placeholder="Search orders, clients or numbers…"
                        aria-label="Search orders"
                      />
                    </label>
                    <span className="muted">{pagedOrders.data?.total ?? 0} matching orders</span>
                  </div>
                  <div className={`orders-layout ${selected ? 'has-selection' : ''}`}>
                    <section>
                      <ErrorNotice error={pagedOrders.error} />
                      {pagedOrders.isPending || pagedOrders.isSearching ? (
                        <div className="panel loading" role="status">
                          Finding orders…
                        </div>
                      ) : filtered.length ? (
                        <div className="order-cards">
                          {filtered.map((o) => (
                            <button
                              className={`order-card ${o.id === selected ? 'selected' : ''}`}
                              key={o.id}
                              onClick={() => setSelected(o.id)}
                            >
                              <div className="between">
                                <span className="project-symbol">
                                  <Box size={20} />
                                </span>
                                <span className="order-number">{o.number}</span>
                              </div>
                              <h3>{o.name}</h3>
                              <p>{o.client_name}</p>
                              <div className="card-meta">
                                <strong>{o.ordered_boxes.toLocaleString()} boxes</strong>
                                <span>{o.due_date ? `Due ${o.due_date}` : 'Due date not set'}</span>
                              </div>
                              <Progress done={o.completed_stages} total={o.stage_count} />
                              <div className="card-footer">
                                <span>{money(o.total_due_fils)}</span>
                                <ArrowRight size={15} />
                              </div>
                            </button>
                          ))}
                        </div>
                      ) : (
                        <div className="panel">
                          <Empty title={search ? 'No matching orders' : 'No orders yet'}>
                            {search
                              ? 'Try another client, order name or number.'
                              : 'Create your first order to start production.'}
                          </Empty>
                        </div>
                      )}
                      {pagedOrders.data && (
                        <Pagination
                          label="Orders"
                          total={pagedOrders.data.total}
                          page={pagedOrders.data.page}
                          pageSize={orderPageSize}
                          onPage={setOrderPage}
                          onPageSize={(size) => {
                            setOrderPageSize(size);
                            setOrderPage(1);
                          }}
                          busy={pagedOrders.isFetching || pagedOrders.isSearching}
                        />
                      )}
                    </section>
                    {selected && (
                      <section className="panel detail">
                        <ErrorNotice error={detail.error} />
                        {detail.isPending ? (
                          <div className="loading">Loading order…</div>
                        ) : (
                          detail.data && (
                            <OrderPanel
                              detail={detail.data}
                              onStage={(id) => setModal({ kind: 'stage', stageId: id })}
                            />
                          )
                        )}
                      </section>
                    )}
                  </div>
                </>
              )}
              {page === 'clients' && (
                <>
                  <Heading
                    title="Built on good relationships."
                    subtitle="Your leading customers, connected orders and payment patterns."
                  >
                    <button className="btn primary" onClick={() => setModal({ kind: 'client' })}>
                      <Plus size={16} />
                      Add client
                    </button>
                  </Heading>
                  <ClientInsights
                    search={search}
                    onPayments={(id) => {
                      setSelected(id);
                      navigate('payments');
                    }}
                  />
                  <div className="panel">
                    <div className="panel-heading">
                      <h2>Client directory</h2>
                      <label className="search">
                        <Search size={16} />
                        <input
                          id="screen-search"
                          value={search}
                          onChange={(e) => setSearch(e.target.value)}
                          placeholder="Search clients…"
                          aria-label="Search clients"
                        />
                      </label>
                    </div>
                    {data.clients.length ? (
                      <div className="table-wrap">
                        <table>
                          <thead>
                            <tr>
                              <th>CLIENT</th>
                              <th>CONTACT</th>
                              <th>ORDERS</th>
                              <th>STATUS</th>
                              <th />
                            </tr>
                          </thead>
                          <tbody>
                            {data.clients
                              .filter((c) =>
                                `${c.name} ${c.contact_name}`
                                  .toLowerCase()
                                  .includes(search.toLowerCase()),
                              )
                              .map((c) => (
                                <tr key={c.id}>
                                  <td>
                                    <div className="identity">
                                      <span className="avatar company">
                                        {c.name.slice(0, 2).toUpperCase()}
                                      </span>
                                      <div>
                                        <strong>{c.name}</strong>
                                        <small>{c.address || 'No address recorded'}</small>
                                      </div>
                                    </div>
                                  </td>
                                  <td>
                                    <strong>{c.contact_name || '—'}</strong>
                                    <small>{c.email || c.phone || '—'}</small>
                                  </td>
                                  <td>{c.order_count}</td>
                                  <td>
                                    <span
                                      className={`badge ${c.archived ? 'not_started' : 'completed'}`}
                                    >
                                      {c.archived ? 'Archived' : 'Active'}
                                    </span>
                                  </td>
                                  <td>
                                    <button
                                      className="btn subtle"
                                      onClick={() => setModal({ kind: 'client', value: c })}
                                    >
                                      Edit
                                      <ArrowRight size={14} />
                                    </button>
                                  </td>
                                </tr>
                              ))}
                          </tbody>
                        </table>
                      </div>
                    ) : (
                      <Empty title="Make room for your first client">
                        Add company and contact details to keep every order connected.
                      </Empty>
                    )}
                  </div>
                </>
              )}
              {page === 'settings' && (
                <>
                  <Heading
                    title="A workspace that fits your process."
                    subtitle="Manage stage templates without rewriting existing order history."
                  />
                  <WorkspaceSafety />
                  <div className="settings-grid">
                    <section className="panel">
                      <div className="panel-heading">
                        <div>
                          <h2>Production stages</h2>
                          <p>Templates are copied when you create an order.</p>
                        </div>
                        <button
                          className="btn primary"
                          onClick={() => setModal({ kind: 'template' })}
                        >
                          <Plus size={15} />
                          Add stage
                        </button>
                      </div>
                      <Templates
                        bootstrap={data}
                        actor={actor}
                        onEdit={(t) => setModal({ kind: 'template', value: t })}
                      />
                    </section>
                    <section className="panel settings-summary">
                      <h2>Workspace defaults</h2>
                      <dl>
                        <dt>Currency</dt>
                        <dd>Kuwaiti dinar · KWD</dd>
                        <dt>Amount precision</dt>
                        <dd>Three decimal places</dd>
                        <dt>Business timezone</dt>
                        <dd>Asia/Kuwait</dd>
                        <dt>Storage</dt>
                        <dd>Local SQLite · schema {data.schema_version}</dd>
                        <dt>Customer documents</dt>
                        <dd>No invoices or tax/VAT</dd>
                      </dl>
                      <p className="form-note">
                        Excel import mapping and templates will be decided after reviewing your
                        source files. No import runs automatically.
                      </p>
                    </section>
                  </div>
                </>
              )}
              {page === 'workers' && (
                <>
                  <Heading
                    title="The people behind each stage."
                    subtitle="Worker profiles, stage effort and monthly salary calculations."
                  >
                    <button className="btn primary" onClick={() => setModal({ kind: 'worker' })}>
                      <Plus size={16} />
                      Add worker
                    </button>
                  </Heading>
                  <Salaries actor={actor} today={data.business_today} />
                  <div className="panel">
                    <div className="panel-heading">
                      <div>
                        <h2>Worker profiles</h2>
                        <p>Your team, connected to stage work and monthly salaries.</p>
                      </div>
                    </div>
                    {data.workers.length ? (
                      <div className="table-wrap">
                        <table>
                          <thead>
                            <tr>
                              <th>WORKER</th>
                              <th>ROLE</th>
                              <th>STATUS</th>
                              <th />
                            </tr>
                          </thead>
                          <tbody>
                            {data.workers.map((w) => (
                              <tr key={w.id}>
                                <td>
                                  <strong>{w.name}</strong>
                                </td>
                                <td>{w.role || '—'}</td>
                                <td>
                                  <span
                                    className={`badge ${w.active ? 'completed' : 'not_started'}`}
                                  >
                                    {w.active ? 'Active' : 'Inactive'}
                                  </span>
                                </td>
                                <td>
                                  <button
                                    className="btn subtle"
                                    onClick={() => setModal({ kind: 'worker', value: w })}
                                  >
                                    Edit
                                    <ArrowRight size={14} />
                                  </button>
                                </td>
                              </tr>
                            ))}
                          </tbody>
                        </table>
                      </div>
                    ) : (
                      <Empty title="Add your production team">
                        Profiles can be selected when recording stage work and time.
                      </Empty>
                    )}
                  </div>
                </>
              )}
              {page === 'costs' && (
                <>
                  <Heading
                    title="Keep every cost in view."
                    subtitle="Separate workspace expenses and costs connected to your orders."
                  />
                  <SeparateCosts actor={actor} today={data.business_today} />
                  <div className="section-heading">
                    <h2>Order costs</h2>
                  </div>
                  <label className="standalone-select">
                    Order
                    <select
                      value={selected ?? ''}
                      onChange={(e) => setSelected(e.target.value || null)}
                    >
                      <option value="">Choose an order</option>
                      {all.map((o) => (
                        <option key={o.id} value={o.id}>
                          {o.number} · {o.name}
                        </option>
                      ))}
                    </select>
                  </label>
                  <ErrorNotice error={detail.error} />
                  {detail.data ? (
                    <Costs detail={detail.data} />
                  ) : (
                    <section className="panel">
                      <Empty title="Choose an order to review its costs">
                        Post expenses through each stage’s management dialog.
                      </Empty>
                    </section>
                  )}
                </>
              )}
              {page === 'stock' && (
                <>
                  <Heading title="Stock" subtitle="Reserved for a future release." />
                  <div className="panel">
                    <Empty title="Stock management is deferred">
                      Materials are recorded as direct purchase expenses inside order stages.
                    </Empty>
                  </div>
                </>
              )}
              {page === 'payments' && (
                <Payments
                  orders={all}
                  selected={selected}
                  onSelect={setSelected}
                  actor={actor}
                  today={data.business_today}
                  onOpenOrder={pick}
                />
              )}
              {page === 'reports' && <Reports today={data.business_today} onPick={pick} />}
            </>
          )}
          <footer>
            <span>PackFlow · A little more order. A lot more flow.</span>
            <span className="footer-state">
              <i /> {bootstrap.isSuccess ? 'Local database connected' : 'Connecting'}
            </span>
          </footer>
        </main>
      </div>
      {data && actor && modal?.kind === 'client' && (
        <ClientDialog actor={actor} client={modal.value} onClose={() => setModal(null)} />
      )}
      {data && actor && modal?.kind === 'worker' && (
        <WorkerDialog actor={actor} worker={modal.value} onClose={() => setModal(null)} />
      )}
      {data && actor && modal?.kind === 'template' && (
        <TemplateDialog actor={actor} template={modal.value} onClose={() => setModal(null)} />
      )}
      {data && actor && modal?.kind === 'new' && (
        <NewOrderDialog
          bootstrap={data}
          orders={all}
          actor={actor}
          onClose={() => setModal(null)}
          onCreated={pick}
        />
      )}
      {data && actor && modal?.kind === 'stage' && detail.data && (
        <StageDialog
          stage={detail.data.stages.find((s) => s.id === modal.stageId)!}
          detail={detail.data}
          bootstrap={data}
          actor={actor}
          onClose={() => setModal(null)}
        />
      )}
    </div>
  );
}
function OrderTable({ orders, onPick }: { orders: OrderSummary[]; onPick: (id: string) => void }) {
  return (
    <div className="table-wrap">
      <table>
        <thead>
          <tr>
            <th>ORDER</th>
            <th>BOXES</th>
            <th>PROGRESS</th>
            <th>DUE DATE</th>
          </tr>
        </thead>
        <tbody>
          {orders.map((o) => (
            <tr key={o.id}>
              <td>
                <button className="table-link" onClick={() => onPick(o.id)}>
                  <span className="project-symbol">
                    <Box size={18} />
                  </span>
                  <span>
                    <strong>{o.name}</strong>
                    <small>
                      {o.number} · {o.client_name}
                    </small>
                  </span>
                </button>
              </td>
              <td>{o.ordered_boxes.toLocaleString()}</td>
              <td>
                <Progress done={o.completed_stages} total={o.stage_count} />
              </td>
              <td>{o.due_date || 'Not set'}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
function OrderPanel({ detail, onStage }: { detail: OrderDetail; onStage: (id: string) => void }) {
  const stages = detail.stages.filter((s) => !s.retired);
  return (
    <>
      <div className="detail-header">
        <span className="eyebrow">{detail.order.number}</span>
        <h2>{detail.order.name}</h2>
        <p>
          {detail.order.client_name} ·{' '}
          {detail.order.due_date ? `Due ${detail.order.due_date}` : 'Due date not set'}
        </p>
        {detail.mobile_source && (
          <p className="form-note">
            From phone {detail.mobile_source.number} · phone revision{' '}
            {detail.mobile_source.source_version}
          </p>
        )}
        <div className="mini-stats">
          <div>
            <small>Customer total</small>
            <strong>{money(detail.order.total_due_fils)}</strong>
          </div>
          <div>
            <small>Recorded costs</small>
            <strong>{money(detail.cost_fils)}</strong>
          </div>
        </div>
        <Progress
          done={stages.filter((s) => s.state === 'completed').length}
          total={stages.length}
        />
      </div>
      <div className="detail-section">
        <h3>Box types</h3>
        {detail.items.map((i) => (
          <div className="item-row" key={i.id}>
            <div>
              <strong>{i.description}</strong>
              <small>{i.dimensions_text || 'No dimensions recorded'}</small>
            </div>
            <strong>{i.ordered_boxes.toLocaleString()} boxes</strong>
          </div>
        ))}
      </div>
      <div className="detail-section">
        <div className="section-heading">
          <h3>Production stages</h3>
          <span className="muted">Manage each step</span>
        </div>
        {stages.map((s) => (
          <button className="stage-row" key={s.id} onClick={() => onStage(s.id)}>
            <span className={`stage-circle ${s.state === 'completed' ? 'done' : ''}`}>
              {s.state === 'completed' ? <Check size={14} /> : s.position + 1}
            </span>
            <span>
              <strong>{s.name_snapshot}</strong>
              <small>{money(s.cost_fils)} recorded</small>
              {detail.stage_prices.some((p) => p.stage_id === s.id) && (
                <small>
                  Customer price:{' '}
                  {money(detail.stage_prices.find((p) => p.stage_id === s.id)!.total_fils)}
                </small>
              )}
            </span>
            <Badge state={s.state} />
            <ChevronRight size={14} />
          </button>
        ))}
      </div>
      {detail.order.notes && (
        <div className="detail-section">
          <h3>Order notes</h3>
          <p className="notes">{detail.order.notes}</p>
        </div>
      )}
      <div className="detail-section">
        <h3>History</h3>
        <div className="history-list">
          {detail.history.slice(0, 5).map((h) => (
            <div key={h.id} className="history-row">
              <span className="history-dot" />
              <div>
                <strong>{actionName[h.action] ?? h.action.replaceAll('_', ' ')}</strong>
                <small>
                  {h.actor_name} · {h.business_date}
                </small>
              </div>
            </div>
          ))}
        </div>
      </div>
    </>
  );
}
function Templates({
  bootstrap,
  actor,
  onEdit,
}: {
  bootstrap: Bootstrap;
  actor: string;
  onEdit: (t: StageTemplate) => void;
}) {
  const write = useWrite(actor);
  const templates = bootstrap.templates;
  const move = (index: number, direction: number) => {
    const next = [...templates];
    [next[index], next[index + direction]] = [next[index + direction], next[index]];
    write.mutate({
      kind: 'reorder_templates',
      ids: next.map((t) => t.id),
      versions: next.map((t) => t.version),
    });
  };
  return (
    <div className="template-list">
      <ErrorNotice error={write.error} />
      {templates.map((t, i) => (
        <div className="template-row" key={t.id}>
          <span className="stage-circle">{i + 1}</span>
          <div className="template-name">
            <strong>{t.name}</strong>
            <small>
              {t.archived ? 'Archived template' : t.default_notes || 'No default instructions'}
            </small>
          </div>
          <button
            className="icon-button"
            aria-label={`Move ${t.name} up`}
            disabled={i === 0 || write.isPending}
            onClick={() => move(i, -1)}
          >
            <ArrowUp size={15} />
          </button>
          <button
            className="icon-button"
            aria-label={`Move ${t.name} down`}
            disabled={i === templates.length - 1 || write.isPending}
            onClick={() => move(i, 1)}
          >
            <ArrowDown size={15} />
          </button>
          <button className="btn subtle" onClick={() => onEdit(t)}>
            Edit
          </button>
          <button
            className="icon-button"
            aria-label={`Delete ${t.name}`}
            disabled={write.isPending}
            onClick={() => {
              if (
                window.confirm(
                  `Delete unused template “${t.name}”? Used templates must be archived.`,
                )
              )
                write.mutate({ kind: 'delete_template', id: t.id, expected_version: t.version });
            }}
          >
            <Trash2 size={15} />
          </button>
        </div>
      ))}
    </div>
  );
}
function Costs({ detail }: { detail: OrderDetail }) {
  const quote = detail.order.total_due_fils;
  const margin = quote === null ? null : (BigInt(quote) - BigInt(detail.cost_fils)).toString();
  return (
    <>
      <div className="stats three-stats">
        <Stat
          label="Customer total"
          value={money(quote)}
          icon={Wallet}
          note="Base quote plus outside-service charges"
        />
        <Stat
          label="Recorded order cost"
          value={money(detail.cost_fils)}
          icon={ChartNoAxesCombined}
          note="Stage and outside-service expenses"
        />
        <Stat
          label="Estimated margin"
          value={margin === null ? 'Unknown' : money(margin)}
          icon={Activity}
          note="Customer total less recorded costs"
        />
      </div>
      <div className="panel">
        {detail.costs.length ? (
          <div className="table-wrap">
            <table>
              <thead>
                <tr>
                  <th>EXPENSE</th>
                  <th>STAGE</th>
                  <th>CATEGORY</th>
                  <th>DATE</th>
                  <th>AMOUNT</th>
                </tr>
              </thead>
              <tbody>
                {detail.costs.map((c) => (
                  <tr key={c.id}>
                    <td>
                      <strong>{c.description}</strong>
                      <small>{c.supplier_text || '—'}</small>
                    </td>
                    <td>
                      {detail.stages.find((s) => s.id === c.stage_id)?.name_snapshot ??
                        'Whole order'}
                    </td>
                    <td>
                      <span className="category">{c.category}</span>
                    </td>
                    <td>{c.business_date}</td>
                    <td>{money(c.amount_fils)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <Empty title="No costs recorded for this order">
            Open a production stage and use its Costs tab to record a direct expense.
          </Empty>
        )}
      </div>
    </>
  );
}
