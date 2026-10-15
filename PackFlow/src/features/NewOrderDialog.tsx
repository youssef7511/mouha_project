import { useId, useRef, useState } from 'react';
import { useFieldArray, useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { z } from 'zod';
import { Plus, Trash2 } from 'lucide-react';
import type { Bootstrap, OrderSummary, OrderDetail } from '../contracts';
import { useWrite } from '../data/hooks';
import { decimal, money, parseScaled, lineTotal, MAX_MONEY } from '../contracts/money';
import { repository } from '../data/repository';
import { Dialog, ErrorNotice } from '../components/Dialog';
const name = z.string().trim().min(1, 'A name is required.').max(120);
const stagePriceSchema = z.object({
  template_id: z.string(),
  pricing_mode: z.enum(['unpriced', 'fixed', 'per_box', 'per_unit']),
  fixed_total: z.string(),
  quantity: z.string(),
  unit_price: z.string(),
  item_prices: z.array(z.string()),
});
type StagePriceForm = z.infer<typeof stagePriceSchema>;
const emptyPrice = (template_id: string): StagePriceForm => ({
  template_id,
  pricing_mode: 'unpriced',
  fixed_total: '',
  quantity: '',
  unit_price: '',
  item_prices: [''],
});
function stageTotal(stage: StagePriceForm, items: { ordered_boxes: string }[]): string | null {
  if (stage.pricing_mode === 'unpriced') return null;
  if (stage.pricing_mode === 'fixed') return parseScaled(stage.fixed_total);
  if (stage.pricing_mode === 'per_unit')
    return lineTotal(parseScaled(stage.quantity, 1_000_000_000n), parseScaled(stage.unit_price));
  const total = items.reduce((sum, item, index) => {
    if (
      !/^\d+$/.test(item.ordered_boxes) ||
      Number(item.ordered_boxes) < 1 ||
      Number(item.ordered_boxes) > 1_000_000
    )
      throw new Error('Enter valid box quantities.');
    return sum + BigInt(parseScaled(stage.item_prices[index] ?? '')) * BigInt(item.ordered_boxes);
  }, 0n);
  if (total > MAX_MONEY) throw new Error('Quote exceeds the supported range.');
  return total.toString();
}
const orderSchema = z
  .object({
    name,
    client_id: z.string().min(1, 'Choose a client.'),
    due_date: z.string().regex(/^\d{4}-\d{2}-\d{2}$/, 'Choose a due date.'),
    notes: z.string().max(4000),
    stage_prices: z.array(stagePriceSchema),
    items: z
      .array(
        z.object({
          description: z.string().trim().min(1, 'Describe this box type.').max(160),
          dimensions_text: z.string().max(200),
          ordered_boxes: z
            .string()
            .regex(/^\d+$/, 'Enter whole boxes.')
            .refine((v) => Number(v) >= 1 && Number(v) <= 1_000_000, 'Enter 1–1,000,000 boxes.'),
        }),
      )
      .min(1)
      .max(100),
    template_ids: z.array(z.string()).min(1, 'Choose at least one stage.'),
  })
  .superRefine((v, ctx) => {
    let total = 0n;
    v.stage_prices.forEach((stage, index) => {
      if (!v.template_ids.includes(stage.template_id)) return;
      try {
        total += BigInt(stageTotal(stage, v.items) ?? '0');
      } catch {
        ctx.addIssue({
          code: 'custom',
          path: ['stage_prices', index, 'pricing_mode'],
          message: 'Complete this stage’s price and quantity using up to three decimals.',
        });
      }
    });
    if (total > MAX_MONEY)
      ctx.addIssue({
        code: 'custom',
        path: ['template_ids'],
        message: 'Combined stage prices exceed the supported range.',
      });
  });
type OrderForm = z.infer<typeof orderSchema>;
export function NewOrderDialog({
  bootstrap,
  orders,
  actor,
  onClose,
  onCreated,
}: {
  bootstrap: Bootstrap;
  orders: OrderSummary[];
  actor: string;
  onClose: () => void;
  onCreated: (id: string) => void;
}) {
  const namesId = useId();
  const templates = bootstrap.templates.filter((t) => !t.archived);
  const form = useForm<OrderForm>({
    resolver: zodResolver(orderSchema),
    defaultValues: {
      name: '',
      client_id: '',
      due_date: '',
      notes: '',
      stage_prices: templates.map((t) => emptyPrice(t.id)),
      items: [{ description: '', dimensions_text: '', ordered_boxes: '' }],
      template_ids: templates.map((t) => t.id),
    },
  });
  const { fields, append, remove } = useFieldArray({ control: form.control, name: 'items' });
  const values = form.watch();
  const write = useWrite(actor);
  const [localError, setLocalError] = useState<unknown>(null);
  const [isCloning, setIsCloning] = useState(false);
  const [cloneSource, setCloneSource] = useState('');
  const cloneRequest = useRef(0);
  const copyOrder = async (source: OrderSummary, request: number) => {
    setIsCloning(true);
    setLocalError(null);
    try {
      const detail = await repository.query<OrderDetail>({ kind: 'order', id: source.id });
      if (request !== cloneRequest.current) return;
      const activeStages = detail.stages.filter(
        (s) => !s.retired && templates.some((t) => t.id === s.template_id),
      );
      const hasStagePrices = detail.stage_prices.length > 0;
      const copiedPrices = templates.map((template) => {
        const original = detail.stage_prices.find((p) => p.template_id === template.id);
        if (original)
          return {
            template_id: template.id,
            pricing_mode: original.pricing_mode,
            fixed_total:
              original.fixed_total_fils === null ? '' : decimal(original.fixed_total_fils),
            quantity: original.quantity_scaled === null ? '' : decimal(original.quantity_scaled),
            unit_price: original.unit_price_fils === null ? '' : decimal(original.unit_price_fils),
            item_prices: detail.items.map((item) => {
              const rate = detail.stage_item_prices.find(
                (p) => p.stage_id === original.stage_id && p.item_id === item.id,
              );
              return rate ? decimal(rate.unit_price_fils) : '';
            }),
          };
        const price = emptyPrice(template.id);
        price.item_prices = detail.items.map(() => '');
        if (
          !hasStagePrices &&
          detail.order.quote_fils !== null &&
          activeStages.some((s) => s.template_id === template.id)
        ) {
          const first = activeStages[0].template_id === template.id;
          price.pricing_mode = first ? detail.order.pricing_mode : 'fixed';
          price.fixed_total = first ? decimal(detail.order.quote_fils) : '0.000';
          if (first && detail.order.pricing_mode === 'per_box')
            price.item_prices = detail.items.map((item) =>
              item.unit_price_fils === null ? '' : decimal(item.unit_price_fils),
            );
        }
        return price;
      });
      form.reset(
        {
          name: detail.order.name,
          client_id: bootstrap.clients.some((c) => c.id === detail.order.client_id && !c.archived)
            ? detail.order.client_id
            : '',
          due_date: detail.order.due_date,
          notes: detail.order.notes,
          stage_prices: copiedPrices,
          items: detail.items.map((item) => ({
            description: item.description,
            dimensions_text: item.dimensions_text,
            ordered_boxes: String(item.ordered_boxes),
          })),
          template_ids: activeStages.map((s) => s.template_id!),
        },
        { keepDefaultValues: true },
      );
      setCloneSource(
        `${source.number} · Review the copied due date, client and available stages before saving.${!hasStagePrices && detail.order.quote_fils !== null ? ' This older quote has no stage breakdown: its amount is placed on the first available stage, with zero on the others. Review this allocation.' : ''}`,
      );
    } catch (error) {
      if (request === cloneRequest.current) setLocalError(error);
    } finally {
      if (request === cloneRequest.current) setIsCloning(false);
    }
  };
  const close = () => {
    if (
      !write.isPending &&
      (!form.formState.isDirty || window.confirm('Discard this unsaved order?'))
    )
      onClose();
  };
  const selectedPrices = values.stage_prices.filter((stage) =>
    values.template_ids.includes(stage.template_id),
  );
  let subtotal = 0n;
  let incomplete = 0;
  let invalid = false;
  for (const stage of selectedPrices) {
    try {
      const total = stageTotal(stage, values.items);
      if (total === null) incomplete++;
      else subtotal += BigInt(total);
    } catch {
      invalid = true;
    }
  }
  const quote =
    selectedPrices.length && !incomplete && !invalid && subtotal <= MAX_MONEY
      ? subtotal.toString()
      : null;
  const submit = (v: OrderForm) => {
    if (isCloning || write.isPending) return;
    try {
      setLocalError(null);
      write.mutate(
        {
          kind: 'create_order',
          name: v.name,
          client_id: v.client_id,
          due_date: v.due_date,
          notes: v.notes,
          pricing_mode: 'unpriced',
          fixed_total_fils: null,
          items: v.items.map((i) => ({
            description: i.description,
            dimensions_text: i.dimensions_text,
            ordered_boxes: Number(i.ordered_boxes),
            unit_price_fils: null,
          })),
          template_ids: templates.filter((t) => v.template_ids.includes(t.id)).map((t) => t.id),
          stage_prices: v.stage_prices
            .filter((s) => v.template_ids.includes(s.template_id))
            .map((s) => ({
              template_id: s.template_id,
              pricing_mode: s.pricing_mode,
              fixed_total_fils: s.pricing_mode === 'fixed' ? parseScaled(s.fixed_total) : null,
              quantity_scaled:
                s.pricing_mode === 'per_unit' ? parseScaled(s.quantity, 1_000_000_000n) : null,
              unit_price_fils: s.pricing_mode === 'per_unit' ? parseScaled(s.unit_price) : null,
              item_prices_fils:
                s.pricing_mode === 'per_box'
                  ? v.items.map((_, i) => parseScaled(s.item_prices[i] ?? ''))
                  : [],
            })),
        },
        {
          onSuccess: (r) => {
            onCreated(r.data.id!);
            onClose();
          },
        },
      );
    } catch (e) {
      setLocalError(e);
    }
  };
  return (
    <Dialog
      title="Create a new order"
      subtitle="Different box types, one clear production flow."
      onClose={close}
      wide
    >
      <form onSubmit={form.handleSubmit(submit)}>
        <div className="field-row">
          <label>
            Order name
            <input
              autoFocus
              list={namesId}
              autoComplete="off"
              placeholder="e.g. Autumn retail packaging"
              {...form.register('name', {
                onChange: (e) => {
                  const request = ++cloneRequest.current;
                  const source = orders.find(
                    (o) => `${o.number} · ${o.name}` === e.target.value && !o.archived,
                  );
                  if (source) void copyOrder(source, request);
                  else setIsCloning(false);
                },
              })}
            />
            <datalist id={namesId}>
              {orders
                .filter((o) => !o.archived)
                .map((order) => (
                  <option
                    key={order.id}
                    value={`${order.number} · ${order.name}`}
                    label={order.client_name}
                  />
                ))}
            </datalist>
          </label>
          <label>
            Client
            <select {...form.register('client_id')}>
              <option value="">Choose a client</option>
              {bootstrap.clients
                .filter((c) => !c.archived)
                .map((c) => (
                  <option key={c.id} value={c.id}>
                    {c.name}
                  </option>
                ))}
            </select>
          </label>
        </div>
        <span className="field-error">
          {form.formState.errors.name?.message ?? form.formState.errors.client_id?.message}
        </span>
        <p className="form-note">
          Choose an existing order to copy its details, or type a new name. Saving creates a
          separate order with fresh stages.
        </p>
        {isCloning && (
          <p className="form-note" role="status">
            Copying order details…
          </p>
        )}
        {cloneSource && <p className="form-note">Copied from {cloneSource}</p>}
        <div className="field-row">
          <label>
            Due date
            <input type="date" {...form.register('due_date')} />
          </label>
        </div>
        <span className="field-error">{form.formState.errors.due_date?.message}</span>
        <div className="section-heading">
          <h3>Box types</h3>
          <button
            type="button"
            className="btn subtle"
            onClick={() => {
              values.stage_prices.forEach((stage, index) =>
                form.setValue(
                  `stage_prices.${index}.item_prices`,
                  [...values.items.map((_, i) => stage.item_prices[i] ?? ''), ''],
                  { shouldDirty: true },
                ),
              );
              append({ description: '', dimensions_text: '', ordered_boxes: '' });
            }}
          >
            <Plus size={14} />
            Add box type
          </button>
        </div>
        {fields.map((f, n) => (
          <div key={f.id} className="item-form">
            <div className="between">
              <strong>Box type {n + 1}</strong>
              <button
                type="button"
                className="icon-button"
                aria-label={`Remove box type ${n + 1}`}
                disabled={fields.length === 1}
                onClick={() => {
                  values.stage_prices.forEach((stage, index) =>
                    form.setValue(
                      `stage_prices.${index}.item_prices`,
                      values.items
                        .map((_, i) => stage.item_prices[i] ?? '')
                        .filter((_, i) => i !== n),
                      { shouldDirty: true },
                    ),
                  );
                  remove(n);
                }}
              >
                <Trash2 size={14} />
              </button>
            </div>
            <div className="field-row">
              <label>
                Description
                <input placeholder="Small gift box" {...form.register(`items.${n}.description`)} />
              </label>
              <label>
                Dimensions / size
                <input
                  placeholder="Enter as described"
                  {...form.register(`items.${n}.dimensions_text`)}
                />
              </label>
            </div>
            <div className="field-row">
              <label>
                Ordered boxes
                <input inputMode="numeric" {...form.register(`items.${n}.ordered_boxes`)} />
              </label>
            </div>
            <span className="field-error">
              {form.formState.errors.items?.[n]?.description?.message ??
                form.formState.errors.items?.[n]?.ordered_boxes?.message}
            </span>
          </div>
        ))}
        <h3>Production stages</h3>
        <p className="form-note">
          Select stages and enter each stage’s customer price. Their amounts add up to the order
          quote.
        </p>
        <div className="stage-pricing-list">
          {templates.map((t, n) => {
            const stage = values.stage_prices[n];
            let amount: string | null = null;
            try {
              amount = stageTotal(stage, values.items);
            } catch {
              /* Incomplete inputs stay editable. */
            }
            return (
              <fieldset
                className={`stage-price-card ${values.template_ids.includes(t.id) ? 'selected' : ''}`}
                key={t.id}
              >
                <legend>
                  <label className="check">
                    <input type="checkbox" value={t.id} {...form.register('template_ids')} />
                    <span>
                      {n + 1}. {t.name}
                    </span>
                  </label>
                </legend>
                {values.template_ids.includes(t.id) && (
                  <>
                    <div className="field-row">
                      <label>
                        Customer pricing · {t.name}
                        <select {...form.register(`stage_prices.${n}.pricing_mode`)}>
                          <option value="unpriced">Not priced yet</option>
                          <option value="fixed">Fixed total quote</option>
                          <option value="per_box">Price per box type</option>
                          <option value="per_unit">Price per Unit</option>
                        </select>
                      </label>
                      {stage.pricing_mode === 'fixed' && (
                        <label>
                          Stage fixed total · KWD
                          <input
                            inputMode="decimal"
                            placeholder="0.000"
                            {...form.register(`stage_prices.${n}.fixed_total`)}
                          />
                        </label>
                      )}
                      {stage.pricing_mode === 'per_unit' && (
                        <label>
                          Quantity · Unit
                          <input
                            inputMode="decimal"
                            placeholder="e.g. 100"
                            {...form.register(`stage_prices.${n}.quantity`)}
                          />
                        </label>
                      )}
                    </div>
                    {stage.pricing_mode === 'per_unit' && (
                      <label>
                        Selling price per Unit · KWD
                        <input
                          inputMode="decimal"
                          placeholder="0.000"
                          {...form.register(`stage_prices.${n}.unit_price`)}
                        />
                      </label>
                    )}
                    {stage.pricing_mode === 'per_box' && (
                      <div className="stage-box-prices">
                        {values.items.map((item, index) => (
                          <label key={fields[index].id}>
                            Selling price per box {index + 1} · KWD
                            <small>
                              {item.description || `Box type ${index + 1}`} ·{' '}
                              {item.ordered_boxes || '—'} boxes
                            </small>
                            <input
                              inputMode="decimal"
                              placeholder="0.000"
                              {...form.register(`stage_prices.${n}.item_prices.${index}`)}
                            />
                          </label>
                        ))}
                      </div>
                    )}
                    <span className="field-error">
                      {form.formState.errors.stage_prices?.[n]?.pricing_mode?.message}
                    </span>
                    <div className="stage-price-total">
                      <span>Stage customer amount</span>
                      <strong>{money(amount)}</strong>
                    </div>
                  </>
                )}
              </fieldset>
            );
          })}
        </div>
        <span className="field-error">{form.formState.errors.template_ids?.message}</span>
        <p className="form-note">
          Stage definitions are copied into this order. Material and work costs stay separate from
          the customer quote.
        </p>
        <label>
          Order notes
          <textarea {...form.register('notes')} />
        </label>
        <div className="quote-summary">
          <span>Customer quote</span>
          <strong>{money(quote)}</strong>
        </div>
        {quote === null && selectedPrices.length > 0 && (
          <p className="form-note" role="status">
            Known stage subtotal: {money(subtotal.toString())}.{' '}
            {incomplete
              ? `${incomplete} selected stage(s) still unpriced.`
              : 'Complete the selected stage inputs.'}{' '}
            The full customer quote is not priced until all selected stages have prices.
          </p>
        )}
        <ErrorNotice error={write.error ?? localError} />
        <div className="dialog-actions">
          <button type="button" className="btn" onClick={close}>
            Cancel
          </button>
          <button className="btn primary" disabled={write.isPending || isCloning}>
            {write.isPending ? 'Creating…' : 'Create order'}
          </button>
        </div>
      </form>
    </Dialog>
  );
}
