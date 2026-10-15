# PackFlow — implementation milestones and verification

Baseline: 11 October 2026. Dependencies follow the milestone order below. No time estimates are asserted before the native feasibility spike. Each milestone produces reviewable behavior and passing evidence, not just scaffolding. Product/database rules are in [PLAN.md](PLAN.md) and [DATA_MODEL.md](DATA_MODEL.md).

## M0 — architecture and transaction feasibility

First implementation task after plan review: create the smallest Tauri/React spike beside the preserved prototype, using an isolated development database. Pin the candidate toolchain/dependencies from ARCHITECTURE.md and commit lockfiles only after they compile together. Verify app-data resolution, frontend invoke contracts, native worker serialization, integer DTO round trips, SQLite backup and packaging.

Acceptance: on a Mac, build/open the app offline; execute a test command inserting two linked rows with injected failure between them and confirm neither remains. Repeat successful request with the same ID and confirm one mutation; same ID/different payload is rejected. Concurrent requests preserve invariants; reopen app and confirm persistence. Online backup opens as a valid snapshot; demonstrate restore of a spike dataset. Record actual bundled SQLite version and verify foreign_keys/WAL settings. Validate Vite React plugin/version resolution and XLSX/PDF adapter feasibility using small examples before confirming pins. Test native WebdriverIO embedded automation; record working versions or adopt the explicit native manual fallback. M0 cannot claim macOS validation from Windows browser checks.

## M1 — first complete persisted order workflow

Deliver offline AppShell, setup (business identity/timezone), client CRUD, basic worker profiles/actor selection, mixed-item order creation/detail/edit/search, quote modes and stage-template CRUD/snapshots. Forms and errors follow design tokens; no seeded business metrics. Implement native migration runner/pre-upgrade backup and manual backup/restore controls before live data.

Acceptance: create a client and two-size order, restart, reopen identical data. Create/edit/reorder/archive stage templates; existing order snapshots stay unchanged and a new order uses updated templates. Fixed/per-box/unpriced quotes round-trip exactly in KWD. Worker inactivation retains assignments. Invalid quantity/name/reference rejected in native commands. PF numbers unique even under repeated/concurrent creation. Archive/cancel/history behavior preserves references. Restore drill recovers a complete order into the same app after controlled loss in a test dataset.

## M2 — stage work, direct expenses and finished production

Deliver stage Overview/Work/Costs/History dialogs, worker assignments, input/output/waste Unit records, manual time and expense records, canonical Costs views, finished-box events by item, and audited corrections. Support per-order stage adjustments with retirement rules. No inventory/rate/payroll side effects.

Acceptance: record multiple batches, fractional Unit amounts, waste descriptions, labor/machine minutes and independent manual costs. Expenses appear once in stage and order totals. Failure in a work-plus-expense batch leaves no rows. Completing a stage does not record finished quantities or expenses. Record partial finished boxes; quantity limits hold per item. Reopen/status correction does not remove work/costs. Work/expense reversal+replacement preserves historical values and correct totals. Out-of-order warning requires acknowledgement/reason without auto-completing earlier stages.

## M3 — dispatches, payments and lifecycle correctness

Deliver partial multi-item dispatches, reversals/corrections, payment/refund allocations, balances/credit, quote revisions, close/reopen/cancel and archive workflows. Payments & balances replaces invoicing. Lifecycle, production, fulfillment and payment stay independent.

Acceptance: all nine combinations of fulfillment (none/partial/full) × payment (unpaid/partial/full), plus unpriced/zero/overpaid. Record deposit before work and fully paid partially dispatched order. Reject dispatch beyond finished quantity, refunds beyond receipts, duplicate reversal and quantity decrease below live counts. Partial dispatch reversal does not change money. Cancel with explicit final amount retains costs and receipts and creates no automatic refund. Close requires fully dispatched and stages complete but permits outstanding balance. Lost IPC reply retry cannot post again. Interrupted transactions leave consistent quantities and money after restart.

M1–M3 plus the backup restore drill are the first usable MVP. M4–M6 complete production v1.

