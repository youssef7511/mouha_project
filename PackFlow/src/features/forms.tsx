import { useState } from 'react';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { z } from 'zod';
import { Plus, Trash2 } from 'lucide-react';
import type { Bootstrap, Client, StageTemplate, Worker } from '../contracts';
import { useWrite } from '../data/hooks';
import { Dialog, ErrorNotice } from '../components/Dialog';

const name = z.string().trim().min(1, 'A name is required.').max(120);
const clientSchema = z.object({
  name,
  contact_name: z.string().max(120),
  email: z.union([z.literal(''), z.email()]),
  phone: z.string().max(60),
  address: z.string().max(1000),
  archived: z.boolean(),
});
type ClientForm = z.infer<typeof clientSchema>;
export function ClientDialog({
  actor,
  client,
  onClose,
}: {
  actor: string;
  client?: Client;
  onClose: () => void;
}) {
  const form = useForm<ClientForm>({
    resolver: zodResolver(clientSchema),
    defaultValues: {
      name: client?.name ?? '',
      contact_name: client?.contact_name ?? '',
      email: client?.email ?? '',
      phone: client?.phone ?? '',
      address: client?.address ?? '',
      archived: !!client?.archived,
    },
  });
  const write = useWrite(actor);
  const close = () => {
    if (
      !write.isPending &&
      (!form.formState.isDirty || window.confirm('Discard unsaved client changes?'))
    )
      onClose();
  };
  return (
    <Dialog
      title={client ? 'Client details' : 'Add a client'}
      subtitle="Customers and orders stay connected through a stable record."
      onClose={close}
    >
      <form
        onSubmit={form.handleSubmit((v) =>
          write.mutate(
            {
              kind: 'save_client',
              id: client?.id ?? null,
              expected_version: client?.version ?? null,
              ...v,
            },
            { onSuccess: onClose },
          ),
        )}
      >
        <label>
          Company name
          <input autoFocus {...form.register('name')} />
        </label>
        <span className="field-error">{form.formState.errors.name?.message}</span>
        <label>
          Contact name
          <input {...form.register('contact_name')} />
        </label>
        <div className="field-row">
          <label>
            Email
            <input type="email" {...form.register('email')} />
          </label>
          <label>
            Phone
            <input {...form.register('phone')} />
          </label>
        </div>
        <span className="field-error">{form.formState.errors.email?.message}</span>
        <label>
          Address
          <textarea {...form.register('address')} />
        </label>
        {client && (
          <label className="check">
            <input type="checkbox" {...form.register('archived')} />
            Archive client
          </label>
        )}
        <ErrorNotice error={write.error} />
        <div className="dialog-actions">
          <button type="button" className="btn" onClick={close}>
            Cancel
          </button>
          <button className="btn primary" disabled={write.isPending}>
            {write.isPending ? 'Saving…' : 'Save client'}
          </button>
        </div>
      </form>
    </Dialog>
  );
}
const templateSchema = z.object({
  name: name.max(80),
  default_notes: z.string().max(2000),
  archived: z.boolean(),
});
export function TemplateDialog({
  actor,
  template,
  onClose,
}: {
  actor: string;
  template?: StageTemplate;
  onClose: () => void;
}) {
  const form = useForm<z.infer<typeof templateSchema>>({
    resolver: zodResolver(templateSchema),
    defaultValues: {
      name: template?.name ?? '',
      default_notes: template?.default_notes ?? '',
      archived: !!template?.archived,
    },
  });
  const write = useWrite(actor);
  const close = () => {
    if (
      !write.isPending &&
      (!form.formState.isDirty || window.confirm('Discard unsaved stage changes?'))
    )
      onClose();
  };
  return (
    <Dialog
      title={template ? 'Edit production stage' : 'Add production stage'}
      subtitle="Template changes apply to new orders. Existing order stages keep their history."
      onClose={close}
    >
      <form
        onSubmit={form.handleSubmit((v) =>
          write.mutate(
            {
              kind: 'save_template',
              id: template?.id ?? null,
              expected_version: template?.version ?? null,
              ...v,
            },
            { onSuccess: onClose },
          ),
        )}
      >
        <label>
          Stage name
          <input autoFocus {...form.register('name')} />
        </label>
        <span className="field-error">{form.formState.errors.name?.message}</span>
        <label>
          Default instructions
          <textarea {...form.register('default_notes')} />
        </label>
        {template && (
          <label className="check">
            <input type="checkbox" {...form.register('archived')} />
            Archive this template
          </label>
        )}
        <ErrorNotice error={write.error} />
        <div className="dialog-actions">
          <button type="button" className="btn" onClick={close}>
            Cancel
          </button>
          <button className="btn primary" disabled={write.isPending}>
            Save stage
          </button>
        </div>
      </form>
    </Dialog>
  );
}
const workerSchema = z.object({ name, role: z.string().max(80), active: z.boolean() });
export function WorkerDialog({
  actor,
  worker,
  onClose,
}: {
  actor: string;
  worker?: Worker;
  onClose: () => void;
}) {
  const form = useForm<z.infer<typeof workerSchema>>({
    resolver: zodResolver(workerSchema),
    defaultValues: {
      name: worker?.name ?? '',
      role: worker?.role ?? '',
      active: worker ? !!worker.active : true,
    },
  });
  const write = useWrite(actor);
  const close = () => {
    if (
      !write.isPending &&
      (!form.formState.isDirty || window.confirm('Discard unsaved worker changes?'))
    )
      onClose();
  };
  return (
    <Dialog
      title={worker ? 'Worker details' : 'Add a worker'}
      subtitle="Profiles and work records. Salary management will come later."
      onClose={close}
    >
      <form
        onSubmit={form.handleSubmit((v) =>
          write.mutate(
            {
              kind: 'save_worker',
              id: worker?.id ?? null,
              expected_version: worker?.version ?? null,
              ...v,
            },
            { onSuccess: onClose },
          ),
        )}
      >
        <label>
          Full name
          <input autoFocus {...form.register('name')} />
        </label>
        <span className="field-error">{form.formState.errors.name?.message}</span>
        <label>
          Role
          <input {...form.register('role')} />
        </label>
        <label className="check">
          <input type="checkbox" {...form.register('active')} />
          Active worker
        </label>
        <ErrorNotice error={write.error} />
        <div className="dialog-actions">
          <button type="button" className="btn" onClick={close}>
            Cancel
          </button>
          <button className="btn primary" disabled={write.isPending}>
            Save worker
          </button>
        </div>
      </form>
    </Dialog>
  );
}

export { NewOrderDialog } from './NewOrderDialog';
