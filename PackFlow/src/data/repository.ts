import { invoke, isTauri } from '@tauri-apps/api/core';
import type { Mutation, MutationResult, Query } from '../contracts';

export class DomainError extends Error {
  constructor(
    public code: string,
    message: string,
    public field?: string,
  ) {
    super(message);
    this.name = 'DomainError';
  }
}
function normalize(error: unknown): DomainError {
  if (error instanceof DomainError) return error;
  if (error && typeof error === 'object' && 'message' in error) {
    const e = error as { code?: string; message: string; field?: string };
    return new DomainError(e.code ?? 'STORAGE', e.message, e.field);
  }
  return new DomainError(
    'STORAGE',
    'Could not reach PackFlow. Check that the local core is running.',
  );
}
async function command<T>(name: 'query' | 'mutate', request: Query | Mutation): Promise<T> {
  try {
    if (isTauri()) return await invoke<T>(name, { request });
    if (!import.meta.env.DEV && import.meta.env.VITE_ENABLE_PREVIEW !== 'true') {
      throw new DomainError(
        'DESKTOP_REQUIRED',
        'Open PackFlow as a desktop app. Browser mode is only available for development.',
      );
    }
    const response = await fetch(`/api/${name}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    });
    const body = await response.json();
    if (!response.ok) throw normalize(body);
    return body as T;
  } catch (error) {
    const normalized = normalize(error);
    if (normalized.code === 'UNAUTHENTICATED')
      window.dispatchEvent(new Event('packflow-session-ended'));
    throw normalized;
  }
}
export const repository = {
  query: <T>(request: Query) => command<T>('query', request),
  mutate: (request: Mutation) => command<MutationResult>('mutate', request),
};