## M4 — real dashboards and reports

Deliver database-derived dashboard, filters, event-based production/dispatch/work/cost/payment reports, current balances and quote/margin cohort views. Use metric date/denominator rules from PLAN.md. Remove sample charts, weekly goals, fake alerts and payroll/inventory figures.

Acceptance: fixed dated fixtures spanning business week/month boundaries match expected totals. Original events and later reversals fall in their own business periods. Archived/cancelled history stays in financial event totals. Stage completion/recompletion does not double-count first milestones. On-time first-completion metric uses snapshotted due date and displays N/A for empty denominator. Current balance is labelled current, not as-of past period. Quote values are never added to receipts as duplicate revenue. Mixed-size order totals and worker minutes aggregate correctly without treating Unit outputs as finished boxes.

## M5 — portability, documents and automatic recovery

Deliver supported CSV/XLSX templates, preview/mapping/all-or-nothing imports, safe exports, A4 internal order summaries/report PDFs, daily backup catch-up/retention/error visibility and guided restore with safety backup/journal. No invoice or label documents. Keep adapters offline and separate from domain writes.

Acceptance: Unicode CSV/XLSX round trips, quoted commas/newlines, KWD 0.001 precision, blanks versus zero, chosen date/decimal profiles, malformed/duplicate/FK-invalid rows, insertion failure and repeated batch hash. No partial imports. Reject oversized/encrypted/formula-containing import fields. Spreadsheet-leading malicious text remains text in exported files. PDF page dimensions/font embedding/totals and multipage table headers verified offline; unsupported scripts are reported rather than rendered incorrectly. Full/missing/read-only backup target reports failure accurately and never deletes previous good backups. Test corrupt checksum, traversal archive, newer schema, foreign-key violations, interruption during swap, failed reopen and rollback. Demonstrate restored clients, items, stages, quote histories, payments and dispatches, not merely an empty DB.

## M6 — native release readiness

Deliver universal macOS 14+ .app/.dmg, license notices, operation guide (backup, restore, corrections, manual update) and known limitations. Signing/notarization only with available Developer ID credentials. Keep test-only automation/debug plugins out of release.

Acceptance: Apple Silicon and Intel target verification, standard/Retina visual checks at 1440×900 and 1024×700, keyboard and assistive-label checks, native dialogs/cancel/retry, secondary-launch focus, offline fonts/assets, restart persistence and manual update preserving data. Real Mac restore drill and native smoke are required. Run long ledger lists with pagination and representative 10,000-row exports without freezing the main UI. Confirm no network is needed for ordinary operations. Unsigned test builds are clearly identified; no false claim of notarized distribution.

## Required test strategy

| Layer | Evidence |
| --- | --- |
| Rust domain unit tests | Exact arithmetic/rounding, pricing modes, status derivations, bounds and all independent state combinations |
| Rust real SQLite integration tests | Fresh/upgraded schema, constraints, transactions, concurrency, idempotency, corrections, migration failures, persistent restarts |
| Contract tests | Rust/TS JSON fixtures, integer-string amounts/quantities, dates, error shapes and field paths |
| Frontend tests | Form validation/error recovery, query invalidation, accessible dialogs, unknown/zero value behavior; avoid tests mirroring styling implementation |
| Browser E2E | Customer → mixed-item order → stage work/cost → production → partial dispatch → deposit/refund using test repositories; verify UI intent and no invented side effects |
| Native macOS integration | Real IPC/database/dialog/offline behavior and recoverability; embedded harness if M0 validates it, otherwise recorded manual protocol |
| Release recovery | Verified backup integrity, retention failure handling, safety backup, interrupted restore journal and complete restore drill |

Representative acceptance fixture: DATA_MODEL.md section 6; exercise each state transition with costs/payment unchanged unless the command explicitly changes those records. No automatic seeded data in production. Existing prototype tests remain available as behavior examples but are not rewritten to force obsolete stock/payroll behavior into the new product.

## Review status and implementation handoff

