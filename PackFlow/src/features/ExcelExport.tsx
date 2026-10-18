import { createContext, useContext, useEffect, useState } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import { Download } from 'lucide-react';
import { nativeCommand } from './LocalAccount';

export type ExportPage =
  | 'dashboard'
  | 'orders'
  | 'clients'
  | 'workers'
  | 'costs'
  | 'payments'
  | 'reports'
  | 'settings'
  | 'stock';
export interface ExportRequest {
  page: ExportPage;
  search?: string;
  from_date?: string | null;
  to_date?: string | null;
  month?: string | null;
  order_id?: string | null;
}
export const ExcelFilters = createContext<(request: ExportRequest) => void>(() => {});
export function useExcelFilters(request: ExportRequest) {
  const publish = useContext(ExcelFilters);
  const serialized = JSON.stringify(request);
  useEffect(() => {
    publish(JSON.parse(serialized) as ExportRequest);
  }, [publish, serialized]);
}

export function ExcelExportButton({
  request,
  disabled = false,
}: {
  request: ExportRequest;
  disabled?: boolean;
}) {
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState('');
  const [error, setError] = useState('');
  return (
    <div className="excel-export">
      <button
        className="btn"
        disabled={disabled || busy}
        onClick={async () => {
          if (busy) return;
          setBusy(true);
          setMessage('');
          setError('');
          try {
            if (isTauri()) {
              const result = await nativeCommand<{ cancelled: boolean; path?: string }>(
                'workspace_export',
                { request },
              );
              if (!result.cancelled) setMessage(`Excel saved: ${result.path}`);
            } else {
              const response = await fetch('/api/export', {
                method: 'POST',
                credentials: 'same-origin',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(request),
              });
              if (!response.ok) {
                const failure = await response.json();
                if (failure.code === 'UNAUTHENTICATED')
                  window.dispatchEvent(new Event('packflow-session-ended'));
                throw new Error(failure.message ?? 'Could not export Excel.');
              }
              const url = URL.createObjectURL(await response.blob());
              const link = document.createElement('a');
              link.href = url;
              link.download =
                response.headers.get('Content-Disposition')?.match(/filename="([^"]+)"/)?.[1] ??
                `PackFlow-${request.page}.xlsx`;
              document.body.append(link);
              link.click();
              link.remove();
              window.setTimeout(() => URL.revokeObjectURL(url), 60_000);
              setMessage('Excel download ready.');
            }
          } catch (e) {
            setError(e instanceof Error ? e.message : 'Could not export Excel.');
          } finally {
            setBusy(false);
          }
        }}
      >
        <Download size={15} />
        {busy ? 'Exporting…' : 'Export Excel'}
      </button>
      {message && (
        <span className="form-note" role="status">
          {message}
        </span>
      )}
      {error && (
        <span className="error" role="alert">
          {error}
        </span>
      )}
    </div>
  );
}
