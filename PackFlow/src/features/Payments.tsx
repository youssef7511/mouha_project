import { Correction } from './Correction';
import { useExcelFilters } from './ExcelExport';
import { useState } from 'react';
import { ArrowRight, Plus } from 'lucide-react';
import type { OrderDetail, OrderSummary } from '../contracts';
import { money, parseScaled } from '../contracts/money';
import { useOrder, useOrderPage, useWrite } from '../data/hooks';
import { ListSearch, Pagination } from '../components/Pagination';
import { Dialog, Empty, ErrorNotice } from '../components/Dialog';
import { StageQuoteBreakdown } from './StageQuoteBreakdown';

export function paymentLabel(total: string | null, paid: string): string {
  if (total === null) return 'Not priced';
  const balance = BigInt(total) - BigInt(paid);
  if (BigInt(total) === 0n && BigInt(paid) === 0n) return 'No payment due';
  if (balance < 0n) return 'Customer credit';
  if (balance === 0n) return 'Paid';
  return BigInt(paid) === 0n ? 'Unpaid' : 'Partially paid';
}
function AmountCard({ label, amount }: { label: string; amount: string | null }) {
  return (
    <div className="finance-stat">
      <span>{label}</span>
      <strong>{money(amount)}</strong>
    </div>
  );
}
export function Payments({
  orders,
  selected,
  onSelect,
  actor,
  today,
  onOpenOrder,
}: {
  orders: OrderSummary[];
  selected: string | null;
  onSelect: (id: string | null) => void;
  actor: string;
  today: string;
  onOpenOrder: (id: string) => void;
}) {
  const detail = useOrder(selected);
  const [balanceSearch, setBalanceSearch] = useState('');
  useExcelFilters({ page: 'payments', search: balanceSearch });
  const [balancePage, setBalancePage] = useState(1);
  const [balancePageSize, setBalancePageSize] = useState(25);
  const balances = useOrderPage(balanceSearch, balancePage, balancePageSize, 'active');
  const [modal, setModal] = useState<'service' | 'payment' | 'deadline' | null>(null);
  const active = orders.filter((o) => !o.archived && o.lifecycle !== 'cancelled');
  const data = detail.data;
  const options: Pick<OrderSummary, 'id' | 'number' | 'name' | 'client_name'>[] = [...active];
  if (data && !options.some((o) => o.id === data.order.id)) options.push(data.order);
  return (
    <>
      <div className="page-heading">
        <div>
          <h1>Every payment, connected.</h1>
          <p>Customer balances and outside services, attached to the right order.</p>
        </div>
      </div>
      <label className="standalone-select order-picker">
        Order
        <select value={selected ?? ''} onChange={(e) => onSelect(e.target.value || null)}>
          <option value="">Choose an existing order</option>
          {options.map((o) => (
            <option key={o.id} value={o.id}>
              {o.number} · {o.name} · {o.client_name}
            </option>
          ))}
        </select>
      </label>
      <ErrorNotice error={detail.error} />
      {selected && detail.isPending && (
        <div className="panel loading" role="status">
          Loading balance…
        </div>
      )}
      {data && (
        <>
          <section className="panel payment-summary">
            <div className="panel-heading">
              <div>
                <h2>{data.order.name}</h2>
                <p>
                  {data.order.number} · {data.order.client_name}
                </p>
              </div>
              <button className="btn subtle" onClick={() => onOpenOrder(data.order.id)}>
                Open order <ArrowRight size={14} />
              </button>
            </div>
            <div className="finance-stats">
              <AmountCard label="Base customer quote" amount={data.order.quote_fils} />
              <AmountCard label="Outside-service charges" amount={data.order.service_charge_fils} />
              <AmountCard label="Total order amount" amount={data.order.total_due_fils} />
              <AmountCard label="Net received" amount={data.order.paid_fils} />
            </div>
            <StageQuoteBreakdown detail={data} />
            <div className="payment-deadline between">
              <div>
                <small>Agreed payment due date</small>
                <strong>{data.order.payment_due_date ?? 'Not set'}</strong>
                {data.order.payment_due_date &&
                  data.order.payment_due_date < today &&
                  data.order.balance_fils !== null &&
                  BigInt(data.order.balance_fils) > 0n && (
                    <span className="badge in_progress">Payment overdue</span>
                  )}
              </div>
              <button
                className="btn"
                disabled={!actor || !!data.order.archived || data.order.lifecycle === 'cancelled'}
                onClick={() => setModal('deadline')}
              >
                {data.order.payment_due_date ? 'Change payment due date' : 'Set payment due date'}
              </button>
            </div>
            <div className="payment-balance between">
              <div>
                <span className="badge">
                  {paymentLabel(data.order.total_due_fils, data.order.paid_fils)}
                </span>
                <p>
                  {data.order.balance_fils !== null && BigInt(data.order.balance_fils) < 0n
                    ? 'Customer credit'
                    : 'Remaining balance'}{' '}
                  <strong>
                    {money(
                      data.order.balance_fils === null
                        ? null
                        : (BigInt(data.order.balance_fils) < 0n
                            ? -BigInt(data.order.balance_fils)
                            : BigInt(data.order.balance_fils)
                          ).toString(),
                    )}
                  </strong>
                </p>
              </div>
              <button
                className="btn primary"
                disabled={
                  !actor || data.order.archived !== 0 || data.order.lifecycle === 'cancelled'
                }
                onClick={() => setModal('payment')}
              >
                <Plus size={15} />
                Record payment
              </button>
            </div>
          </section>
          <section className="panel">
            <div className="panel-heading">
              <div>
                <h2>Outside services</h2>
                <p>Customer charges add to the total. Supplier costs flow into Costs.</p>
              </div>
              <button
                className="btn"
                disabled={!actor || data.order.lifecycle !== 'open' || !!data.order.archived}
                onClick={() => setModal('service')}
              >
                <Plus size={15} />
                Add outside service
              </button>
            </div>
            {data.services.length ? (
              <div className="table-wrap">
                <table>
                  <thead>
                    <tr>
                      <th>SERVICE</th>
                      <th>DATE</th>
                      <th>CUSTOMER CHARGE</th>
                      <th>SUPPLIER COST</th>
                    </tr>
                  </thead>
                  <tbody>
                    {data.services.map((s) => (
                      <tr key={s.id}>
                        <td>
                          <strong>{s.description}</strong>
                          <small>{s.reason}</small>
                          <small>
                            {s.supplier_text || 'No supplier specified'}
                            {s.reference && ` · ${s.reference}`}
                          </small>
                        </td>
                        <td>{s.business_date}</td>
                        <td>{money(s.charge_fils)}</td>
                        <td>
                          {money(s.supplier_cost_fils)}
                          <Correction
                            actor={actor}
                            id={s.id}
                            type="service"
                            today={today}
                            reversed={s.reversed}
                            label={s.description}
                          />
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            ) : (
              <Empty title="No outside services recorded">
                Add a service with its customer charge and supplier cost.
              </Empty>
            )}
          </section>
          <section className="panel">
            <div className="panel-heading">
              <div>
                <h2>Payment history</h2>
                <p>
                  Dated receipts for this order. Payment records leave stage progress unchanged.
                </p>
              </div>
            </div>
            {data.payments.length ? (
              <div className="table-wrap">
                <table>
                  <thead>
                    <tr>
                      <th>DATE</th>
                      <th>METHOD</th>
                      <th>REFERENCE / NOTES</th>
                      <th>AMOUNT</th>
                    </tr>
                  </thead>
                  <tbody>
                    {data.payments.map((p) => (
                      <tr key={p.id}>
                        <td>{p.business_date}</td>
                        <td>{p.method}</td>
                        <td>
                          {p.reference || '—'}
                          <small>{p.notes}</small>
                          <small>{p.reason}</small>
                          {p.kind === 'receipt' && (
                            <Correction
                              actor={actor}
                              id={p.id}
                              type="payment"
                              today={today}
                              reversed={p.reversed}
                              label="receipt"
                            />
                          )}
                        </td>
                        <td>
                          {p.kind === 'refund' ? '−' : ''}
                          {money(p.amount_fils)}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            ) : (
              <Empty title="No payments yet">
                Record a customer payment to update the order’s balance.
              </Empty>
            )}
          </section>
        </>
      )}
      <section className="panel">
        <div className="panel-heading">
          <div>
            <h2>Order balances</h2>
            <p>
              Search all current orders by name, client or number, then select one to manage its
              payments.
            </p>
          </div>
        </div>
        <div className="list-toolbar">
          <ListSearch
            label="Search order balances"
            value={balanceSearch}
            onChange={(value) => {
              setBalanceSearch(value);
              setBalancePage(1);
            }}
          />
        </div>
        <ErrorNotice error={balances.error} />
        {balances.isPending || balances.isSearching ? (
          <div className="loading" role="status">
            Finding order balances…
          </div>
        ) : balances.data?.records.length ? (
          <div className="table-wrap">
            <table>
              <thead>
                <tr>
                  <th>ORDER</th>
                  <th>TOTAL</th>
                  <th>RECEIVED</th>
                  <th>BALANCE</th>
                  <th>PAYMENT STATUS</th>
                </tr>
              </thead>
              <tbody>
                {balances.data.records.map((o) => (
                  <tr key={o.id}>
                    <td>
                      <button className="table-link" onClick={() => onSelect(o.id)}>
                        <span>
                          <strong>{o.name}</strong>
                          <small>
                            {o.number} · {o.client_name}
                          </small>
                        </span>
                      </button>
                    </td>
                    <td>{money(o.total_due_fils)}</td>
                    <td>{money(o.paid_fils)}</td>
                    <td>{money(o.balance_fils)}</td>
                    <td>
                      <span className="badge">{paymentLabel(o.total_due_fils, o.paid_fils)}</span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <Empty title={balanceSearch ? 'No matching order balances' : 'No orders yet'}>
            {balanceSearch
              ? 'Try another order name, client or number.'
              : 'Create an order to connect customer payments and services.'}
          </Empty>
        )}
        {balances.data && (
          <Pagination
            label="Order balances"
            total={balances.data.total}
            page={balances.data.page}
            pageSize={balancePageSize}
            onPage={setBalancePage}
            onPageSize={(size) => {
              setBalancePageSize(size);
              setBalancePage(1);
            }}
            busy={balances.isFetching || balances.isSearching}
          />
        )}
      </section>
      {data && modal === 'deadline' && (
        <PaymentDeadlineDialog
          key={data.order.id}
          detail={data}
          actor={actor}
          onClose={() => setModal(null)}
        />
      )}
      {data && modal && modal !== 'deadline' && (
        <FinanceDialog
          key={`${data.order.id}-${modal}`}
          mode={modal}
          detail={data}
          actor={actor}
          today={today}
          onClose={() => setModal(null)}
        />
      )}
    </>
  );
}

function PaymentDeadlineDialog({
  detail,
  actor,
  onClose,
}: {
  detail: OrderDetail;
  actor: string;
  onClose: () => void;
}) {
  const initial = detail.order.payment_due_date ?? '';
  const [due, setDue] = useState(initial);
  const write = useWrite(actor);
  const close = () => {
    if (
      !write.isPending &&
      (due === initial || window.confirm('Discard payment due date changes?'))
    )
      onClose();
  };
  return (
    <Dialog
      title="Payment due date"
      subtitle={`${detail.order.number} · ${detail.order.name}`}
      onClose={close}
    >
      <form
        onSubmit={(e) => {
          e.preventDefault();
          if (!write.isPending)
            write.mutate(
              {
                kind: 'set_payment_due_date',
                order_id: detail.order.id,
                expected_version: detail.order.version,
                payment_due_date: due || null,
              },
              { onSuccess: onClose },
            );
        }}
      >
        <label>
          Agreed payment due date
          <input autoFocus type="date" value={due} onChange={(e) => setDue(e.target.value)} />
        </label>
        <p className="form-note">
          Use the date agreed with your client. It is separate from production delivery. Leave it
          blank if no payment deadline has been agreed.
        </p>
        <ErrorNotice error={write.error} />
        <div className="dialog-actions">
          <button type="button" className="btn" onClick={close}>
            Cancel
          </button>
          <button className="btn primary" disabled={write.isPending}>
            Save payment due date
          </button>
        </div>
      </form>
    </Dialog>
  );
}

function FinanceDialog({
  mode,
  detail,
  actor,
  today,
  onClose,
}: {
  mode: 'service' | 'payment';
  detail: OrderDetail;
  actor: string;
  today: string;
  onClose: () => void;
}) {
  const [description, setDescription] = useState('');
  const [charge, setCharge] = useState('');
  const [cost, setCost] = useState('');
  const [supplier, setSupplier] = useState('');
  const [date, setDate] = useState(today);
  const [method, setMethod] = useState('cash');
  const [reference, setReference] = useState('');
  const [notes, setNotes] = useState('');
  const [error, setError] = useState<unknown>(null);
  const write = useWrite(actor);
  const dirty = !!(
    description ||
    charge ||
    cost ||
    supplier ||
    reference ||
    notes ||
    date !== today ||
    method !== 'cash'
  );
  const close = () => {
    if (!write.isPending && (!dirty || window.confirm('Discard unsaved changes?'))) onClose();
  };
  const service = mode === 'service';
  let total: string | null = detail.order.total_due_fils;
  try {
    if (service && total !== null && charge.trim())
      total = (BigInt(total) + BigInt(parseScaled(charge))).toString();
  } catch {
    total = null;
  }
  return (
    <Dialog
      title={service ? 'Add an outside service' : 'Record a customer payment'}
      subtitle={`${detail.order.number} · ${detail.order.name}`}
      onClose={close}
    >
      <form
        onSubmit={(e) => {
          e.preventDefault();
          if (write.isPending) return;
          try {
            setError(null);
            if (service)
              write.mutate(
                {
                  kind: 'add_outside_service',
                  order_id: detail.order.id,
                  expected_version: detail.order.version,
                  description,
                  supplier_text: supplier,
                  charge_fils: parseScaled(charge),
                  supplier_cost_fils: parseScaled(cost),
                  business_date: date,
                  reference,
                },
                { onSuccess: onClose },
              );
            else
              write.mutate(
                {
                  kind: 'receive_payment',
                  order_id: detail.order.id,
                  amount_fils: parseScaled(charge),
                  business_date: date,
                  method,
                  reference,
                  notes,
                },
                { onSuccess: onClose },
              );
          } catch (error) {
            setError(error);
          }
        }}
      >
        {service && (
          <>
            <label>
              Outside service description
              <input
                autoFocus
                required
                maxLength={1000}
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                placeholder="e.g. Custom gift-box finishing"
              />
            </label>
            <label>
              Supplier
              <input
                maxLength={200}
                value={supplier}
                onChange={(e) => setSupplier(e.target.value)}
              />
            </label>
          </>
        )}
        <div className="field-row">
          <label>
            {service ? 'Charge to customer · KWD' : 'Payment amount · KWD'}
            <input
              autoFocus={!service}
              inputMode="decimal"
              required
              placeholder="0.000"
              value={charge}
              onChange={(e) => setCharge(e.target.value)}
            />
          </label>
          {service ? (
            <label>
              Supplier cost · KWD
              <input
                inputMode="decimal"
                required
                placeholder="0.000"
                value={cost}
                onChange={(e) => setCost(e.target.value)}
              />
            </label>
          ) : (
            <label>
              Payment method
              <select value={method} onChange={(e) => setMethod(e.target.value)}>
                <option value="cash">Cash</option>
                <option value="bank">Bank transfer</option>
                <option value="card">Card</option>
                <option value="other">Other</option>
              </select>
            </label>
          )}
        </div>
        <div className="field-row">
          <label>
            Business date
            <input type="date" required value={date} onChange={(e) => setDate(e.target.value)} />
          </label>
          <label>
            Reference
            <input
              maxLength={200}
              value={reference}
              onChange={(e) => setReference(e.target.value)}
            />
          </label>
        </div>
        {!service && (
          <label>
            Notes
            <textarea maxLength={2000} value={notes} onChange={(e) => setNotes(e.target.value)} />
          </label>
        )}
        {service ? (
          <>
            <div className="quote-summary">
              <span>New order total</span>
              <strong>{money(total)}</strong>
            </div>
            <p className="form-note">
              The customer charge and one supplier expense are saved together. An unpriced base
              quote keeps the total unknown.
            </p>
          </>
        ) : (
          <p className="form-note">
            Payments above the order amount create customer credit. An unpriced order keeps its
            balance unknown.
          </p>
        )}
        <ErrorNotice error={error ?? write.error} />
        <div className="dialog-actions">
          <button type="button" className="btn" onClick={close}>
            Cancel
          </button>
          <button className="btn primary" disabled={write.isPending}>
            {write.isPending ? 'Saving…' : service ? 'Save outside service' : 'Save payment'}
          </button>
        </div>
      </form>
    </Dialog>
  );
}
