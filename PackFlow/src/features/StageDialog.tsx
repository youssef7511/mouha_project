import { Correction } from './Correction';
import { useEffect, useRef, useState } from 'react';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { z } from 'zod';
import type {
  Bootstrap,
  OrderDetail,
  OrderStage,
  ExpenseCategory,
  ExpenseInput,
} from '../contracts';
import { useWrite } from '../data/hooks';
import { decimal, lineTotal, money, parseScaled } from '../contracts/money';
import { Dialog, ErrorNotice } from '../components/Dialog';

const statusSchema = z.object({
  state: z.enum(['not_started', 'in_progress', 'completed']),
  notes: z.string().max(4000),
  business_date: z.string().min(1),
  reason: z.string().max(1000),
});
const workSchema = z
  .object({
    business_date: z.string().min(1),
    description: z.string().trim().min(1, 'Describe the work.'),
    dimensions: z.string().max(200),
    item_id: z.string(),
    input: z.string(),
    output: z.string(),
    waste: z.string(),
    worker_id: z.string(),
    worker_minutes: z.string().regex(/^\d*$/),
    machine: z.string().max(200),
    machine_minutes: z.string().regex(/^\d*$/),
  })
  .superRefine((v, ctx) => {
    for (const f of ['input', 'output', 'waste'] as const)
      if (v[f])
        try {
          parseScaled(v[f], 1_000_000_000n);
        } catch {
          ctx.addIssue({ code: 'custom', path: [f], message: 'Use up to three decimals in Unit.' });
        }
  });
const costSchema = z
  .object({
    business_date: z.string().min(1),
    description: z.string().trim().min(1, 'Describe this expense.').max(1000),
    category: z.enum(['material', 'outsourced', 'machine', 'labor', 'transport', 'other']),
    supplier: z.string().max(200),
    reference: z.string().max(200),
    item_id: z.string(),
    basis: z.enum(['unit', 'total']),
    quantity: z.string(),
    unit_cost: z.string(),
    amount: z.string(),
  })
  .superRefine((v, ctx) => {
    try {
      if (v.quantity) {
        const q = parseScaled(v.quantity, 1_000_000_000n);
        if (BigInt(q) <= 0n) throw new Error();
      } else if (v.basis === 'unit') throw new Error();
    } catch {
      ctx.addIssue({
        code: 'custom',
        path: ['quantity'],
        message: 'Enter a positive quantity in Unit.',
      });
    }
    try {
      if (v.basis === 'unit')
        lineTotal(parseScaled(v.quantity, 1_000_000_000n), parseScaled(v.unit_cost));
      else parseScaled(v.amount);
    } catch {
      ctx.addIssue({
        code: 'custom',
        path: [v.basis === 'unit' ? 'unit_cost' : 'amount'],
        message: 'Enter a valid cost with up to three decimals.',
      });
    }
  });
