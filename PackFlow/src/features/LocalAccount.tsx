import { createContext, useContext, useEffect, useState, type ReactNode } from 'react';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { useQueryClient } from '@tanstack/react-query';
import { ErrorNotice } from '../components/Dialog';

type Status = {
  configured: boolean;
  unlocked: boolean;
  display_name: string;
  recovery_code?: string;
  warning?: string;
};
type Request = { kind: string; [key: string]: string | boolean };
export const AccountContext = createContext<{
  run: (request: Request) => Promise<void>;
  lock: (notice: string) => void;
}>({ run: async () => {}, lock: () => {} });
export async function nativeCommand<T>(name: string, args?: Record<string, unknown>): Promise<T> {
  try {
    if (isTauri()) return await invoke<T>(name, args);
    if (
      name !== 'auth' ||
      (!import.meta.env.DEV && import.meta.env.VITE_ENABLE_PREVIEW !== 'true')
    ) {
      throw new Error('This operation requires the desktop application.');
    }
    const response = await fetch('/api/auth', {
      method: 'POST',
      credentials: 'same-origin',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(args?.request),
    });
    const result = await response.json();
    if (!response.ok) throw new Error(result.message ?? 'Could not sign in.');
    return result as T;
  } catch (e) {
    throw new Error(
      typeof e === 'string'
        ? e
        : typeof e === 'object' && e && 'message' in e
          ? String(e.message)
          : 'Could not complete the local operation.',
    );
  }
}
export function LocalAccount({ children }: { children: ReactNode }) {
  const cache = useQueryClient();
  const [status, setStatus] = useState<Status | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [busy, setBusy] = useState(false);
  const [recover, setRecover] = useState(false);
  const [notice, setNotice] = useState('');
  const [username, setUsername] = useState('');
  const [name, setName] = useState('');
  const [password, setPassword] = useState('');
  const [confirm, setConfirm] = useState('');
  const [code, setCode] = useState('');
  const [remember, setRemember] = useState(false);
  const run = async (request: Request) => {
    const result = await nativeCommand<Status>('auth', { request });
    await cache.cancelQueries();
    cache.clear();
    setStatus(result);
    setPassword('');
    setConfirm('');
    setCode('');
    setNotice(result.warning ?? '');
  };
  useEffect(() => {
    nativeCommand<Status>('auth', { request: { kind: 'status' } })
      .then(setStatus)
      .catch(setError);
  }, []);
  useEffect(() => {
    const expired = () => {
      cache.clear();
      setStatus((s) => (s ? { ...s, unlocked: false } : s));
      setNotice('Your session ended. Please sign in again.');
    };
    window.addEventListener('packflow-session-ended', expired);
    return () => window.removeEventListener('packflow-session-ended', expired);
  }, [cache]);
  const lock = (message: string) => {
    cache.clear();
    setStatus((s) => (s ? { ...s, unlocked: false } : null));
    setNotice(message);
  };
  if (status?.unlocked)
    return (
      <AccountContext.Provider value={{ run, lock }}>
        {notice && (
          <p role="status" className="account-notice">
            {notice}
          </p>
        )}
        <div className="account-bar">
          <span>
            Signed in as <strong>{status.display_name}</strong>
            {!isTauri() && ' · Local web preview'}
          </span>
          <button
            className="btn"
            disabled={busy}
            onClick={async () => {
              if (busy) return;
              setBusy(true);
              setError(null);
              try {
                await run({ kind: 'logout' });
              } catch (e) {
                setNotice(e instanceof Error ? e.message : 'Could not log out.');
              } finally {
                setBusy(false);
              }
            }}
          >
            Log out
          </button>
        </div>
        {children}
      </AccountContext.Provider>
    );
  const setup = status && !status.configured;
  return (
    <main className="account-screen">
      <section className="panel account-card">
        <h1>PackFlow</h1>
        <p>
          {isTauri()
            ? 'Your workspace, stored on this Mac.'
            : 'Local web preview · Create a preview account to test login. Your existing records stay unchanged.'}
        </p>
        {notice && <p role="status">{notice}</p>}
        <ErrorNotice error={error} />
        {!status ? (
          <>
            <p>Opening local account…</p>
            <button className="btn" onClick={() => window.location.reload()}>
              Retry
            </button>
          </>
        ) : status.recovery_code ? (
          <>
            <h2>Save your recovery code</h2>
            <p>
              Keep this code somewhere safe, separate from this device. It lets you reset your
              password offline. A password reset creates a new code.
            </p>
            <textarea aria-label="Recovery code" readOnly value={status.recovery_code} />
            <button
              className="btn primary"
              onClick={() => {
                setStatus({ ...status, recovery_code: undefined });
                setRecover(false);
              }}
            >
              I saved it — continue to sign in
            </button>
          </>
        ) : (
          <form
            onSubmit={async (e) => {
              e.preventDefault();
              if (busy) return;
              setError(null);
              if ((setup || recover) && password !== confirm) {
                setError(new Error('Passwords do not match.'));
                return;
              }
              setBusy(true);
              try {
                await run(
                  setup
                    ? { kind: 'setup', username, display_name: name, password }
                    : recover
                      ? { kind: 'recover', code, password }
                      : { kind: 'login', username, password, remember },
                );
              } catch (err) {
                setError(err);
              } finally {
                setBusy(false);
              }
            }}
          >
            <h2>
              {setup ? 'Create your account' : recover ? 'Reset your password' : 'Welcome back'}
            </h2>
            <fieldset disabled={busy}>
              {setup && (
                <label>
                  Your name
                  <input
                    required
                    maxLength={100}
                    autoComplete="name"
                    value={name}
                    onChange={(e) => setName(e.target.value)}
                  />
                </label>
              )}
              {!recover && (
                <label>
                  Username
                  <input
                    required
                    maxLength={100}
                    autoComplete="username"
                    value={username}
                    onChange={(e) => setUsername(e.target.value)}
                  />
                </label>
              )}
              {recover && (
                <label>
                  Recovery code
                  <input
                    required
                    maxLength={64}
                    autoComplete="off"
                    value={code}
                    onChange={(e) => setCode(e.target.value)}
                  />
                </label>
              )}
              <label>
                {setup || recover ? 'New password (at least 10 characters)' : 'Password'}
                <input
                  required
                  type="password"
                  minLength={setup || recover ? 10 : 1}
                  maxLength={1024}
                  autoComplete={setup || recover ? 'new-password' : 'current-password'}
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                />
              </label>
              {(setup || recover) && (
                <label>
                  Confirm password
                  <input
                    required
                    type="password"
                    autoComplete="new-password"
                    value={confirm}
                    onChange={(e) => setConfirm(e.target.value)}
                  />
                </label>
              )}
              {!setup && !recover && (
                <>
                  <label className="remember">
                    <input
                      type="checkbox"
                      checked={remember}
                      onChange={(e) => setRemember(e.target.checked)}
                    />
                    Remember me on this {isTauri() ? 'Mac' : 'browser'}
                  </label>
                  <p>
                    {isTauri()
                      ? 'Keep this workspace signed in after quitting, until you log out or reset your password.'
                      : 'Stay signed in on this browser for up to 30 days, or until you log out or reset your password.'}
                  </p>
                </>
              )}
              <button className="btn primary">
                {busy
                  ? 'Please wait…'
                  : setup
                    ? 'Create account'
                    : recover
                      ? 'Reset password'
                      : 'Sign in'}
              </button>
              {!setup && (
                <button
                  className="btn"
                  type="button"
                  onClick={() => {
                    setRecover(!recover);
                    setError(null);
                    setPassword('');
                    setConfirm('');
                  }}
                >
                  {recover ? 'Back to sign in' : 'Forgot password?'}
                </button>
              )}
            </fieldset>
          </form>
        )}
      </section>
    </main>
  );
}

