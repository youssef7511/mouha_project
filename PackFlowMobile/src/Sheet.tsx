import { useEffect, useRef, useState, type ReactNode } from 'react';
import { X } from 'lucide-react';
import { message } from './api';
export function Sheet({
  title,
  children,
  onClose,
  dirty = false,
  busy = false,
}: {
  title: string;
  children: ReactNode;
  onClose: () => void;
  dirty?: boolean;
  busy?: boolean;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  const [discard, setDiscard] = useState(false);
  useEffect(() => {
    const el = ref.current!;
    el.showModal();
    return () => el.close();
  }, []);
  const close = () => {
    if (busy) return;
    if (dirty) setDiscard(true);
    else onClose();
  };
  return (
    <dialog
      ref={ref}
      className="sheet"
      onCancel={(e) => {
        e.preventDefault();
        close();
      }}
      aria-label={title}
    >
      <header className="sheet-head">
        <div>
          <span className="eyebrow">PACKFLOW MOBILE</span>
          <h2>{title}</h2>
        </div>
        <button className="icon-btn" aria-label="Close" disabled={busy} onClick={close}>
          <X size={22} />
        </button>
      </header>
      {discard ? (
        <div className="sheet-body">
          <h3>Discard unsaved changes?</h3>
          <p>Your saved records stay unchanged.</p>
          <div className="actions">
            <button className="button" onClick={() => setDiscard(false)}>
              Keep editing
            </button>
            <button className="button danger" onClick={onClose}>
              Discard changes
            </button>
          </div>
        </div>
      ) : (
        children
      )}
    </dialog>
  );
}
export function ErrorLine({ error }: { error: unknown }) {
  return error ? (
    <p role="alert" className="error">
      {message(error)}
    </p>
  ) : null;
}
export function Empty({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div className="empty">
      <span className="empty-mark">◇</span>
      <h3>{title}</h3>
      <p>{children}</p>
    </div>
  );
}
