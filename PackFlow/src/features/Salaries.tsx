import { useState } from 'react';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { z } from 'zod';
import { Plus } from 'lucide-react';
import type { SalariesData, SalaryRecord } from '../contracts';
import { useSalaries, useWrite } from '../data/hooks';
import { decimal, money, parseScaled } from '../contracts/money';
import { salaryAmounts } from '../contracts/salary';
import { Dialog, Empty, ErrorNotice } from '../components/Dialog';

export function Salaries({ actor, today }: { actor: string; today: string }) {
  const [month, setMonth] = useState(today.slice(0, 7));
  const [archived, setArchived] = useState(false);
  const [editing, setEditing] = useState<SalaryRecord | null | undefined>(undefined);
  const query = useSalaries(month);
  const data = query.data;
  const write = useWrite(actor);
  const records = data?.records.filter((r) => archived || !r.archived) ?? [];
  return (
    <div className="salary-workspace">
      <section className="panel salary-month-panel">
        <div className="panel-heading">
          <div>
            <h2>Monthly salaries</h2>
            <p>
              One salary record per worker and month, with a separate monthly calculation basis.
            </p>
          </div>
          <button
            className="btn primary"
            disabled={!actor || !data}
            onClick={() => setEditing(null)}
          >
            <Plus size={15} />
            Add monthly salary
          </button>
        </div>
        <div className="salary-month-controls">
          <label>
            Salary month
            <input type="month" value={month} onChange={(e) => setMonth(e.target.value)} />
          </label>
          {!!data?.months.length && (
            <label>
              Saved months
              <select
                value={data.months.some((m) => m.month === month) ? month : ''}
                onChange={(e) => {
                  if (e.target.value) setMonth(e.target.value);
                }}
              >
                <option value="">Choose a saved month</option>
                {data.months.map((m) => (
                  <option key={m.month}>{m.month}</option>
                ))}
              </select>
            </label>
          )}
          {data && <DivisorForm key={`${data.month}-${data.version}`} data={data} actor={actor} />}
        </div>
        <p className="salary-formulas">
          Normal pay = basic ÷ divisor × normal days. Friday pay = basic ÷ divisor × Fridays × 1.5.
          Overtime = basic ÷ divisor ÷ hours/day × overtime hours × 1.25. Each payable component
          rounds once to three decimals.
        </p>
      </section>
      <ErrorNotice error={query.error ?? write.error} />
      {query.isPending && month && (
        <div className="panel loading" role="status">
          Loading monthly salaries…
        </div>
      )}
      {data && (
        <>
          <div className="stats">
            <div className="stat report-metric">
              <span>Normal pay</span>
              <strong className="stat-value">{money(data.totals.normal_fils)}</strong>
              <small>
                {data.totals.workers} included workers · {month}
              </small>
            </div>
            <div className="stat report-metric">
              <span>Friday pay</span>
              <strong className="stat-value">{money(data.totals.friday_fils)}</strong>
              <small>Daily rate × Fridays × 1.5</small>
            </div>
            <div className="stat report-metric">
              <span>Overtime pay</span>
              <strong className="stat-value">{money(data.totals.overtime_fils)}</strong>
              <small>Hourly rate × overtime hours × 1.25</small>
            </div>
            <div className="stat report-metric">
              <span>Total salaries</span>
              <strong className="stat-value">{money(data.totals.total_fils)}</strong>
              <small>Calculated amounts for this month</small>
            </div>
          </div>
          <section className="panel salary-records">
            <div className="panel-heading">
              <div>
                <h2>Worker salaries · {month}</h2>
                <p>
                  Monthly inputs are saved separately from order work. Archived rows are excluded
                  from totals.
                </p>
              </div>
              <label className="client-archive-toggle">
                <input
                  type="checkbox"
                  checked={archived}
                  onChange={(e) => setArchived(e.target.checked)}
                />
                Show archived salaries
              </label>
            </div>
            {records.length ? (
              <div className="table-wrap">
                <table>
                  <thead>
                    <tr>
                      <th>NAME</th>
                      <th>BASIC SALARY</th>
                      <th>NORMAL DAYS</th>
                      <th>HOURS / DAY</th>
                      <th>FRIDAYS</th>
                      <th>OVERTIME HOURS</th>
                      <th>CALCULATED PAY</th>
                      <th>TOTAL</th>
                      <th />
                    </tr>
                  </thead>
                  <tbody>
                    {records.map((r) => (
                      <tr key={r.id}>
                        <td>
                          <strong>{r.worker_name_snapshot}</strong>
                          {!!r.archived && <small>Archived salary</small>}
                        </td>
                        <td>{money(r.basic_fils)}</td>
                        <td>{decimal(r.normal_days_scaled)}</td>
                        <td>{decimal(r.hours_per_day_scaled)}</td>
                        <td>{r.fridays}</td>
                        <td>{decimal(r.overtime_hours_scaled)}</td>
                        <td>
                          <small>Normal {money(r.normal_fils)}</small>
                          <small>Friday {money(r.friday_fils)}</small>
                          <small>Overtime {money(r.overtime_fils)}</small>
                        </td>
                        <td>
                          <strong>{money(r.total_fils)}</strong>
                        </td>
                        <td>
                          {!r.archived && (
                            <button
                              className="btn subtle"
                              onClick={() => setEditing(r)}
                              aria-label={`Edit salary for ${r.worker_name_snapshot}`}
                            >
                              Edit
                            </button>
                          )}
                          <button
                            className="btn subtle"
                            disabled={!actor || write.isPending}
                            onClick={() =>
                              write.mutate({
                                kind: 'archive_salary',
                                id: r.id,
                                expected_version: r.version,
                                expected_month_version: data.version,
                                archived: !r.archived,
                              })
                            }
                            aria-label={`${r.archived ? 'Restore' : 'Archive'} salary for ${r.worker_name_snapshot}`}
                          >
                            {r.archived ? 'Restore' : 'Archive'}
                          </button>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            ) : (
              <Empty title="No monthly salaries yet">
                Choose the month’s divisor, then add each worker’s basic salary, days, working
                hours, Fridays and overtime.
              </Empty>
            )}
          </section>
          {!!data.history.length && (
            <section className="panel salary-history">
              <div className="panel-heading">
                <div>
                  <h2>Salary change history</h2>
                  <p>Latest changes for this month.</p>
                </div>
              </div>
              <div className="table-wrap">
                <table>
                  <thead>
                    <tr>
                      <th>CHANGE</th>
                      <th>ACTOR</th>
                      <th>TIME</th>
                    </tr>
                  </thead>
                  <tbody>
                    {data.history.map((h) => (
                      <tr key={h.id}>
                        <td>{historyText(h, data)}</td>
                        <td>{h.actor_name}</td>
                        <td>{h.created_at}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </section>
          )}
        </>
      )}
      {editing !== undefined && data && (
        <SalaryDialog
          key={editing?.id ?? `new-${month}`}
          data={data}
          record={editing}
          actor={actor}
          onClose={() => setEditing(undefined)}
        />
      )}
    </div>
  );
}
function historyText(h: SalariesData['history'][number], data: SalariesData) {
  const action = JSON.parse(h.after_json);
  const change =
    h.action === 'salary_divisor_updated'
      ? `Monthly divisor set to ${action.divisor} days`
      : h.action === 'salary_archived'
        ? 'Salary archived'
        : h.action === 'salary_restored'
          ? 'Salary restored'
          : action.id
            ? 'Worker salary updated'
            : 'Worker salary added';
  const record = data.records.find((r) => r.id === action.id || r.worker_id === action.worker_id);
  return record ? `${change} · ${record.worker_name_snapshot}` : change;
}
function DivisorForm({ data, actor }: { data: SalariesData; actor: string }) {
  const [value, setValue] = useState(String(data.divisor));
  const [error, setError] = useState<unknown>(null);
  const write = useWrite(actor);
  return (
    <form
      className="salary-divisor-form"
      onSubmit={(e) => {
        e.preventDefault();
        setError(null);
        if (!/^\d+$/.test(value) || Number(value) < 1 || Number(value) > 31) {
          setError(new Error('Enter 1–31 days.'));
          return;
        }
        write.mutate({
          kind: 'set_salary_divisor',
          month: data.month,
          expected_version: data.version,
          divisor: Number(value),
        });
      }}
    >
      <div className="salary-divisor-input">
        <label>
          Monthly salary divisor
          <input inputMode="numeric" value={value} onChange={(e) => setValue(e.target.value)} />
        </label>
        <button
          className="btn"
          disabled={!actor || write.isPending || value === String(data.divisor)}
        >
          {write.isPending ? 'Saving…' : 'Save divisor'}
        </button>
      </div>
      <small>Changing this divisor recalculates this month’s saved salaries.</small>
      <ErrorNotice error={error ?? write.error} />
    </form>
  );
}
const salarySchema = z
  .object({
    worker_id: z.string().min(1, 'Choose a worker.'),
    basic: z.string(),
    normal_days: z.string(),
    hours_per_day: z.string(),
    fridays: z
      .string()
      .regex(/^\d+$/, 'Enter whole Friday days.')
      .refine((v) => Number(v) <= 31, 'Enter 0–31 Friday days.'),
    overtime: z.string(),
    notes: z.string().max(2000),
  })
  .superRefine((v, ctx) => {
    for (const [field, max, positive] of [
      ['basic', 9_000_000_000_000n, false],
      ['normal_days', 31000n, false],
      ['hours_per_day', 24000n, true],
      ['overtime', 744000n, false],
    ] as const) {
      try {
        const amount = BigInt(parseScaled(v[field], max));
        if (positive && amount <= 0n) throw new Error();
      } catch {
        ctx.addIssue({
          code: 'custom',
          path: [field],
          message:
            field === 'hours_per_day'
              ? 'Enter positive hours/day, up to 24.'
              : 'Enter a nonnegative value within range, using up to three decimals.',
        });
      }
    }
  });
type SalaryValues = z.infer<typeof salarySchema>;
function SalaryDialog({
  data,
  record,
  actor,
  onClose,
}: {
  data: SalariesData;
  record: SalaryRecord | null;
  actor: string;
  onClose: () => void;
}) {
  const form = useForm<SalaryValues>({
    resolver: zodResolver(salarySchema),
    defaultValues: {
      worker_id: record?.worker_id ?? '',
      basic: record ? decimal(record.basic_fils) : '',
      normal_days: record ? decimal(record.normal_days_scaled) : String(data.divisor),
      hours_per_day: record ? decimal(record.hours_per_day_scaled) : '',
      fridays: String(record?.fridays ?? 0),
      overtime: record ? decimal(record.overtime_hours_scaled) : '0',
      notes: record?.notes ?? '',
    },
  });
  const values = form.watch();
  const write = useWrite(actor);
  const [localError, setLocalError] = useState<unknown>(null);
  let preview: ReturnType<typeof salaryAmounts> | null = null;
  try {
    preview = salaryAmounts(
      parseScaled(values.basic),
      parseScaled(values.normal_days, 31000n),
      parseScaled(values.hours_per_day, 24000n),
      Number(values.fridays),
      parseScaled(values.overtime, 744000n),
      data.divisor,
    );
  } catch {
    /* Preview appears when inputs are complete. */
  }
  const close = () => {
    if (
      !write.isPending &&
      (!form.formState.isDirty || window.confirm('Discard unsaved salary changes?'))
    )
      onClose();
  };
  const available = data.workers.filter(
    (w) =>
      w.id === record?.worker_id ||
      (w.active && !w.archived && !data.records.some((r) => r.worker_id === w.id)),
  );
  return (
    <Dialog
      title={record ? 'Edit monthly salary' : 'Add monthly salary'}
      subtitle={`${data.month} · salary divisor ${data.divisor} days`}
      onClose={close}
      wide
    >
      <form
        onSubmit={form.handleSubmit((v) => {
          try {
            setLocalError(null);
            salaryAmounts(
              parseScaled(v.basic),
              parseScaled(v.normal_days, 31000n),
              parseScaled(v.hours_per_day, 24000n),
              Number(v.fridays),
              parseScaled(v.overtime, 744000n),
              data.divisor,
            );
            write.mutate(
              {
                kind: 'save_salary',
                id: record?.id ?? null,
                expected_version: record?.version ?? null,
                month: data.month,
                expected_month_version: data.version,
                worker_id: v.worker_id,
                basic_fils: parseScaled(v.basic),
                normal_days_scaled: parseScaled(v.normal_days, 31000n),
                hours_per_day_scaled: parseScaled(v.hours_per_day, 24000n),
                fridays: Number(v.fridays),
                overtime_hours_scaled: parseScaled(v.overtime, 744000n),
                notes: v.notes,
              },
              { onSuccess: onClose },
            );
          } catch (error) {
            setLocalError(error);
          }
        })}
      >
        <div className="field-row">
          <label>
            Worker name
            <select {...form.register('worker_id')} disabled={!!record}>
              <option value="">Choose a worker</option>
              {available.map((w) => (
                <option key={w.id} value={w.id}>
                  {record && record.worker_id === w.id ? record.worker_name_snapshot : w.name}
                </option>
              ))}
            </select>
            <span className="field-error">{form.formState.errors.worker_id?.message}</span>
          </label>
          <label>
            Basic salary · KWD
            <input inputMode="decimal" placeholder="0.000" {...form.register('basic')} />
            <span className="field-error">{form.formState.errors.basic?.message}</span>
          </label>
        </div>
        {!available.length && (
          <p className="form-note">
            Add an active worker profile, or edit/restore their existing salary for this month.
          </p>
        )}
        <div className="field-row">
          <label>
            Normal days worked
            <input inputMode="decimal" {...form.register('normal_days')} />
            <span className="field-error">{form.formState.errors.normal_days?.message}</span>
          </label>
          <label>
            Working hours / day
            <input inputMode="decimal" placeholder="e.g. 6" {...form.register('hours_per_day')} />
            <span className="field-error">{form.formState.errors.hours_per_day?.message}</span>
          </label>
        </div>
        <div className="field-row">
          <label>
            Fridays worked · days
            <input inputMode="numeric" {...form.register('fridays')} />
            <span className="field-error">{form.formState.errors.fridays?.message}</span>
          </label>
          <label>
            Overtime · hours
            <input inputMode="decimal" {...form.register('overtime')} />
            <span className="field-error">{form.formState.errors.overtime?.message}</span>
          </label>
        </div>
        <p className="form-note">
          Enter normal days separately from Fridays. Friday pay uses 1.5 × daily rate; overtime uses
          1.25 × hourly rate. Monthly salary amounts are calculated here independently of order
          costs.
        </p>
        <label>
          Salary notes
          <textarea {...form.register('notes')} />
        </label>
        <div className="salary-preview" aria-live="polite">
          <div>
            <span>Normal pay</span>
            <strong>{money(preview?.normal_fils ?? null)}</strong>
          </div>
          <div>
            <span>Friday pay</span>
            <strong>{money(preview?.friday_fils ?? null)}</strong>
          </div>
          <div>
            <span>Overtime pay</span>
            <strong>{money(preview?.overtime_fils ?? null)}</strong>
          </div>
        </div>
        <div className="quote-summary">
          <span>Total calculated salary</span>
          <strong>{money(preview?.total_fils ?? null)}</strong>
        </div>
        <ErrorNotice error={localError ?? write.error} />
        <div className="dialog-actions">
          <button type="button" className="btn" onClick={close}>
            Cancel
          </button>
          <button className="btn primary" disabled={!actor || write.isPending}>
            {write.isPending ? 'Saving…' : 'Save monthly salary'}
          </button>
        </div>
      </form>
    </Dialog>
  );
}
