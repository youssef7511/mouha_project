import { useState } from 'react';
import { useWrite } from '../data/hooks';
import { ErrorNotice } from '../components/Dialog';

export function Correction({
  actor,
  id,
  type,
  today,
  reversed,
  label,
}: {
  actor: string;
  id: string;
  type: 'expense' | 'work' | 'payment' | 'service' | 'separate_cost';
  today: string;
  reversed?: number;
  label: string;
}) {
  const [open, setOpen] = useState(false);
  const [reason, setReason] = useState('');
  const [date, setDate] = useState(today);
  const write = useWrite(actor);
  if (reversed) return <small>Reversed · original retained</small>;
  if (!open)
    return (
      <button type="button" className="btn" onClick={() => setOpen(true)}>
        Correct entry
      </button>
    );
  return (
    <form
      className="correction-form"
      onSubmit={async (e) => {
        e.preventDefault();
        if (write.isPending) return;
        try {
          await write.mutateAsync({
            kind: 'reverse_entry',
            entry_type: type,
            id,
            reason,
            business_date: date,
          });
          setOpen(false);
        } catch {
          /* mutation error is shown below */
        }
      }}
    >
      <strong>Reverse {label}</strong>
      <p>
        The original stays in history. Its effect is removed on this date. You can then enter the
        correct replacement.
      </p>
      {type === 'service' && <p>Both the customer charge and supplier cost will be reversed.</p>}
      {type === 'payment' && (
        <p>This offsets the recorded receipt; it does not transfer money to the customer.</p>
      )}
      <label>
        Correction date
        <input
          type="date"
          required
          value={date}
          disabled={write.isPending}
          onChange={(e) => setDate(e.target.value)}
        />
      </label>
      <label>
        Reason
        <textarea
          required
          maxLength={1000}
          value={reason}
          disabled={write.isPending}
          onChange={(e) => setReason(e.target.value)}
        />
      </label>
      <ErrorNotice error={write.error} />
      <button
        className="btn"
        type="button"
        disabled={write.isPending}
        onClick={() => setOpen(false)}
      >
        Cancel correction
      </button>
      <button className="btn primary" disabled={write.isPending || !reason.trim()}>
        {write.isPending ? 'Saving…' : 'Confirm reversal'}
      </button>
    </form>
  );
}
