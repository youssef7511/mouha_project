import { ChevronLeft, ChevronRight, Search } from 'lucide-react';
export function ListSearch({
  label,
  value,
  onChange,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <label className="search list-search">
      <Search size={16} />
      <input
        id="screen-search"
        type="search"
        aria-label={label}
        placeholder="Search order name, client or number…"
        maxLength={200}
        value={value}
        onChange={(e) => onChange(e.target.value)}
      />
    </label>
  );
}
export function Pagination({
  label,
  total,
  page,
  pageSize,
  onPage,
  onPageSize,
  busy = false,
}: {
  label: string;
  total: number;
  page: number;
  pageSize: number;
  onPage: (page: number) => void;
  onPageSize: (size: number) => void;
  busy?: boolean;
}) {
  const pages = Math.max(1, Math.ceil(total / pageSize));
  const first = total ? (page - 1) * pageSize + 1 : 0;
  const last = Math.min(page * pageSize, total);
  return (
    <nav className="pagination" aria-label={`${label} pagination`}>
      <span className="pagination-count" aria-live="polite">
        Showing {first}–{last} of {total} orders
      </span>
      <label className="pagination-size">
        Rows per page
        <select
          aria-label={`${label} rows per page`}
          value={pageSize}
          disabled={busy}
          onChange={(e) => onPageSize(Number(e.target.value))}
        >
          {[10, 25, 50, 100].map((size) => (
            <option key={size}>{size}</option>
          ))}
        </select>
      </label>
      <div className="pagination-pages">
        <button
          className="btn subtle"
          aria-label={`${label} previous page`}
          disabled={busy || page <= 1}
          onClick={() => onPage(page - 1)}
        >
          <ChevronLeft size={15} />
          Previous
        </button>
        <span>
          Page {page} of {pages}
        </span>
        <button
          className="btn subtle"
          aria-label={`${label} next page`}
          disabled={busy || page >= pages}
          onClick={() => onPage(page + 1)}
        >
          Next
          <ChevronRight size={15} />
        </button>
      </div>
    </nav>
  );
}
