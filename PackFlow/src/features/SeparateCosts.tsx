import { Correction } from './Correction';
import { useEffect, useRef, useState } from 'react';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { z } from 'zod';
import { Plus } from 'lucide-react';
import type { SeparateCostCategory } from '../contracts';
import { money, parseScaled } from '../contracts/money';
import { useSeparateCosts, useWrite } from '../data/hooks';
import { Dialog, Empty, ErrorNotice } from '../components/Dialog';
const labels: Record<SeparateCostCategory, string> = {
  machine: 'Machine',
  finish: 'Finish',
  other: 'Others',
};
export function SeparateCosts({ actor, today }: { actor: string; today: string }) {
  const query = useSeparateCosts();
  const [open, setOpen] = useState(false);
  const data = query.data;
  return (
    <div className="separate-costs">
      <section className="panel">
        <div className="panel-heading">
          <div>
            <h2>Separate costs</h2>
            <p>Record machine, finish and other expenses for the workspace.</p>
          </div>
          <button className="btn primary" disabled={!actor} onClick={() => setOpen(true)}>
            <Plus size={15} />
            Add separate costs
          </button>
        </div>
        <ErrorNotice error={query.error} />
        {query.isPending && (
          <div className="loading" role="status">
            Loading separate costs…
          </div>
        )}
        {data && (
          <>
            <div className="separate-cost-stats">
              {(['machine', 'finish', 'other'] as const).map((category) => (
                <div key={category}>
                  <span>{labels[category]}</span>
                  <strong>
                    {money(
                      data.categories.find((c) => c.category === category)?.amount_fils ?? '0',
                    )}
                  </strong>
                </div>
              ))}
              <div>
                <span>Total separate costs</span>
                <strong>{money(data.total_fils)}</strong>
                <small>{data.count} saved entries · all time</small>
              </div>
            </div>
            {data.records.length ? (
              <>
                <p className="form-note separate-cost-note">
                  Showing the latest {data.records.length} entries ({data.count} active). These
                  expenses also appear in Reports; order margins use their own linked costs.
                </p>
                <div className="table-wrap">
                  <table>
                    <thead>
                      <tr>
                        <th>DATE</th>
                        <th>CATEGORY</th>
                        <th>DESCRIPTION</th>
                        <th>AMOUNT</th>
                        <th>RECORDED BY</th>
                      </tr>
                    </thead>
                    <tbody>
                      {data.records.map((cost) => (
                        <tr key={cost.id}>
                          <td>{cost.business_date}</td>
                          <td>
                            <span className="category">{labels[cost.category]}</span>
                          </td>
                          <td className="separate-cost-description">
                            {cost.description}
                            <small>{cost.reason}</small>
                          </td>
                          <td>{money(cost.amount_fils)}</td>
                          <td>
                            {cost.actor_name}
                            <Correction
                              actor={actor}
                              id={cost.id}
                              type="separate_cost"
                              today={today}
                              reversed={cost.reversed}
                              label={cost.description}
                            />
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </>
            ) : (
              <Empty title="No separate costs recorded">
                Add expenses here, then save as many entries as you need in one dialog.
              </Empty>
            )}
          </>
        )}
      </section>
      {open && <SeparateCostsDialog actor={actor} today={today} onClose={() => setOpen(false)} />}
    </div>
  );
}
const schema = z.object({
  category: z.enum(['machine', 'finish', 'other']),
  amount: z.string().refine((value) => {
    try {
      parseScaled(value);
      return true;
    } catch {
      return false;
    }
  }, 'Enter a cost in KWD with up to three decimals.'),
  description: z.string().trim().min(1, 'Describe this cost.').max(1000),
  business_date: z.string().regex(/^\d{4}-\d{2}-\d{2}$/, 'Choose a date.'),
});
type Values = z.infer<typeof schema>;
function SeparateCostsDialog({
  actor,
  today,
  onClose,
}: {
  actor: string;
  today: string;
  onClose: () => void;
}) {
  const form = useForm<Values>({
    resolver: zodResolver(schema),
    defaultValues: { category: 'machine', amount: '', description: '', business_date: today },
  });
  const write = useWrite(actor);
  const values = form.watch();
  const saving = useRef(false);
  const [saved, setSaved] = useState<
    { id: string; category: SeparateCostCategory; amount: string; description: string }[]
  >([]);
  useEffect(() => {
    if (!write.isPending && saved.length) form.setFocus('amount');
  }, [write.isPending, saved.length, form]);
  const close = () => {
    if (
      !saving.current &&
      ((!values.amount.trim() && !values.description.trim()) ||
        !form.formState.isDirty ||
        window.confirm('Discard this unsaved cost? Previously saved entries will remain.'))
    )
      onClose();
  };
  let preview: string | null = null;
  try {
    preview = parseScaled(values.amount);
  } catch {
    /* Empty inputs stay editable. */
  }
  return (
    <Dialog
      title="Add separate costs"
      subtitle="Save each expense and continue here. Choose Done when you finish."
      onClose={close}
    >
      <form
        onSubmit={form.handleSubmit((v) => {
          if (saving.current) return;
          saving.current = true;
          write.mutate(
            {
              kind: 'add_separate_cost',
              category: v.category,
              amount_fils: parseScaled(v.amount),
              description: v.description,
              business_date: v.business_date,
            },
            {
              onSuccess: (result) => {
                setSaved((entries) => [
                  ...entries,
                  {
                    id: result.data.id!,
                    category: v.category,
                    amount: parseScaled(v.amount),
                    description: v.description,
                  },
                ]);
                form.reset({
                  category: v.category,
                  business_date: v.business_date,
                  amount: '',
                  description: '',
                });
              },
              onSettled: () => {
                saving.current = false;
              },
            },
          );
        })}
      >
        <fieldset className="separate-cost-fields" disabled={write.isPending}>
          <div className="field-row">
            <label>
              Cost category
              <select {...form.register('category')}>
                <option value="machine">Machine</option>
                <option value="finish">Finish</option>
                <option value="other">Others</option>
              </select>
            </label>
            <label>
              Cost amount · KWD
              <input inputMode="decimal" placeholder="0.000" {...form.register('amount')} />
              <span className="field-error">{form.formState.errors.amount?.message}</span>
            </label>
          </div>
          <label>
            Cost description
            <textarea placeholder="Describe this expense" {...form.register('description')} />
            <span className="field-error">{form.formState.errors.description?.message}</span>
          </label>
          <label>
            Cost date
            <input type="date" {...form.register('business_date')} />
            <span className="field-error">{form.formState.errors.business_date?.message}</span>
          </label>
        </fieldset>
        <div className="quote-summary">
          <span>{labels[values.category]} cost</span>
          <strong>{money(preview)}</strong>
        </div>
        {!!saved.length && (
          <div className="separate-cost-saved">
            <p role="status">
              {saved.length} {saved.length === 1 ? 'entry' : 'entries'} saved. Enter another cost or
              choose Done.
            </p>
            <ul>
              {saved.map((entry) => (
                <li key={entry.id}>
                  <span>
                    <strong>{labels[entry.category]}</strong> · {entry.description}
                  </span>
                  <strong>{money(entry.amount)}</strong>
                </li>
              ))}
            </ul>
          </div>
        )}
        <ErrorNotice error={write.error} />
        <div className="dialog-actions">
          <button type="button" className="btn" disabled={write.isPending} onClick={close}>
            Done
          </button>
          <button className="btn primary" disabled={!actor || write.isPending}>
            {write.isPending ? 'Saving…' : 'Save & add another'}
          </button>
        </div>
      </form>
    </Dialog>
  );
}
