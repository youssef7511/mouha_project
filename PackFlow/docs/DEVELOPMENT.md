# PackFlow implementation foundation

## Environment verified

- Workspace: `C:\Users\Youssef\Desktop\mouha_project\PackFlow`; the separate Medic repository was not edited.
- Host: Windows, Node 24.18.0, npm 11.16.0, Docker Engine 29.7.2 using Linux x86_64 containers. This Node version satisfies the selected tool engine requirements; no global Node update was needed.
- Host Rust/Cargo/rustup and Visual Studio C++ build tools were absent. Rust 1.99.0 is available in the isolated Docker image; no system-wide toolchain was installed.
- Dependency locks: `package-lock.json`, `Cargo.lock`; selected React 19.3.0, TypeScript 7.0.2, Vite 8.3.4, plugin-react 6.1.2, Tailwind 4.3.3, TanStack Query 5.104.2, RHF 7.90.0/Zod 4.6.6, Tauri 2.12.3/rusqlite 0.40.2. Bundled SQLite reported 3.53.2.
- Frontend fonts are local Fontsource DM Sans/Manrope resources. No font/CDN calls are required.

Validation completed on this host: React/TypeScript production build; Rust real-file persistence suite; frontend exact-money suite; browser workflows against the real Rust core; original prototype regression checks; and Tauri desktop Rust/config compile check in Linux. macOS runtime, CPU-specific bundles and signing remain unverified. This is implementation progress across M0–M2, not a claim that those milestones' complete macOS/recovery acceptance gates are finished.

## Layout and integration

`crates/packflow-core` owns the database, embedded versioned schema, exact monetary calculations, typed mutation/query requests, native validation and domain services. `DbWorker` owns one connection on a bounded dedicated thread. Mutations are immediate transactions with actor/time history, optimistic versions and durable request-id deduplication. Failed multi-row commands roll back their sequence/quote/item/stage/audit changes.

`src-tauri` is a thin native adapter: only allowlisted query/mutate commands, single-instance behavior and an app-data directory. Debug data uses a distinct `.dev` app-data location. No frontend arbitrary SQL or generic filesystem capability. Startup refuses unrecognized/newer/checksum-mismatched databases rather than replacing them.

`src` contains React forms, typed repositories, TanStack Query hooks, shared dialogs and production UI. It uses Tauri invoke in desktop mode. Browser development uses a feature-gated Rust HTTP adapter through Vite's local proxy. The adapter is excluded from the desktop build and is not a production server architecture.

Initial schema contains clients, workers, templates, orders/items/stage snapshots, quote history, stage work, canonical expense rows, and the future production/dispatch/payment/audit/operational tables. Migration 002 adds outside services and a derived financial view; customer receipts now have a mutation workflow. Migration 003 adds optional agreed payment deadlines and supports uncapped client insights derived from live ledgers. Future tables do not imply all their mutation workflows are implemented. Migration 005 adds separate monthly salary calculation tables. No inventory or invoice tables exist. Applied migration SQL remains immutable. Migration 004 persists stage-specific customer pricing and box-type selling rates without changing existing quotes. Migration 006 adds separate Machine/Finish/Others expenses in Costs. Schema upgrades through v6 verify every applied migration checksum and create one verified SQLite safety snapshot under `.dev-data/backups/` before upgrading an existing development database. General recovery tooling and guided recovery UI remain later work.

## Run and stop

```powershell
npm ci
docker compose up -d core
npm run dev
```

Browser: `http://127.0.0.1:1420`. Core: `127.0.0.1:4318` through Vite; development database `.dev-data/packflow.sqlite`. Stop Vite with Ctrl+C; stop only PackFlow's development container with `docker compose stop core`. Existing development data remains. Do not delete `.dev-data` or Docker volumes to stop the application.

If Rust sources change, restart the development container to recompile: `docker compose restart core`. Vite reloads frontend changes automatically. Production builds intentionally refuse standalone browser database access. Use the development server for review or the Tauri native wrapper for desktop runtime.

## Checks

```powershell
npm run build
npm test
npm run test:core
npm run test:prototype
```

Rust formatting/static analysis:

```powershell
docker compose run --rm core-test cargo fmt --all -- --check
docker compose run --rm core-test cargo clippy --locked -p packflow-core --all-targets --features preview-server -- -D warnings
```

Browser tests use a different development server (1421), core port (4319), and a container tmpfs database. They never write `.dev-data` or desktop application data. Start the test core, install a test browser if needed, and run:

```powershell
docker compose --profile test up -d core-e2e
npx playwright install chromium
npm run test:browser
```

`PACKFLOW_CHROMIUM_PATH` may point to an existing Chromium executable instead of downloading another. A new test container starts with empty data; browser tests should use independent fixtures rather than rely on a previous test. Stop it with `docker compose --profile test stop core-e2e`. Native source compile check in an isolated Linux image: `npm run check:native`. This checks Rust/Tauri/config integration, not macOS runtime/signing.

## Implemented and remaining

Implemented and tested: empty real workspace startup; client/profile forms; template add/edit/reorder/delete-unused/archive; order creation with multiple box sizes, fixed/per-box/unpriced KWD quote; stage snapshots; stage status/notes/out-of-order acknowledgement; work descriptions/Unit values/waste/manual time; stage expense posting and exact quantity × purchase-cost arithmetic; costs and margin; query invalidation/reload persistence. Native transactions support work+expense batch posting even though the current UI records the two forms separately.

