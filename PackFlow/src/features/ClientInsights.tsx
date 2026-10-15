import { useEffect, useRef, useState } from 'react';
import { ArrowRight, Trophy } from 'lucide-react';
import { useClientInsights } from '../data/hooks';
import type { ClientInsight } from '../contracts';
import { money } from '../contracts/money';
import { Empty, ErrorNotice } from '../components/Dialog';
import { paymentLabel } from './Payments';

type Ranking = 'orders' | 'received' | 'value';
const sum = (values: string[]) =>
  values.reduce((total, value) => total + BigInt(value), 0n).toString();
const compareMoney = (left: string, right: string) =>
  BigInt(left) > BigInt(right) ? -1 : BigInt(left) < BigInt(right) ? 1 : 0;
function rankClients(clients: ClientInsight[], ranking: Ranking) {
  return [...clients]
    .filter((client) => client.order_count > 0)
    .sort((a, b) => {
      const primary =
        ranking === 'orders'
          ? b.order_count - a.order_count
          : compareMoney(
              ranking === 'received' ? a.net_received_fils : a.quoted_fils,
              ranking === 'received' ? b.net_received_fils : b.quoted_fils,
            );
      return (
        primary ||
        b.order_count - a.order_count ||
        compareMoney(a.net_received_fils, b.net_received_fils) ||
        a.name.localeCompare(b.name) ||
        a.id.localeCompare(b.id)
      );
    });
}
export function ClientInsights({
  search,
  onPayments,
}: {
  search: string;
  onPayments: (id: string) => void;
}) {
  const query = useClientInsights();
  const [ranking, setRanking] = useState<Ranking>('orders');
  const [reviewSort, setReviewSort] = useState('overdue');
  const [includeArchived, setIncludeArchived] = useState(false);
  const [selected, setSelected] = useState<string | null>(null);
  const historyRef = useRef<HTMLElement>(null);
  useEffect(() => {
    if (selected) historyRef.current?.scrollIntoView({ behavior: 'smooth', block: 'start' });
  }, [selected]);
  const data = query.data;
  const clients = (data?.clients ?? []).filter(
    (client) =>
      (includeArchived || !client.archived) &&
      `${client.name} ${client.contact_name}`.toLowerCase().includes(search.toLowerCase()),
  );
  const leaders = rankClients(clients, ranking).slice(0, 3);
  const reviewed = [...clients].sort((a, b) => {
    const primary =
      reviewSort === 'partial'
        ? b.partial_orders - a.partial_orders
        : reviewSort === 'late'
          ? b.late_receipt_orders - a.late_receipt_orders
          : compareMoney(a.overdue_fils, b.overdue_fils);
    return (
      primary ||
      compareMoney(a.outstanding_fils, b.outstanding_fils) ||
      a.name.localeCompare(b.name)
    );
  });
  const client = clients.find((entry) => entry.id === selected);
  const orders = client
    ? (data?.orders.filter((order) => order.client_id === client.id) ?? [])
    : [];
  const payments = client
    ? (data?.payments.filter((payment) => payment.client_id === client.id) ?? [])
    : [];
  return (
    <div className="client-insights">
      <ErrorNotice error={query.error} />
      {query.isPending && (
        <div className="panel loading" role="status">
          Preparing client statistics…
        </div>
      )}
      {data && (
        <>
          <div className="stats client-summary">
            <div className="stat report-metric">
              <span>Clients with orders</span>
              <strong className="stat-value">
                {clients.filter((c) => c.order_count > 0).length}
              </strong>
              <small>Across the clients shown below</small>
            </div>
            <div className="stat report-metric">
              <span>Orders placed</span>
              <strong className="stat-value">
                {clients.reduce((n, c) => n + c.order_count, 0)}
              </strong>
              <small>All time · cancelled orders excluded</small>
            </div>
            <div className="stat report-metric">
              <span>Outstanding balances</span>
              <strong className="stat-value">
                {money(sum(clients.map((c) => c.outstanding_fils)))}
              </strong>
              <small>Unpriced orders excluded · credits stay separate</small>
            </div>
            <div className="stat report-metric">
              <span>Overdue balances</span>
              <strong className="stat-value">
                {money(sum(clients.map((c) => c.overdue_fils)))}
              </strong>
              <small>Unpaid amounts past agreed payment deadlines</small>
            </div>
          </div>
          <section className="panel client-leaderboard">
            <div className="panel-heading">
              <div>
                <h2>Top clients</h2>
                <p>Rank by order volume, customer money received, or current order value.</p>
              </div>
              <label className="standalone-select">
                Rank clients by
                <select value={ranking} onChange={(e) => setRanking(e.target.value as Ranking)}>
                  <option value="orders">Number of orders</option>
                  <option value="received">Net payments received</option>
                  <option value="value">Order value</option>
                </select>
              </label>
            </div>
            {leaders.length ? (
              <div className="client-leaders">
                {leaders.map((leader, index) => (
                  <button
                    className="client-leader"
                    key={leader.id}
                    onClick={() => setSelected(leader.id)}
                  >
                    <div className="between">
                      <span className="client-rank">
                        <Trophy size={15} />#{index + 1}
                      </span>
                      {leader.overdue_orders > 0 && (
                        <span className="badge in_progress">{leader.overdue_orders} overdue</span>
                      )}
                    </div>
                    <h3>{leader.name}</h3>
                    <p>
                      {leader.order_count} orders · {money(leader.quoted_fils)} priced value
                    </p>
                    <div className="client-leader-amount">
                      <small>Net payments received</small>
                      <strong>{money(leader.net_received_fils)}</strong>
                    </div>
                    <div className="client-payment-track">
                      <span
                        style={{
                          width: `${leader.priced_orders ? (leader.settled_orders / leader.priced_orders) * 100 : 0}%`,
                        }}
                      />
                    </div>
                    <small>
                      {leader.priced_orders
                        ? `${leader.settled_orders} of ${leader.priced_orders} payable orders settled`
                        : 'No payable orders to assess'}
                    </small>
                  </button>
                ))}
              </div>
            ) : (
              <Empty title="Client rankings start with an order">
                Create orders and record customer payments to see your leading clients.
              </Empty>
            )}
          </section>
          <section className="panel client-review">
            <div className="panel-heading">
              <div>
                <h2>Payment patterns & follow-up</h2>
                <p>Live balances and receipt history as of {data.business_today}.</p>
              </div>
              <div className="client-review-controls">
                <label className="standalone-select">
                  Prioritize
                  <select value={reviewSort} onChange={(e) => setReviewSort(e.target.value)}>
                    <option value="overdue">Largest overdue balance</option>
                    <option value="partial">Most partially paid orders</option>
                    <option value="late">Most orders with receipts after deadline</option>
                  </select>
                </label>
                <label className="client-archive-toggle">
                  <input
                    type="checkbox"
                    checked={includeArchived}
                    onChange={(e) => setIncludeArchived(e.target.checked)}
                  />
                  Include archived clients
                </label>
              </div>
            </div>
            <p className="client-definition">
              Partial payments can be agreed deposits or instalments. Overdue means a positive
              balance after the separate payment due date. Receipts after deadline count orders with
              a receipt dated after that agreed date; they can include orders now fully paid.
            </p>
            {clients.length ? (
              <div className="table-wrap">
                <table>
                  <thead>
                    <tr>
                      <th>CLIENT</th>
                      <th>ORDERS / RECEIVED</th>
                      <th>PARTIALLY PAID</th>
                      <th>OVERDUE</th>
                      <th>RECEIPTS AFTER DEADLINE</th>
                      <th />
                    </tr>
                  </thead>
                  <tbody>
                    {reviewed.map((entry) => (
                      <tr key={entry.id}>
                        <td>
                          <strong>{entry.name}</strong>
                          <small>
                            {entry.archived
                              ? 'Archived client'
                              : `${entry.settled_orders} settled · ${entry.unpriced_orders} unpriced`}
                          </small>
                        </td>
                        <td>
                          {entry.order_count} orders
                          <small>{money(entry.net_received_fils)} received</small>
                        </td>
                        <td>
                          <span
                            className={`badge ${entry.partial_orders >= 2 ? 'in_progress' : 'not_started'}`}
                          >
                            {entry.partial_orders} partially paid
                          </span>
                          <small>
                            {entry.priced_orders
                              ? `${Math.round((entry.partial_orders / entry.priced_orders) * 100)}% of ${entry.priced_orders} payable orders`
                              : 'No payable orders'}{' '}
                            · {entry.instalment_orders} with multiple receipts
                          </small>
                        </td>
                        <td>
                          <strong className={entry.overdue_orders > 0 ? 'negative-amount' : ''}>
                            {money(entry.overdue_fils)}
                          </strong>
                          <small>
                            {entry.overdue_orders
                              ? `${entry.overdue_orders} orders · oldest ${entry.oldest_overdue_days} days late`
                              : 'No confirmed overdue balance'}
                          </small>
                        </td>
                        <td>
                          {entry.dated_orders
                            ? `${entry.late_receipt_orders} ${entry.late_receipt_orders === 1 ? 'order' : 'orders'}`
                            : 'No payment dates set'}
                          <small>
                            {entry.dated_orders} dated · {entry.missing_deadline_orders} payable
                            orders without a deadline
                          </small>
                        </td>
                        <td>
                          <button
                            className="btn subtle"
                            onClick={() => setSelected(entry.id)}
                            aria-label={`Review ${entry.name}`}
                          >
                            Review <ArrowRight size={14} />
                          </button>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            ) : (
              <Empty title="No matching clients">
                Try another client name or include archived clients.
              </Empty>
            )}
          </section>
          {client && (
            <section className="panel client-history" ref={historyRef}>
              <div className="panel-heading">
                <div>
                  <h2>{client.name} · order & payment history</h2>
                  <p>
                    {money(client.outstanding_fils)} outstanding · {money(client.credit_fils)}{' '}
                    credit held on individual orders.
                  </p>
                </div>
                <button className="btn subtle" onClick={() => setSelected(null)}>
                  Close review
                </button>
              </div>
              {orders.length ? (
                <div className="table-wrap">
                  <table>
                    <thead>
                      <tr>
                        <th>ORDER</th>
                        <th>PAYMENT DUE</th>
                        <th>TOTAL / RECEIVED</th>
                        <th>BALANCE</th>
                        <th>STATUS</th>
                      </tr>
                    </thead>
                    <tbody>
                      {orders.map((order) => (
                        <tr key={order.id}>
                          <td>
                            <button className="table-link" onClick={() => onPayments(order.id)}>
                              <span>
                                <strong>{order.name}</strong>
                                <small>
                                  {order.number}
                                  {order.archived ? ' · Archived order' : ''}
                                </small>
                              </span>
                            </button>
                          </td>
                          <td>{order.payment_due_date ?? 'Not set'}</td>
                          <td>
                            {money(order.total_due_fils)}
                            <small>{money(order.paid_fils)} received</small>
                          </td>
                          <td>{money(order.balance_fils)}</td>
                          <td>
                            <span className="badge">
                              {paymentLabel(order.total_due_fils, order.paid_fils)}
                            </span>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              ) : (
                <Empty title="No non-cancelled orders">
                  Customer receipt history is retained below.
                </Empty>
              )}
              <div className="panel-heading">
                <div>
                  <h3>Recent receipts & refunds</h3>
                  <p>
                    Latest {Math.min(payments.length, 8)} records, including historical cancelled
                    orders.
                  </p>
                </div>
              </div>
              {payments.length ? (
                <div className="table-wrap">
                  <table>
                    <thead>
                      <tr>
                        <th>DATE</th>
                        <th>ORDER</th>
                        <th>METHOD / REFERENCE</th>
                        <th>AMOUNT</th>
                      </tr>
                    </thead>
                    <tbody>
                      {payments.slice(0, 8).map((payment) => (
                        <tr key={payment.id}>
                          <td>{payment.business_date}</td>
                          <td>
                            <button
                              className="table-link"
                              onClick={() => onPayments(payment.order_id)}
                            >
                              {payment.number} · {payment.name}
                            </button>
                          </td>
                          <td>
                            {payment.method}
                            <small>{payment.reference || '—'}</small>
                          </td>
                          <td>
                            {payment.kind === 'refund' ? '−' : ''}
                            {money(payment.amount_fils)}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              ) : (
                <Empty title="No payments recorded">
                  Select an order above to record a receipt or set its payment deadline.
                </Empty>
              )}
            </section>
          )}
        </>
      )}
    </div>
  );
}
