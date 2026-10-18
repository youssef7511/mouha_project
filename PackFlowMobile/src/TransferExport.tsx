import { useState } from 'react';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { ErrorLine } from './Sheet';
export function TransferExport({ today, disabled }: { today: string; disabled: boolean }) {
  const [from, setFrom] = useState(today.slice(0, 8) + '01');
  const [to, setTo] = useState(today);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState('');
  const [error, setError] = useState<unknown>(null);
  return (
    <section className="settings-card">
      <h2>Transfer to Mac</h2>
      <p>
        Export records created or changed during this period. Related clients, full order details,
        receipts and corrections are included automatically. Import the unchanged file on the Mac.
      </p>
      <form
        className="form-grid"
        onSubmit={async (e) => {
          e.preventDefault();
          if (busy) return;
          setBusy(true);
          setError(null);
          setNotice('');
          try {
            if (isTauri()) {
              const r = await invoke<{ name: string }>('mobile_transfer', {
                fromDate: from,
                toDate: to,
              });
              setNotice(
                `Saved ${r.name}. Open Files → On My iPhone → PackFlow Mobile → PackFlow Mobile Backups and share it to the Mac.`,
              );
            } else {
              const r = await fetch('/api/transfer', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ from_date: from, to_date: to }),
              });
              if (!r.ok) throw await r.json();
              const u = URL.createObjectURL(await r.blob());
              const a = document.createElement('a');
              a.href = u;
              a.download = `PackFlow-transfer-${from}-${to}.xlsx`;
              document.body.append(a);
              a.click();
              a.remove();
              setTimeout(() => URL.revokeObjectURL(u), 60000);
              setNotice('Transfer Excel downloaded. Open Import from phone on the Mac.');
            }
          } catch (e) {
            setError(e);
          } finally {
            setBusy(false);
          }
        }}
      >
        <label>
          Changed from
          <input
            type="date"
            required
            value={from}
            max={to}
            onChange={(e) => setFrom(e.target.value)}
          />
        </label>
        <label>
          Changed through
          <input
            type="date"
            required
            value={to}
            min={from}
            onChange={(e) => setTo(e.target.value)}
          />
        </label>
        <p className="hint">
          Dates use Kuwait time. Older related records may be included. Reimporting the same records
          will not duplicate them.
        </p>
        <button className="button primary full" disabled={disabled || busy}>
          {busy ? 'Exporting…' : 'Export transfer Excel'}
        </button>
      </form>
      <ErrorLine error={error} />
      {notice && <p role="status">{notice}</p>}
      <p className="hint">
        This is a one-way transfer. Mac edits do not return to this phone; its balance remains
        independent.
      </p>
    </section>
  );
}