Orders, Payments balances and Reports performance use native search/pagination over all relevant orders. Directory views and legacy suggestion lists remain capped at 200; dashboard, report and client aggregates are uncapped. Paging for the remaining directories and larger production-sized performance checks remain later work. Current time basis is Asia/Kuwait UTC+3; settings editing is not implemented. Stage-template CRUD is complete, while additional per-order stage CRUD, work assignments and corrections remain upcoming. Create uses template order even if checkboxes are toggled in a different sequence.

Clients ranks customers by order volume, net receipts or current order value and links each customer's order/payment history to Payments. Optional agreed payment deadlines are set independently of production deadlines in Payments. Partial payments are current balances, multiple receipts describe instalment patterns, overdue balances require an agreed deadline, and receipts after deadline count distinct payable orders using receipt business dates. Unknown deadlines remain unclassified. Credits remain attached to individual orders; they never offset other unpaid balances. Statistics include archived order history, exclude cancelled orders from order metrics, retain historical receipts, and ignore future-dated receipts until their business date.

The consistent SQLite snapshot helper and recovery into a separate database are tested, including a schema-1 upgrade safety snapshot. Scheduled backups, manifests, retention, durable restore swap/recovery and user-selected file handling are not implemented yet. Customer receipts, exact balances/credits and atomic outside-service customer charges/canonical supplier expenses are implemented. Reports uses uncapped current portfolio/stage data and date-filtered historical financial activity. New-order suggestions copy commercial details into a distinct new order, leaving existing ledgers/history untouched. Refunds, finished quantities, partial dispatches, lifecycle edits, quote revisions, expense/work reversals, report exports, CSV/XLSX/PDF and macOS release acceptance remain ahead.

Excel import is deliberately not implemented in this turn. The core creates no automatic imported/seed business records. When spreadsheets are supplied, inspect their sheets/columns and decide supported dataset mapping, dates, amounts, duplicates and opening balances. The intended import flow is file selection → template/mapping → validation preview → explicit atomic commit. Preserve the original workbook; rehearse on a separate database and back up before initializing business data. Do not use the old localStorage export as a production database.

New-order customer pricing is entered under each selected stage: fixed KWD total, per-box-type rates or quantity × rate in Unit. Complete stage totals form one saved base quote; incomplete quotes show the known subtotal and stay unpriced in balances/reports. Customer amounts appear in stage overviews, order details and Payments. Existing quotes retain their amounts; legacy order copying explicitly asks users to review the first-stage allocation.


Workers now includes monthly salary calculations with a per-month divisor (default 26), worker input snapshots, exact Friday/overtime amounts, edit/archive/restore and audit history. Salary records are separate from order costs and payments. Saving the month divisor recalculates all that month's rows in one transaction; other months are unchanged. The opt-in demo seed includes three fictional salary rows. The workbook remains untouched and has not been imported. Salary payments, deductions and advances are not implemented.

Costs now records separate expenses through a dialog that stays open after each save. Amounts and descriptions clear, category/date remain selected, and Done returns to the page. Separate costs feed Reports but remain outside order totals and margins. Their immutable entries are audited and request-id deduplicated; correction/reversal is future work.

Stage Costs now stays open after each expense save. Save & add another commits one expense, resets entry values while retaining category/date/supplier/box item/basis, updates the ledger/total and reports the count saved. Done closes the stage dialog. This is a frontend workflow change; the audited, idempotent native expense transaction is unchanged.

OrderPage provides parameter-bound literal search and 10/25/50/100 row pages; changing a search or page size resets to the first page. Reports UI requests aggregate portfolio summary values and loads its performance rows separately. Its totals/charts retain their original full workspace/date scope. Native tests and browser tests exercise an oldest record beyond 200 orders; no database migration is needed for these read-model changes.


## Reliability/account implementation — 15 October 2026

Schema 7 adds append-only service/separate-cost void records and updates current financial views. Existing work/expense reversal and receipt-refund allocation tables are reused; request deduplication/audit remain transactional. Financial event reports include separate-cost adjustments on their correction dates, and fully corrected receipts no longer inflate late/instalment classifications.

Portable `.pfbackup` snapshots use a magic header and SHA-256 checksum over SQLite snapshot bytes. Restore validates a staged copy and uses SQLite's backup transaction on the serialized worker, with a verified safety backup before committing. This avoids a live WAL-file rename/swap. Corrupt-startup recovery is documented as a manual preservation/reopen/restore procedure. There is no scheduled backup task yet.

`account.sqlite` is separate from the business DB and excluded from portable snapshots. Native commands require the local authenticated session. Passwords use salted Argon2id; recovery/session secrets are generated from two random UUIDs and stored as hashes. Login/recovery failures persist a cooldown. The native account mutex serializes access checks with logout/restore. The authenticated owner maps to the established Manager audit actor. Keychain is used only on macOS; other native development hosts offer password sessions without persistent Remember me. Browser preview remains a local development adapter, with its own local account; it is not a hosted production service.


### Web preview login — 15 October 2026

The local browser preview now asks for account setup/login and shows a Log out button above the workspace. Preview credentials use the same Rust password/recovery implementation, stored separately in `.dev-data/preview-account.sqlite`; existing business records stay intact. Protected preview reads/writes require a valid HttpOnly, SameSite=Strict cookie. Remember me on the browser uses a 30-day cookie; ordinary login uses a session cookie (browser session-restore settings may preserve it). Logout/password reset revoke the token. This does not change the Mac Keychain implementation. Backup/restore file dialogs remain desktop-only.

The automated business-flow fixture may skip authentication only in a debug preview process explicitly configured with `PACKFLOW_TEST_NO_AUTH=1` and `/test-data` as its disposable data directory. The normal `/data` preview does not accept this bypass. The separate `preview-auth.spec.ts` test exercises real login and API rejection with authentication enabled on a fresh isolated server.
