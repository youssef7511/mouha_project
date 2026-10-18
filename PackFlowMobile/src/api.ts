import { invoke, isTauri } from '@tauri-apps/api/core';
import type { Operation, Snapshot, SaveResult } from './types';
export function message(e: unknown): string {
  return e instanceof Error
    ? e.message
    : typeof e === 'string'
      ? e
      : e && typeof e === 'object' && 'message' in e
        ? String(e.message)
        : 'Could not complete this local operation.';
}
async function command<T>(name: 'query' | 'mutate', request: unknown): Promise<T> {
  try {
    if (isTauri()) return await invoke<T>(`mobile_${name}`, { request });
    if (!import.meta.env.DEV) throw new Error('Open PackFlow Mobile as an installed app.');
    const r = await fetch(`/api/${name}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    });
    const data = await r.json();
    if (!r.ok) throw data;
    return data as T;
  } catch (e) {
    throw new Error(message(e));
  }
}
export const api = {
  query: <T>(request: unknown) => command<T>('query', request),
  snapshot: () => command<Snapshot>('query', { kind: 'snapshot' }),
  mutate: (request: Operation) => command<SaveResult>('mutate', request),
};
export async function downloadBackup(name?: string) {
  if (isTauri()) {
    if (name) throw new Error('Open saved backups using the Files app.');
    const r = await invoke<{ path: string }>('mobile_backup');
    return `Saved in Files → On My iPhone → PackFlow Mobile → PackFlow Mobile Backups. (${r.path.split('/').at(-1)})`;
  }
  const r = await fetch('/api/backup', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ name }),
  });
  if (!r.ok) throw new Error(message(await r.json()));
  const url = URL.createObjectURL(await r.blob());
  const a = document.createElement('a');
  a.href = url;
  a.download =
    r.headers.get('Content-Disposition')?.match(/filename="([^"]+)"/)?.[1] ??
    'PackFlow-Mobile.pfmobile';
  document.body.append(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 60000);
  return 'Mobile backup downloaded.';
}
export async function restoreBackup(file: File, confirmation: string) {
  if (file.size > 25 * 1024 * 1024 + 100) throw new Error('Mobile backups are limited to 25 MB.');
  const data = await new Promise<string>((resolve, reject) => {
    const reader = new FileReader();
    reader.onerror = () => reject(new Error('Could not read this file.'));
    reader.onload = () => resolve(String(reader.result).split(',')[1]);
    reader.readAsDataURL(file);
  });
  try {
    if (isTauri())
      return await invoke<{ safety_backup: string; message: string }>('mobile_restore', {
        data,
        confirmation,
      });
    const r = await fetch('/api/restore', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ data, confirmation }),
    });
    const v = await r.json();
    if (!r.ok) throw v;
    return v as { safety_backup: string; message: string };
  } catch (e) {
    throw new Error(message(e));
  }
}
