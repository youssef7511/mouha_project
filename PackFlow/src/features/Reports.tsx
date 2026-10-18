import { useState } from 'react';
import { useExcelFilters } from './ExcelExport';
import { useOrderPage, useReports } from '../data/hooks';
import { ListSearch, Pagination } from '../components/Pagination';
import { money } from '../contracts/money';
import type { Reports as ReportData } from '../contracts';
import { Empty, ErrorNotice } from '../components/Dialog';

const categoryLabel: Record<string, string> = {
  material: 'Materials',
  outsourced: 'Outside services',
  machine: 'Machine work',
  labor: 'Manual labor costs',
  transport: 'Transport',
  other: 'Other',
  separate_machine: 'Separate costs · Machine',
  separate_finish: 'Separate costs · Finish',
  separate_other: 'Separate costs · Others',
};
const sum = (values: string[]) =>
  values.reduce((total, value) => total + BigInt(value), 0n).toString();
function Metric({ label, value, note }: { label: string; value: string; note: string }) {
  return (
    <div className="stat report-metric">
      <span>{label}</span>
      <strong className="stat-value">{value}</strong>
      <small>{note}</small>
    </div>
  );
}
function dateOffset(today: string, days: number) {
  const date = new Date(`${today}T12:00:00Z`);
  date.setUTCDate(date.getUTCDate() + days);
  return date.toISOString().slice(0, 10);
}
function ActivityChart({ data }: { data: ReportData }) {
  const days: ReportData['activity'] = [];
  for (let day = data.from_date; day <= data.to_date; day = dateOffset(day, 1)) {
    days.push(
      data.activity.find((entry) => entry.business_date === day) ?? {
        business_date: day,
        cost_fils: '0',
        received_fils: '0',
      },
    );
  }
  const values = days.flatMap((day) => [Number(day.cost_fils), Number(day.received_fils)]);
  const minimum = Math.min(0, ...values);
  const maximum = Math.max(1, ...values);
  const y = (value: number) => 205 - ((value - minimum) / (maximum - minimum)) * 168;
  const baseline = y(0);
  const step = 540 / days.length;
  const width = Math.min(14, step * 0.32);
  return (
    <>
      <div className="chart-legend">
        <span>
          <i className="cost-color" />
          Recorded costs
        </span>
        <span>
          <i className="receipt-color" />
          Net receipts
        </span>
      </div>
      <svg
        className="activity-chart"
        viewBox="0 0 640 250"
        role="img"
        aria-label={`Daily costs and net receipts in KWD, ${data.from_date} to ${data.to_date}`}
      >
        <title>Daily recorded costs and net customer receipts</title>
        {[0, 1, 2, 3, 4].map((tick) => (
          <g key={tick}>
            <line x1="76" x2="616" y1={205 - tick * 42} y2={205 - tick * 42} stroke="#e6e8e2" />
            <text x="66" y={209 - tick * 42} textAnchor="end">
              {((minimum + ((maximum - minimum) * tick) / 4) / 1000).toFixed(1)}
            </text>
          </g>
        ))}
        <text x="20" y="22">
          KWD
        </text>
        {days.map((day, index) => {
          const x = 76 + step * (index + 0.5);
          const cost = y(Number(day.cost_fils));
          const receipts = y(Number(day.received_fils));
          return (
            <g key={day.business_date}>
              <title>
                {day.business_date}: costs {money(day.cost_fils)}, net receipts{' '}
                {money(day.received_fils)}
              </title>
              <rect
                x={x - width - 1}
                y={Math.min(baseline, cost)}
                width={width}
                height={Math.abs(baseline - cost)}
                rx="2"
                fill="#c47d52"
              />
              <rect
                x={x + 1}
                y={Math.min(baseline, receipts)}
                width={width}
                height={Math.abs(baseline - receipts)}
                rx="2"
                fill="#698262"
              />
              {(index % Math.max(1, Math.ceil(days.length / 8)) === 0 ||
                index === days.length - 1) && (
                <text x={x} y="230" textAnchor="middle">
                  {day.business_date.slice(5)}
                </text>
              )}
            </g>
          );
        })}
      </svg>
      <details className="chart-data">
        <summary>View daily amounts</summary>
        <div className="table-wrap">
          <table>
            <thead>
              <tr>
                <th>DATE</th>
                <th>COSTS</th>
                <th>NET RECEIPTS</th>
              </tr>
            </thead>
            <tbody>
              {days.map((day) => (
                <tr key={day.business_date}>
                  <td>{day.business_date}</td>
                  <td>{money(day.cost_fils)}</td>
                  <td>{money(day.received_fils)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </details>
    </>
  );
}
export function Reports({ today, onPick }: { today: string; onPick: (id: string) => void }) {
  const [from, setFrom] = useState(`${today.slice(0, 8)}01`);
  const [to, setTo] = useState(today);
  const [performanceSearch, setPerformanceSearch] = useState('');
  useExcelFilters({ page: 'reports', from_date: from, to_date: to, search: performanceSearch });
  const [performancePage, setPerformancePage] = useState(1);
  const [performancePageSize, setPerformancePageSize] = useState(25);
  const performance = useOrderPage(
    performanceSearch,
    performancePage,
    performancePageSize,
    'active',
  );
  const query = useReports(from, to);
  const data = query.data;
  const quoted = data?.portfolio_summary.total_due_fils ?? '0';
  const unpriced = data?.portfolio_summary.unpriced_orders ?? 0;
  const maxCategory = Math.max(
    1,
    ...(data?.categories.map((row) => Math.abs(Number(row.amount_fils))) ?? []),
  );
  return (
    <>
      <div className="page-heading">
        <div>
          <h1>A clear picture of your workspace.</h1>
          <p>Order value, production progress, costs and customer receipts from saved records.</p>
        </div>
        <span className="badge completed">Live local data</span>
      </div>
      <div className="report-toolbar">
        <div className="report-dates">
          <label className="standalone-select">
            From
            <input type="date" value={from} onChange={(e) => setFrom(e.target.value)} />
          </label>
          <label className="standalone-select">
            To
            <input type="date" value={to} onChange={(e) => setTo(e.target.value)} />
          </label>
        </div>
        <div className="report-presets">
          {[7, 30, 90].map((days) => (
            <button
              key={days}
              className="btn"
              onClick={() => {
                setFrom(dateOffset(today, 1 - days));
                setTo(today);
              }}
            >
              Last {days} days
            </button>
          ))}
        </div>
      </div>
      <p className="report-scope">
        The date range filters separate costs and order cost/receipt activity, including archived
        and cancelled orders. Order totals and stage progress show the current workspace, excluding
        archived and cancelled orders.
      </p>
      <ErrorNotice error={query.error} />
      {query.isPending && (
        <div className="panel loading" role="status">
          Preparing your reports…
        </div>
      )}
      {data && (
        <>
          <div className="stats">
            <Metric
              label="Workspace order value"
              value={money(quoted)}
              note={`${unpriced} unpriced orders excluded · includes service charges`}
            />
            <Metric
              label="Costs in selected period"
              value={money(sum(data.activity.map((row) => row.cost_fils)))}
              note="Order expenses, outside services and separate costs"
            />
            <Metric
              label="Net receipts in selected period"
              value={money(sum(data.activity.map((row) => row.received_fils)))}
              note="Customer receipts less any recorded refunds"
            />
            <Metric
              label="Orders in workspace"
              value={String(data.portfolio_summary.order_count)}
              note={`${data.portfolio_summary.ordered_boxes.toLocaleString()} ordered boxes`}
            />
          </div>
          <div className="reports-grid">
            <section className="panel report-activity">
              <div className="panel-heading">
                <div>
                  <h2>Costs & customer receipts</h2>
                  <p>
                    Daily amounts · {data.from_date} to {data.to_date}
                  </p>
                </div>
              </div>
              {data.activity.length ? (
                <ActivityChart data={data} />
              ) : (
                <Empty title="No financial activity in this period">
                  Choose another date range or record costs and customer payments.
                </Empty>
              )}
            </section>
            <section className="panel">
              <div className="panel-heading">
                <div>
                  <h2>Cost breakdown</h2>
                  <p>Recorded expense categories in the selected period.</p>
                </div>
              </div>
              {data.categories.length ? (
                <div className="category-chart" aria-label="Costs by category">
                  {data.categories.map((row) => (
                    <div key={row.category}>
                      <div className="between">
                        <span>{categoryLabel[row.category]}</span>
                        <strong>{money(row.amount_fils)}</strong>
                      </div>
                      <div className="chart-track">
                        <span
                          style={{
                            width: `${(Math.abs(Number(row.amount_fils)) / maxCategory) * 100}%`,
                          }}
                        />
                      </div>
                    </div>
                  ))}
                </div>
              ) : (
                <Empty title="No costs to group">
                  Expenses appear here after they are recorded.
                </Empty>
              )}
            </section>
            <section className="panel report-stages">
              <div className="panel-heading">
                <div>
                  <h2>Production stage progress</h2>
                  <p>Current manual stage states across workspace orders.</p>
                </div>
              </div>
              <div className="chart-legend">
                <span>
                  <i className="waiting-color" />
                  Not started
                </span>
                <span>
                  <i className="cost-color" />
                  In progress
                </span>
                <span>
                  <i className="receipt-color" />
                  Completed
                </span>
              </div>
              {data.stages.length ? (
                <div className="stage-chart">
                  {data.stages.map((stage) => {
                    const total = stage.not_started + stage.in_progress + stage.completed;
                    return (
                      <div key={stage.name}>
                        <div className="between">
                          <strong>{stage.name}</strong>
                          <span>
                            {stage.completed} / {total} complete
                          </span>
                        </div>
                        <div
                          className="stacked-track"
                          role="img"
                          aria-label={`${stage.name}: ${stage.not_started} not started, ${stage.in_progress} in progress, ${stage.completed} completed`}
                        >
                          <span
                            className="waiting-color"
                            style={{ width: `${(stage.not_started / total) * 100}%` }}
                          />
                          <span
                            className="cost-color"
                            style={{ width: `${(stage.in_progress / total) * 100}%` }}
                          />
                          <span
                            className="receipt-color"
                            style={{ width: `${(stage.completed / total) * 100}%` }}
                          />
                        </div>
                        <small>
                          {stage.not_started} waiting · {stage.in_progress} in progress ·{' '}
                          {stage.completed} completed
                        </small>
                      </div>
                    );
                  })}
                </div>
              ) : (
                <Empty title="No production stages yet">
                  Create an order to begin tracking progress.
                </Empty>
              )}
            </section>
          </div>
          <section className="panel">
            <div className="panel-heading">
              <div>
                <h2>Order performance</h2>
                <p>
                  Current totals and recorded-cost margins. Select an order to inspect its stages.
                </p>
              </div>
            </div>
            <div className="list-toolbar">
              <ListSearch
                label="Search order performance"
                value={performanceSearch}
                onChange={(value) => {
                  setPerformanceSearch(value);
                  setPerformancePage(1);
                }}
              />
            </div>
            <ErrorNotice error={performance.error} />
            {performance.isPending || performance.isSearching ? (
              <div className="loading" role="status">
                Finding order performance…
              </div>
            ) : performance.data?.records.length ? (
              <div className="table-wrap">
                <table>
                  <thead>
                    <tr>
                      <th>ORDER</th>
                      <th>BOXES</th>
                      <th>ORDER TOTAL</th>
                      <th>RECORDED COST</th>
                      <th>ESTIMATED MARGIN</th>
                      <th>BALANCE</th>
                    </tr>
                  </thead>
                  <tbody>
                    {performance.data.records.map((order) => {
                      const margin =
                        order.total_due_fils === null
                          ? null
                          : (BigInt(order.total_due_fils) - BigInt(order.cost_fils)).toString();
                      return (
                        <tr key={order.id}>
                          <td>
                            <button className="table-link" onClick={() => onPick(order.id)}>
                              <span>
                                <strong>{order.name}</strong>
                                <small>
                                  {order.number} · {order.client_name}
                                </small>
                              </span>
                            </button>
                          </td>
                          <td>{order.ordered_boxes.toLocaleString()}</td>
                          <td>{money(order.total_due_fils)}</td>
                          <td>{money(order.cost_fils)}</td>
                          <td
                            className={
                              margin !== null && BigInt(margin) < 0n ? 'negative-amount' : ''
                            }
                          >
                            {margin === null ? 'Unknown' : money(margin)}
                          </td>
                          <td>{money(order.balance_fils)}</td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            ) : (
              <Empty
                title={
                  performanceSearch
                    ? 'No matching order performance'
                    : 'Your first report starts with an order'
                }
              >
                {performanceSearch
                  ? 'Try another order name, client or number.'
                  : 'Add clients and orders, then record production work and costs.'}
              </Empty>
            )}
            {performance.data && (
              <Pagination
                label="Order performance"
                total={performance.data.total}
                page={performance.data.page}
                pageSize={performancePageSize}
                onPage={setPerformancePage}
                onPageSize={(size) => {
                  setPerformancePageSize(size);
                  setPerformancePage(1);
                }}
                busy={performance.isFetching || performance.isSearching}
              />
            )}
          </section>
        </>
      )}
    </>
  );
}
