# PackFlow — offline production workspace

The first implementation now lives in `src/`, `crates/packflow-core/`, and `src-tauri/`. The original design prototype is preserved in `prototype/`.

## Run the development app on this Windows workspace

Node 24 and Docker Desktop with Linux containers are sufficient for the browser development build:

```powershell
npm ci
npm run dev:core
npm run dev
```

Open **http://127.0.0.1:1420**. The React UI uses the same Rust/SQLite domain core as the desktop wrapper. Development records persist under `.dev-data/`. Docker binds the development API to host loopback only. Docker is a development tool, not a production application dependency.

For optional fictional test data, run `npm run seed:demo` after the development core starts, then refresh the app. It adds four clients, three workers, and twelve orders covering mixed sizes, fixed/per-box/unpriced quotes, a zero-value quote, different stage progress, direct purchases, outsourced work, and a cost overrun. Three fictional monthly salary records demonstrate Friday/overtime calculations in Workers. Three linked outside services and six customer receipts demonstrate partial payment, full payment, credit, and an unknown balance. Agreed payment deadlines demonstrate overdue balances and receipts after deadline, with several partially paid orders for client comparison. Sample clients and orders use a `[Demo]` prefix. The script preserves existing records and records its progress in `.dev-data/demo-seed-v1.json`; rerunning it resumes interrupted work without duplicating completed seed steps. Keep that journal with its database. Stage completion itself never records a dispatch or payment. New databases remain empty unless you explicitly run the seed command.

Implemented: clients and client rankings/payment-pattern statistics, worker profiles and separate monthly salary calculations with editable divisors, stage-template CRUD/reordering/archive, mixed-item order creation and copying with stage-level fixed/per-box/Unit KWD prices summed into base quotes, snapshotted order stages, audited manual status updates, stage work and direct purchase/manual costs, customer receipt/balance tracking with separate agreed payment deadlines, linked outside-service charges/costs, separate Machine/Finish/Others expenses with repeated entry in one dialog, search/pagination for Orders, order balances and report performance, and live reports with financial activity, expense categories, stage progress, and order margins. Stock is empty. Manual verified backup/restore, local desktop login with recovery and macOS Remember me, and audited full-entry corrections are implemented. Partial refunds, dispatches, order/quote editing, report exports, automatic backups and Excel import remain subsequent milestones. See [operation guide](docs/OPERATIONS.md). **This foundation build is for development review; the complete release acceptance gates have not passed.**

On a Mac with Rust and Xcode Command Line Tools, use `npm run tauri -- dev` for the native app. Development and production desktop data locations differ. A macOS `.app`/`.dmg` has not been built or verified on this Windows host.

See [development and verification notes](docs/DEVELOPMENT.md) and [the approved product specification](planning/PLAN.md). The source package's original handoff instructions remain below for reference; their prototype-only commands are now `npm run preview:prototype` and `npm run test:prototype`.

---

## Original design handoff

This package contains the complete source of the current PackFlow interactive design prototype and a planning prompt for a production macOS app.

**The current source is plain HTML, CSS, and JavaScript. It is not yet a React, Tailwind, TypeScript, Tauri, or SQLite application.** The proposed production stack will be selected and implemented after the planning phase.

## Start the Codex planning phase

1. Extract this ZIP into a folder on your Mac.
2. Open the extracted `PackFlow` folder in Codex.
3. Select Plan mode if your Codex interface provides it.
4. Paste this instruction:

```text
Read README.md, CODEX_PLAN_PROMPT.md, and docs/KNOWN_LIMITATIONS.md.
Inspect the prototype source and tests as the visual and behavior reference.
Follow CODEX_PLAN_PROMPT.md to plan the entire production PackFlow macOS app.
Stay in the planning phase: produce architecture, database design, business
rules, delivery milestones, acceptance criteria, and only the essential
questions. Do not scaffold or implement the application yet.
```

The full prompt in `CODEX_PLAN_PROMPT.md` is self-contained. You can also copy its contents directly into Codex.

## What's included

| Path | Purpose |
| --- | --- |
| `CODEX_PLAN_PROMPT.md` | Full product and engineering brief for the planning phase |
| `docs/KNOWN_LIMITATIONS.md` | What the prototype does and what production must replace |
| `docs/PROTOTYPE_DATA_EXPORT.md` | Optional export of changes already saved in your web browser |
| `prototype/dist/index.html` | App entry point |
| `prototype/dist/styles.css` | Shared visual system and responsive layouts |
| `prototype/dist/app.js` | Dashboard, projects, clients, stock, reports, stages, and navigation |
| `prototype/dist/finance.js` | Linked order payments, costs, fulfillment, workers, and salaries |
| `prototype/tests/finance.test.cjs` | Logic and template checks for linked financial workflows |
| `package.json` | Convenience preview and test commands; no runtime dependencies |