Confirmed by user: one shared Mac; editable Lamination/Impose (printing)/Diecut/Finish stages; mixed box types/sizes; Unit materials with descriptive sheet details; direct purchases; empty Stock; separate customer quote; fixed-total/per-box pricing; KWD; no invoices/VAT; monthly salaries with editable divisors per the 14 October amendment, salary payments/advances later; partial dated dispatches; manual machine/labor costs.

Chosen reviewable defaults: Asia/Kuwait business timezone, English UI, macOS 14+, Apple Silicon+Intel, direct internal distribution, A4 reports/order summaries, first-completion metrics, native transaction ownership, finished-box prerequisite for dispatch, same-order customer credits, generic common stage fields, no attachments/labels/updater. These are documented defaults rather than undisclosed assumptions. Revise the specification before implementation if any default is rejected.

No further business answers are required to start M0. Mac access and signing credentials affect native verification/distribution, not approval of the domain design. Implementation must not start by porting all nine prototype modules indiscriminately; follow the current release boundaries and dependencies above.


## Monthly salary amendment acceptance

Verify workbook formula fixtures, per-component rounding, editable divisors, unchanged other months, persisted input/name snapshots, one record per worker/month, inactive historical workers, archive/restore totals and stale version rejection. Divisor changes must either update every monthly row or roll back entirely on overflow. Reload must preserve salaries and settings. Salary saves must leave order costs, work and customer payments unchanged. Browser coverage must add/edit a salary, change the divisor, archive/restore, navigate months and reopen history. Spreadsheet import and salary-payment workflows remain excluded.


## Separate costs amendment acceptance

Save Machine, Finish and Others entries consecutively in one open dialog, verify the amount/description clear and category/date remain selected after each successful save, and use Done to return without reposting. Reject invalid amounts, blank descriptions, dates and categories. Retry the same native request without adding duplicates or audits. Reload must retain entries. Costs totals must include entries beyond its latest-200 list limit. Period reports must include only correctly dated separate entries, with distinct category labels, while individual order data and margins remain unchanged. Upgrade from schema 5 must preserve quotes and monthly salary settings with one verified safety snapshot.


## Repeated stage cost entry acceptance

Record a quantity × unit-cost expense and a fixed-amount expense without closing the same stage dialog. Verify cost context is retained, new-entry fields clear, stage total and recorded rows refresh, invalid saves keep the dialog open, and Done closes after completion. Discarding an unsaved third expense must preserve the two committed expenses. Repeat cost entry on every selected stage and reload to verify persistence, unchanged stage status and customer quote/balance, and the exact order cost sum.


## Order search and pagination acceptance

Use more than 200 orders and confirm complete totals, stable non-overlapping pages, literal punctuation search, matching by order/client/number, page reset after changing search/page size, empty states and out-of-range clamping. Archived/cancelled exclusions apply only to the current balance/performance scopes. Browser tests must locate the oldest order beyond the legacy list limit in all three screens, follow its links into the correct details/payment view, and confirm report aggregate figures remain unchanged while searching/page navigation. Loading/errors must not display a wrong cached search page as current results.


## 15 October 2026 — reliability and local-account amendment

Implemented schema 7 full-entry corrections (stage work/cost, receipt, outside service and separate costs), manual portable backup/restore with integrity checks and a pre-restore safety backup, and a separate local owner account with Argon2id password hashing, offline recovery and macOS Keychain Remember me. Native business commands require authentication; the local development HTTP adapter is not part of the desktop release. Restoring business records preserves the device's account and revokes its active/remembered session. Reports post reversals on their correction date. Current financial totals exclude voided service charges; corrected receipts no longer inflate late/instalment statistics.

See docs/OPERATIONS.md and docs/MAC_HANDOFF.md for behavior, verification evidence and the on-device acceptance checklist. Real Mac installer/Keychain/file-dialog/update/restore checks are still pending. Automatic backup scheduling, order/item editing and quote revisions, partial refunds, dispatches and imports/exports remain deferred. Phone work remains deferred. Do not mark production acceptance complete based on Linux or browser checks.