export function WorkspaceSafety() {
  const account = useContext(AccountContext);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<unknown>(null);
  const [message, setMessage] = useState('');
  const [confirmation, setConfirmation] = useState('');
  const [current, setCurrent] = useState('');
  const [password, setPassword] = useState('');
  const [confirm, setConfirm] = useState('');
  const perform = async (fn: () => Promise<void>) => {
    if (busy) return;
    setBusy(true);
    setError(null);
    try {
      await fn();
    } catch (e) {
      setError(e);
    } finally {
      setBusy(false);
    }
  };
  return (
    <section className="panel safety-panel">
      <h2>Backup, restore & account</h2>
      <p>
        Save a verified backup to an external drive or another safe location. Backups contain
        business records, without this Mac’s password or remembered login.
      </p>
      {!isTauri() && (
        <p>
          This browser uses a separate local preview account. Backup and restore file dialogs remain
          desktop-only.
        </p>
      )}
      <>
        <ErrorNotice error={error} />
        {message && <p role="status">{message}</p>}
        <fieldset disabled={busy}>
          {isTauri() && (
            <>
              <button
                className="btn primary"
                onClick={() =>
                  perform(async () => {
                    const r = await nativeCommand<{ path?: string; cancelled?: boolean }>(
                      'workspace_backup',
                    );
                    if (!r.cancelled) setMessage(`Verified backup saved: ${r.path}`);
                  })
                }
              >
                Save backup…
              </button>
              <h3>Restore a backup</h3>
              <p>
                Restore replaces the current business records. A safety backup is saved first. Your
                current account stays on this Mac, and you will sign in again.
              </p>
              <label>
                Type RESTORE to confirm
                <input
                  value={confirmation}
                  onChange={(e) => setConfirmation(e.target.value)}
                  autoComplete="off"
                />
              </label>
              <button
                className="btn"
                disabled={confirmation !== 'RESTORE'}
                onClick={() =>
                  perform(async () => {
                    const r = await nativeCommand<{ cancelled?: boolean; safety_backup?: string }>(
                      'workspace_restore',
                      { confirmation },
                    );
                    if (!r.cancelled)
                      account.lock(
                        `Workspace restored. Previous workspace backup: ${r.safety_backup}`,
                      );
                  })
                }
              >
                Choose backup and restore…
              </button>
            </>
          )}
          <h3>Change password</h3>
          <form
            onSubmit={(e) => {
              e.preventDefault();
              perform(async () => {
                if (password !== confirm) throw new Error('Passwords do not match.');
                await account.run({ kind: 'change_password', current, password });
              });
            }}
          >
            <label>
              Current password
              <input
                required
                type="password"
                autoComplete="current-password"
                value={current}
                onChange={(e) => setCurrent(e.target.value)}
              />
            </label>
            <label>
              New password
              <input
                required
                minLength={10}
                maxLength={1024}
                type="password"
                autoComplete="new-password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
              />
            </label>
            <label>
              Confirm new password
              <input
                required
                type="password"
                autoComplete="new-password"
                value={confirm}
                onChange={(e) => setConfirm(e.target.value)}
              />
            </label>
            <button className="btn">Change password and sign out</button>
          </form>
        </fieldset>
      </>
    </section>
  );
}
