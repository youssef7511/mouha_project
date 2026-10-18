import { useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { isTauri } from '@tauri-apps/api/core';
import { Upload } from 'lucide-react';
import { Dialog, ErrorNotice } from '../components/Dialog';
import { nativeCommand } from './LocalAccount';
import { money } from '../contracts/money';
interface Row {
  entity: string;
  source_id: string;
  label: string;
  status: string;
  message: string;
  target_id: string;
  amount_fils?: string | null;
  payment_kind?: string;
  method?: string;
  reference?: string;
  source_order?: string;
  source_number?: string;
}
interface Plan {
  rows: Row[];
  ready: number;
  can_apply: boolean;
  preview_token?: string;
  workspace_id: string;
  from_date: string;
  to_date: string;
  clients: { id: string; name: string; phone: string }[];
  mac_clients: { id: string; name: string; phone: string }[];
  safety_backup?: string;
  import_id?: string;
}
async function call(request: unknown): Promise<Plan> {
  if (isTauri()) return nativeCommand<Plan>('phone_import', { request });
  const r = await fetch('/api/phone-import', {
    method: 'POST',
    credentials: 'same-origin',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(request),
  });
  const v = await r.json();
  if (!r.ok) {
    if (v.code === 'UNAUTHENTICATED') window.dispatchEvent(new Event('packflow-session-ended'));
    throw new Error(v.message ?? 'Could not import phone records.');
  }
  return v as Plan;
}
export function PhoneImportButton({ disabled = false }: { disabled?: boolean }) {
  const [open, setOpen] = useState(false);
  return (
    <>
      <button className="btn" disabled={disabled} onClick={() => setOpen(true)}>
        <Upload size={15} />
        Import from phone
      </button>
      {open && <PhoneImport close={() => setOpen(false)} />}
    </>
  );
}
function PhoneImport({ close }: { close: () => void }) {
  const cache = useQueryClient();
  const [data, setData] = useState('');
  const [links, setLinks] = useState<Record<string, string>>({});
  const [plan, setPlan] = useState<Plan | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [busy, setBusy] = useState(false);
  const [reviewed, setReviewed] = useState(false);
  const [mappingPage, setMappingPage] = useState(0);
  const [clientSearch, setClientSearch] = useState('');
  const [rowLimit, setRowLimit] = useState(100);
  const [issuesOnly, setIssuesOnly] = useState(false);
  const execute = async (apply: boolean) => {
    if (busy) return;
    setBusy(true);
    setError(null);
    try {
      const result = await call({
        data,
        client_links: links,
        preview_token: apply ? plan?.preview_token : null,
      });
      setPlan(result);
      setReviewed(false);
      if (apply) await cache.invalidateQueries();
    } catch (e) {
      setError(e);
      setPlan((p) => (p ? { ...p, preview_token: undefined, can_apply: false } : null));
    } finally {
      setBusy(false);
    }
  };
  const mappings =
    plan?.rows.filter(
      (r) => r.entity === 'client' && (!r.target_id || links[r.source_id] !== undefined),
    ) ?? [];
  const conflicts =
    plan?.rows.filter((r) => r.status === 'conflict' || r.status === 'blocked').length ?? 0;
  const visibleRows =
    plan?.rows.filter(
      (r) => !issuesOnly || ['conflict', 'blocked', 'needs_mapping'].includes(r.status),
    ) ?? [];
  const suggestions =
    plan?.mac_clients
      .filter((c) => `${c.name} ${c.phone}`.toLowerCase().includes(clientSearch.toLowerCase()))
      .slice(0, 50) ?? [];
  return (
    <Dialog
      title="Import phone Excel"
      subtitle="Clients, orders, stage pricing and payments are linked and imported together."
      wide
      onClose={() => {
        if (!busy) close();
      }}
    >
      <div className="form-stack">
        <p>
          Choose an unchanged transfer workbook exported from the phone. Reporting exports and
          ordinary spreadsheets are not transfer files. Mac order numbers are assigned
          automatically; existing production work is retained.
        </p>
        <label>
          Phone transfer workbook
          <input
            type="file"
            accept=".xlsx"
            disabled={busy}
            onChange={async (e) => {
              setError(null);
              setPlan(null);
              setData('');
              setLinks({});
              setReviewed(false);
              setMappingPage(0);
              setRowLimit(100);
              setIssuesOnly(false);
              setClientSearch('');
              const f = e.target.files?.[0];
              if (!f) return;
              if (f.size > 8 * 1024 * 1024) {
                setError(new Error('Transfer files are limited to 8 MB.'));
                return;
              }
              setBusy(true);
              try {
                const encoded = await new Promise<string>((resolve, reject) => {
                  const r = new FileReader();
                  r.onload = () => resolve(String(r.result).split(',')[1]);
                  r.onerror = () => reject(new Error('Could not read the selected workbook.'));
                  r.readAsDataURL(f);
                });
                setData(encoded);
                setPlan(await call({ data: encoded, client_links: {}, preview_token: null }));
              } catch (e) {
                setError(e);
              } finally {
                setBusy(false);
              }
            }}
          />
        </label>
        <ErrorNotice error={error} />
        {plan && (
          <>
            <p className="form-note">
              Phone changes: {plan.from_date} through {plan.to_date}. Related older records are
              included. Phone archives/status are retained as source information; they do not cancel
              or archive Mac orders.
            </p>
            {mappings.length > 0 && !plan.import_id && (
              <section>
                <h3>Match phone clients</h3>
                <p>
                  Choose an existing Mac contact or create a new one. Matching a contact keeps its
                  Mac details.
                </p>
                <label>
                  Search existing Mac clients
                  <input
                    value={clientSearch}
                    onChange={(e) => setClientSearch(e.target.value)}
                    disabled={busy}
                    placeholder="Name or phone (up to 50 matches)"
                  />
                </label>
                {mappings.slice(mappingPage * 20, (mappingPage + 1) * 20).map((r) => {
                  const c = plan.clients.find((c) => c.id === r.source_id)!;
                  const selected = plan.mac_clients.find((m) => m.id === links[c.id]);
                  const options =
                    selected && !suggestions.some((m) => m.id === selected.id)
                      ? [selected, ...suggestions]
                      : suggestions;
                  return (
                    <label key={r.source_id}>
                      {c.name}
                      {c.phone ? ` · ${c.phone}` : ''}
                      <select
                        aria-label={`Match ${c.name}`}
                        disabled={busy}
                        value={links[c.id] ?? ''}
                        onChange={(e) => {
                          const target = e.target.value;
                          setLinks((v) => ({ ...v, [c.id]: target }));
                          setReviewed(false);
                          setPlan((v) =>
                            v ? { ...v, preview_token: undefined, can_apply: false } : null,
                          );
                        }}
                      >
                        <option value="">Choose a client…</option>
                        <option value="new">Create a new Mac client</option>
                        {options.map((m) => (
                          <option key={m.id} value={m.id}>
                            {m.name}
                            {m.phone ? ` · ${m.phone}` : ''}
                          </option>
                        ))}
                      </select>
                    </label>
                  );
                })}
                {mappings.length > 20 && (
                  <div className="dialog-actions">
                    <button
                      className="btn"
                      disabled={busy || mappingPage === 0}
                      onClick={() => setMappingPage((p) => p - 1)}
                    >
                      Previous clients
                    </button>
                    <span>
                      {mappingPage * 20 + 1}–{Math.min((mappingPage + 1) * 20, mappings.length)} of{' '}
                      {mappings.length}
                    </span>
                    <button
                      className="btn"
                      disabled={busy || (mappingPage + 1) * 20 >= mappings.length}
                      onClick={() => setMappingPage((p) => p + 1)}
                    >
                      Next clients
                    </button>
                  </div>
                )}
              </section>
            )}
            <button
              className="btn"
              aria-pressed={issuesOnly}
              onClick={() => {
                setIssuesOnly((v) => !v);
                setRowLimit(100);
              }}
            >
              {issuesOnly ? 'Show all records' : 'Show conflicts and blocked records'}
            </button>
            <div className="table-wrap" style={{ maxHeight: 360, overflow: 'auto' }}>
              <table>
                <thead>
                  <tr>
                    <th>Record</th>
                    <th>Status</th>
                    <th>Result</th>
                  </tr>
                </thead>
                <tbody>
                  {visibleRows.slice(0, rowLimit).map((r, i) => (
                    <tr key={`${r.entity}-${r.source_id}-${i}`}>
                      <td>
                        {r.entity} · {r.label}
                        {r.entity === 'payment' && (
                          <div>
                            {r.payment_kind === 'correction' ? 'Reverse ' : ''}
                            {money(r.amount_fils ?? null)} · {r.method} · {r.reference}
                            <small style={{ display: 'block' }}>{r.source_order}</small>
                          </div>
                        )}
                        <small className="form-note" style={{ display: 'block' }}>
                          {r.source_number}
                        </small>
                      </td>
                      <td>{r.status}</td>
                      <td>{r.message}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <p className="form-note">
              Showing {Math.min(rowLimit, visibleRows.length)} of {visibleRows.length} rows.
            </p>
            {rowLimit < visibleRows.length && (
              <button className="btn" onClick={() => setRowLimit((n) => n + 100)}>
                Show 100 more records
              </button>
            )}
            {!plan.import_id && (
              <>
                <p>
                  {plan.ready} ready records. {conflicts} conflicts or blocked records will remain
                  unchanged.
                </p>
                <label className="check">
                  <input
                    type="checkbox"
                    disabled={busy || !plan.can_apply || !plan.preview_token}
                    checked={reviewed}
                    onChange={(e) => setReviewed(e.target.checked)}
                  />
                  I reviewed the matches and results. Import ready records and leave conflicts
                  unchanged.
                </label>
              </>
            )}
            {plan.import_id && (
              <div role="status">
                <strong>Import completed.</strong>
                <p>Safety backup: {plan.safety_backup}</p>
                <p>
                  Repeated files will not duplicate these imported records. Mac changes are not sent
                  back to the phone.
                </p>
              </div>
            )}
          </>
        )}
        <div className="dialog-actions">
          <button className="btn" disabled={busy} onClick={close}>
            {plan?.import_id ? 'Done' : 'Cancel'}
          </button>
          {!plan?.import_id && (
            <>
              <button className="btn" disabled={busy || !data} onClick={() => void execute(false)}>
                {busy ? 'Checking…' : 'Preview import'}
              </button>
              <button
                className="btn primary"
                disabled={busy || !plan?.can_apply || !plan.preview_token || !reviewed}
                onClick={() => void execute(true)}
              >
                {busy ? 'Working…' : 'Import ready records'}
              </button>
            </>
          )}
        </div>
      </div>
    </Dialog>
  );
}