type WorkValues = z.infer<typeof workSchema>;
type CostValues = z.infer<typeof costSchema>;
export function StageDialog({
  stage,
  detail,
  bootstrap,
  actor,
  onClose,
}: {
  stage: OrderStage;
  detail: OrderDetail;
  bootstrap: Bootstrap;
  actor: string;
  onClose: () => void;
}) {
  const [tab, setTab] = useState<'overview' | 'work' | 'costs' | 'history'>('overview');
  const status = useForm<z.infer<typeof statusSchema>>({
    resolver: zodResolver(statusSchema),
    defaultValues: {
      state: stage.state,
      notes: stage.notes,
      business_date: bootstrap.business_today,
      reason: '',
    },
  });
  const work = useForm<WorkValues>({
    resolver: zodResolver(workSchema),
    defaultValues: {
      business_date: bootstrap.business_today,
      description: '',
      dimensions: '',
      item_id: '',
      input: '',
      output: '',
      waste: '',
      worker_id: '',
      worker_minutes: '',
      machine: '',
      machine_minutes: '',
    },
  });
  const cost = useForm<CostValues>({
    resolver: zodResolver(costSchema),
    defaultValues: {
      business_date: bootstrap.business_today,
      description: '',
      category: 'material',
      supplier: '',
      reference: '',
      item_id: '',
      basis: 'total',
      quantity: '',
      unit_cost: '',
      amount: '',
    },
  });
  const write = useWrite(actor);
  const values = cost.watch();
  const statusDirty = status.formState.isDirty;
  const workDirty = work.formState.isDirty;
  const costDirty = cost.formState.isDirty;
  const savingCost = useRef(false);
  const [savedCosts, setSavedCosts] = useState(0);
  useEffect(() => {
    if (tab === 'costs' && savedCosts > 0 && !write.isPending) cost.setFocus('description');
  }, [tab, savedCosts, write.isPending, cost]);
  const close = () => {
    if (
      !savingCost.current &&
      !write.isPending &&
      (!(statusDirty || workDirty || costDirty) || window.confirm('Discard unsaved stage changes?'))
    )
      onClose();
  };
  const history = detail.history.filter((h) => h.entity_id === stage.id);
  const stagePrice = detail.stage_prices.find((p) => p.stage_id === stage.id);
  const itemOptions = detail.items.map((i) => (
    <option key={i.id} value={i.id}>
      {i.description}
    </option>
  ));
  let preview: string | null = null;
  try {
    preview =
      values.basis === 'unit'
        ? lineTotal(parseScaled(values.quantity, 1_000_000_000n), parseScaled(values.unit_cost))
        : parseScaled(values.amount);
  } catch {
    preview = null;
  }
  function submitWork(v: WorkValues) {
    write.mutate(
      {
        kind: 'post_stage_entry',
        order_id: detail.order.id,
        stage_id: stage.id,
        business_date: v.business_date,
        work: {
          description: v.description,
          dimensions_text: v.dimensions,
          item_id: v.item_id || null,
          input_scaled: v.input ? parseScaled(v.input, 1_000_000_000n) : null,
          output_scaled: v.output ? parseScaled(v.output, 1_000_000_000n) : null,
          waste_scaled: v.waste ? parseScaled(v.waste, 1_000_000_000n) : null,
          worker_id: v.worker_id || null,
          worker_minutes: v.worker_minutes ? Number(v.worker_minutes) : null,
          machine_description: v.machine,
          machine_minutes: v.machine_minutes ? Number(v.machine_minutes) : null,
        },
        expenses: [],
      },
      {
        onSuccess: () => {
          work.reset();
          onClose();
        },
      },
    );
  }
  function submitCost(v: CostValues) {
    if (savingCost.current || write.isPending) return;
    const expense: ExpenseInput = {
      item_id: v.item_id || null,
      category: v.category as ExpenseCategory,
      description: v.description,
      supplier_text: v.supplier,
      reference: v.reference,
      pricing_basis: v.basis,
      quantity_scaled: v.quantity ? parseScaled(v.quantity, 1_000_000_000n) : null,
      unit_cost_fils: v.basis === 'unit' ? parseScaled(v.unit_cost) : null,
      amount_fils: v.basis === 'total' ? parseScaled(v.amount) : null,
    };
    savingCost.current = true;
    write.mutate(
      {
        kind: 'post_stage_entry',
        order_id: detail.order.id,
        stage_id: stage.id,
        business_date: v.business_date,
        work: null,
        expenses: [expense],
      },
      {
        onSuccess: () => {
          cost.reset({
            ...v,
            description: '',
            reference: '',
            quantity: '',
            unit_cost: '',
            amount: '',
          });
          setSavedCosts((count) => count + 1);
        },
        onSettled: () => {
          savingCost.current = false;
        },
      },
    );
  }
  return (
    <Dialog
      wide
      title={`${stage.position + 1}. ${stage.name_snapshot}`}
      subtitle={`${detail.order.number} · ${detail.order.name}`}
      onClose={close}
    >
      <div className="quote-summary">
        <span>Recorded stage cost</span>
        <strong>{money(stage.cost_fils)}</strong>
      </div>
      <div className="tabs" role="tablist" aria-label="Stage management">
        {(['overview', 'work', 'costs', 'history'] as const).map((t) => (
          <button
            type="button"
            role="tab"
            aria-selected={tab === t}
            className={tab === t ? 'active' : ''}
            key={t}
            disabled={write.isPending}
            onClick={() => {
              setTab(t);
              write.reset();
            }}
          >
            {t.charAt(0).toUpperCase() + t.slice(1)}
          </button>
        ))}
      </div>
      {tab === 'overview' && (
        <form
          onSubmit={status.handleSubmit((v) =>
            write.mutate(
              {
                kind: 'update_stage',
                id: stage.id,
                expected_version: stage.version,
                state: v.state,
                notes: v.notes,
                business_date: v.business_date,
                reason: v.reason.trim() || null,
              },
              {
                onSuccess: () => {
                  status.reset(v);
                  onClose();
                },
              },
            ),
          )}
        >
          {stagePrice && (
            <div className="quote-summary">
              <span>Stage customer quote</span>
              <strong>{money(stagePrice.total_fils)}</strong>
            </div>
          )}
          <div className="field-row">
            <label>
              Status
              <select {...status.register('state')}>
                <option value="not_started">Not started</option>
                <option value="in_progress">In progress</option>
                <option value="completed">Completed</option>
              </select>
            </label>
            <label>
              Business date
              <input type="date" {...status.register('business_date')} />
            </label>
          </div>
          <label>
            Stage instructions / notes
            <textarea {...status.register('notes')} />
          </label>
          <label>
            Reason / out-of-order acknowledgement
            <textarea
              placeholder="Required when earlier stages are incomplete"
              {...status.register('reason')}
            />
          </label>
          <p className="form-note">
            Status updates do not create boxes, payments or costs. Earlier stages keep their own
            status.
          </p>
          <ErrorNotice error={write.error} />
          <div className="dialog-actions">
            <button type="button" className="btn" onClick={close}>
              Cancel
            </button>
            <button className="btn primary" disabled={write.isPending}>
              Save stage status
            </button>
          </div>
        </form>
      )}
      {tab === 'work' && (
        <>
          <form onSubmit={work.handleSubmit(submitWork)}>
            <div className="field-row">
              <label>
                Business date
                <input type="date" {...work.register('business_date')} />
              </label>
              <label>
                Box type
                <select {...work.register('item_id')}>
                  <option value="">Whole order</option>
                  {itemOptions}
                </select>
              </label>
            </div>
            <label>
              Work / sheets description
              <textarea
                placeholder="Describe sheet quantities and the work performed"
                {...work.register('description')}
              />
            </label>
            <span className="field-error">{work.formState.errors.description?.message}</span>
            <label>
              Dimensions / specifications
              <input {...work.register('dimensions')} />
            </label>
            <div className="field-row three">
              {(['input', 'output', 'waste'] as const).map((f) => (
                <label key={f}>
                  {f.charAt(0).toUpperCase() + f.slice(1)} · Unit
                  <input inputMode="decimal" placeholder="Optional" {...work.register(f)} />
                  <span className="field-error">{work.formState.errors[f]?.message}</span>
                </label>
              ))}
            </div>
            <div className="field-row">
              <label>
                Worker
                <select {...work.register('worker_id')}>
                  <option value="">Not assigned</option>
                  {bootstrap.workers
                    .filter((w) => w.active && !w.archived)
                    .map((w) => (
                      <option key={w.id} value={w.id}>
                        {w.name}
                      </option>
                    ))}
                </select>
              </label>
              <label>
                Worker time · minutes
                <input inputMode="numeric" {...work.register('worker_minutes')} />
              </label>
            </div>
            <div className="field-row">
              <label>
                Machine work
                <input {...work.register('machine')} />
              </label>
              <label>
                Machine time · minutes
                <input inputMode="numeric" {...work.register('machine_minutes')} />
              </label>
            </div>
            <p className="form-note">
              Quantities are recorded in Unit. Time does not calculate a cost; record manual costs
              in the Costs tab.
            </p>
            <ErrorNotice error={write.error} />
            <div className="dialog-actions">
              <button type="button" className="btn" onClick={close}>
                Cancel
              </button>
              <button className="btn primary" disabled={write.isPending}>
                Record work
              </button>
            </div>
          </form>
          <h3 className="ledger-heading">Recorded work</h3>
          {detail.work
            .filter((w) => w.stage_id === stage.id)
            .map((w) => (
              <div className="ledger-row" key={w.id}>
                <div>
                  <strong>
                    {w.kind === 'reversal' ? 'Reversal: ' : ''}
                    {w.description}
                  </strong>
                  <small>{w.reason}</small>
                  <small>
                    {w.business_date} · {w.worker_minutes ?? 0} worker minutes ·{' '}
                    {w.machine_minutes ?? 0} machine minutes
                  </small>
                </div>
                <div>
                  {w.kind === 'work' && (
                    <Correction
                      actor={actor}
                      id={w.id}
                      type="work"
                      today={bootstrap.business_today}
                      reversed={w.reversed}
                      label={w.description}
                    />
                  )}
                </div>
                <span>{w.input_scaled !== null ? `${decimal(w.input_scaled)} Unit` : '—'}</span>
              </div>
            ))}
        </>
      )}
      {tab === 'costs' && (
        <>
          <form onSubmit={cost.handleSubmit(submitCost)}>
            <p className="form-note">
              Save each cost and continue here. Choose Done when you finish.
            </p>
            <fieldset className="separate-cost-fields" disabled={write.isPending}>
              <div className="field-row">
                <label>
                  Category
                  <select {...cost.register('category')}>
                    <option value="material">Material · direct purchase</option>
                    <option value="outsourced">Outsourced work</option>
                    <option value="machine">Machine work</option>
                    <option value="labor">Worker labor</option>
                    <option value="transport">Transport</option>
                    <option value="other">Other</option>
                  </select>
                </label>
                <label>
                  Business date
                  <input type="date" {...cost.register('business_date')} />
                </label>
              </div>
              <label>
                Expense description
                <input {...cost.register('description')} />
              </label>
              <span className="field-error">{cost.formState.errors.description?.message}</span>
              <div className="field-row">
                <label>
                  Supplier / provider
                  <input {...cost.register('supplier')} />
                </label>
                <label>
                  Reference
                  <input {...cost.register('reference')} />
                </label>
              </div>
              <div className="field-row">
                <label>
                  Box type
                  <select {...cost.register('item_id')}>
                    <option value="">Whole order</option>
                    {itemOptions}
                  </select>
                </label>
                <label>
                  Cost entry
                  <select {...cost.register('basis')}>
                    <option value="total">Manual total amount</option>
                    <option value="unit">Quantity × unit purchase cost</option>
                  </select>
                </label>
              </div>
              <div className="field-row">
                <label>
                  Quantity · Unit
                  <input
                    inputMode="decimal"
                    placeholder={values.basis === 'unit' ? 'Required' : 'Optional'}
                    {...cost.register('quantity')}
                  />
                  <span className="field-error">{cost.formState.errors.quantity?.message}</span>
                </label>
                {values.basis === 'unit' ? (
                  <label>
                    Purchase cost per Unit · KWD
                    <input inputMode="decimal" {...cost.register('unit_cost')} />
                    <span className="field-error">{cost.formState.errors.unit_cost?.message}</span>
                  </label>
                ) : (
                  <label>
                    Total cost · KWD
                    <input inputMode="decimal" {...cost.register('amount')} />
                    <span className="field-error">{cost.formState.errors.amount?.message}</span>
                  </label>
                )}
              </div>
            </fieldset>
            <div className="quote-summary">
              <span>Cost to record</span>
              <strong>{preview === null ? 'Enter an amount' : money(preview)}</strong>
            </div>
            <p className="form-note">
              A direct expense for this order. It does not change the customer quote or any stock
              balance.
            </p>
            {savedCosts > 0 && (
              <p className="stage-cost-saved" role="status">
                {savedCosts} {savedCosts === 1 ? 'cost' : 'costs'} saved. Add another cost or choose
                Done.
              </p>
            )}
            <ErrorNotice error={write.error} />
            <div className="dialog-actions">
              <button type="button" className="btn" disabled={write.isPending} onClick={close}>
                Done
              </button>
              <button className="btn primary" disabled={write.isPending}>
                {write.isPending ? 'Saving…' : 'Save & add another'}
              </button>
            </div>
          </form>
          <h3 className="ledger-heading">Recorded costs</h3>
          {detail.costs
            .filter((c) => c.stage_id === stage.id)
            .map((c) => (
              <div className="ledger-row" key={c.id}>
                <div>
                  <strong>
                    {c.kind === 'reversal' ? 'Reversal: ' : ''}
                    {c.description}
                  </strong>
                  <small>{c.reason}</small>
                  <small>
                    {c.business_date} · {c.category}
                  </small>
                </div>
                <span>
                  {c.kind === 'reversal' ? '−' : ''}
                  {money(c.amount_fils)}
                </span>
                {c.kind === 'expense' && !c.service_linked && (
                  <Correction
                    actor={actor}
                    id={c.id}
                    type="expense"
                    today={bootstrap.business_today}
                    reversed={c.reversed}
                    label={c.description}
                  />
                )}
              </div>
            ))}
        </>
      )}
      {tab === 'history' && (
        <div className="history-list">
          {history.length ? (
            history.map((h) => (
              <div className="history-row" key={h.id}>
                <span className="history-dot" />
                <div>
                  <strong>{h.action.replaceAll('_', ' ')}</strong>
                  <small>
                    {h.actor_name} · {h.business_date}
                  </small>
                  {h.reason && <p>{h.reason}</p>}
                </div>
              </div>
            ))
          ) : (
            <p>No changes recorded for this stage.</p>
          )}
        </div>
      )}
    </Dialog>
  );
}