Git history, hosting configuration, repository credentials, and deployment archives are excluded. This is a portable source copy, not a connection to the hosted Site repository. It contains fictional sample data; edits made in your browser are not embedded in the source ZIP.

## Run the design prototype

From the extracted `PackFlow` folder, with Python 3 installed:

```bash
python3 -m http.server 8080 --bind 127.0.0.1 --directory prototype/dist
```

Open `http://127.0.0.1:8080` in your browser. Stop the server with Control+C.

Alternatively, with Node/npm and Python 3 installed:

```bash
npm run preview:prototype
```

No `npm install` is needed for this handoff package. These are prototype commands, not Tauri build commands. Use the same browser and origin to retain local prototype edits. A different port, browser, or hostname has separate local storage.

The prototype loads DM Sans and Manrope from Google Fonts. Without internet, system font fallbacks are used. The production plan requires local, properly licensed fonts and an offline UI.

## Run the existing checks

With Node.js installed:

```bash
npm run test:prototype
```

Or:

```bash
cd prototype
node tests/finance.test.cjs
```

These checks cover screen template rendering, handler syntax, all nine fulfillment/payment combinations, unpriced and zero-value orders, customer credits, refunds, outside-service costs, inventory deductions, salary history, and new-order defaults. They use a mocked DOM; they are not browser screenshots, React tests, SQLite integration tests, or macOS application tests.

## Current screens and behavior

- Dashboard: project metrics, packing progress, low-stock cards, sample activity and weekly goal.
- Projects: cards, detail panel, seven manual packing stages, served quantities, and order/payment summary.
- New project: client, name, box count, due date, and packing notes.
- Clients: directory, customer projects, contacts, and add-client form.
- Stock: boxes/materials, receive-stock form, filters, stock bars, and CSV export.
- Reports: sample weekly packing chart and completion bars.
- Payments & invoicing: linked orders, independent fulfillment/payment statuses, payment/refund ledger, optional outside-service charge and supplier expense.
- Costs: per-project expense totals, recorded-cost margin, material allocation, labor allocation, transport, and other expenses.
- Workers & salaries: worker details, active/inactive state, monthly salary, manual adjustments, advances, and payment history.

## Production stack to assess in planning

- Shell: Tauri 2, targeting macOS.
- UI: React + TypeScript + Vite + Tailwind CSS.
- Data: local SQLite, versioned migrations, typed repositories, atomic domain transactions.
- Database integration: assess the official Tauri SQL plugin first; evaluate Drizzle tooling separately. An ORM does not replace the SQLite engine or native database bridge.
- State: TanStack Query for repository queries/mutations; Zustand only for genuinely useful transient UI state.
- Forms: React Hook Form + zod, plus validation at the native/data boundary.
- Extras: CSV/XLSX import/export, PDF reports/invoices/labels, local automatic backups and restore.

The prompt asks Codex to verify current compatible versions, macOS prerequisites, the database transaction boundary, and practical release requirements rather than assuming an untested driver combination.

## Official engineering references

References checked on 11 October 2026; Codex should verify them again when planning and implementation begin.

- Tauri SQL plugin and migrations: https://v2.tauri.app/plugin/sql/
- Tauri macOS prerequisites: https://v2.tauri.app/start/prerequisites/
- Tauri capabilities: https://v2.tauri.app/security/capabilities/
- Tauri Vite integration: https://v2.tauri.app/start/frontend/vite/
- Drizzle driver/proxy documentation: https://orm.drizzle.team/docs/connect-drizzle-proxy
- SQLite online backup API: https://www.sqlite.org/backup.html
- SQLite transactions: https://www.sqlite.org/lang_transaction.html

Production implementation and a distributable `.app`/`.dmg` are the next project, not deliverables already included in this ZIP.


### Web preview login — 15 October 2026

The local browser preview now asks for account setup/login and shows a Log out button above the workspace. Preview credentials use the same Rust password/recovery implementation, stored separately in `.dev-data/preview-account.sqlite`; existing business records stay intact. Protected preview reads/writes require a valid HttpOnly, SameSite=Strict cookie. Remember me on the browser uses a 30-day cookie; ordinary login uses a session cookie (browser session-restore settings may preserve it). Logout/password reset revoke the token. This does not change the Mac Keychain implementation. Backup/restore file dialogs remain desktop-only.

The automated business-flow fixture may skip authentication only in a debug preview process explicitly configured with `PACKFLOW_TEST_NO_AUTH=1` and `/test-data` as its disposable data directory. The normal `/data` preview does not accept this bypass. The separate `preview-auth.spec.ts` test exercises real login and API rejection with authentication enabled on a fresh isolated server.

For the optional seed script after preview account creation, supply `PACKFLOW_PREVIEW_USERNAME` and `PACKFLOW_PREVIEW_PASSWORD` in that shell's environment. The script signs in locally before writing; do not commit those values to a file. A new sign-in replaces the previous preview session, so the browser may ask you to sign in again afterward.
