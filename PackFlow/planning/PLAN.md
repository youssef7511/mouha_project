# PackFlow — production plan and product specification

Planning baseline: 11 October 2026. Status: ready for review; application implementation has not started.

## 1. Executive recommendation

Build a small offline desktop application for one shared Mac: Tauri 2, React/TypeScript/Vite/Tailwind, and SQLite owned by Rust. Rebuild the prototype UI while preserving its visual identity. Use native domain commands for every database operation; do not expose arbitrary SQL to the frontend. Start with a transaction/persistence feasibility spike, then a complete customer-to-order-to-stage workflow.

The user's decisions override the original planning prompt: stages are editable, materials are direct purchases, Stock is empty, monthly salary calculations are included by the 14 October amendment below, and invoices/VAT are excluded. Salary payment ledgers and advances remain deferred. Customer quotes and payment tracking remain required. Currency is KWD, precision three decimal places.

Read alongside [ARCHITECTURE.md](ARCHITECTURE.md), [DATA_MODEL.md](DATA_MODEL.md), and [ACCEPTANCE_CRITERIA.md](ACCEPTANCE_CRITERIA.md). These four documents form one specification. Defaults explicitly marked below can be revised during review; they are implementation instructions until changed.

## 2. Current reference and gaps

Inspected the handoff README, limitations, optional prototype export instructions, HTML/CSS, app.js, finance.js, and finance tests. Existing Node checks pass: nine screen templates and handlers, nine fulfillment/payment combinations, refunds, outside-service costs, inventory allocation, salary snapshots, and localStorage persistence. These checks use a mocked DOM and do not establish browser appearance, SQLite atomicity, or macOS behavior. UI_screenshots contains no files at inspection time.

| Reference behavior | Production treatment |
| --- | --- |
| Seven fixed warehouse stages with status dialogs | Replace with configurable production stages and dedicated management dialogs |
| Packing completion sets packed quantity to all ordered boxes | Explicit partial finished-box records per order item; status alone never generates quantities |
| Cumulative served count changes Shipped status | Dated partial dispatches and reversals; no fixed Shipped production stage |
| Separate localStorage keys and global JS state | One durable database, atomic Rust transactions, typed query/mutation boundary |
| GBP examples and two-decimal arithmetic | KWD integer fils; three-decimal display and exact calculations |
| Inventory/material deductions | Direct purchase expenses only; Stock remains a placeholder |
| Payments & invoicing / Workers & salaries | Payments & balances / Workers with separate monthly salary calculations; no invoices or salary payment ledger |
| Sample charts, activity, targets and trend annotations | Actual dated records, no invented metrics; omit weekly targets in v1 |
| Editable outside-service fields generating costs | Multiple explicit expense rows; no automatic customer charge or duplicate cost |
| Remote Google fonts | Locally bundled fonts with license notices |

## 3. Scope and release boundaries

| First usable MVP | Production v1 completion | Later / explicitly deferred |
| --- | --- | --- |
| Local setup, customers, mixed-item orders, quotes | Dashboard and full event-based reports | Multi-Mac sync, cloud services, Windows release |
| Stage template CRUD and per-order snapshots | CSV/XLSX import/export and PDF reports | Stock, inventory tables, reservations, stock costing |
| Stage work, direct expenses, manual machine/labor costs | Automatic backup retention and guided restore | Salary payment ledgers/advances, automated stage wage allocation |
| Finished-box records, partial dispatches/reversals | Native macOS verification and .app/.dmg delivery | Invoices, invoice numbers, tax/VAT, credit-note documents |
| Payments/refunds, costs and margin | Resize/accessibility/offline release checks | Printer integration, shipping labels, barcodes/QR |
| Manual backup and restore before live use | Correction/import/recovery hardening | Attachments, prototype data migration, statutory accounting |

No real business use until the MVP restore drill and financial/quantity tests pass. Prototype migration is optional future work: the supplied sample data is fictional and its old stages/stock/payroll do not match this model.

## 4. Screens and navigation

Preserve the warm ivory/charcoal/burnt-orange design, green completion and amber progress, cards, sidebar, project detail panel, and English labels. Rename customer-facing project references to Orders while retaining PF numbering. Production navigation: Dashboard, Orders, New order, Clients, Stock, Reports, Payments & balances, Costs, Workers, Settings. Stock displays only a neutral empty placeholder; no buttons, counts, tables, backend, or stock alerts. Workers contains profiles and assignments only.

| Screen | Required behavior |
| --- | --- |
| Dashboard | Active orders, remaining finished boxes, completed stages, overdue orders, production/dispatch summaries, recent audited activity; no stock/payroll/invoice metrics |
| Orders | Search by number/name/client; lifecycle, production, fulfillment and payment filters; cards/table, item quantities and expandable detail |
| Order detail | Overview, box items, quote, stage list, finished production, dispatches, costs, payments and dated history; edit/archive/cancel/reopen workflows |
| New order | Client, name, due date, notes, repeatable box items, pricing mode, optional prices, stage selection/reorder and worker assignments; preview totals |
| Clients | Add/edit/archive, contact/billing-free address details, linked orders, balances and box totals; duplicate name warning |
| Payments & balances | Linked orders with quote, received/refunded/net, outstanding/credit and independent fulfillment; record payment/refund and history |
| Costs | One canonical expense ledger, grouped by order/stage/category; direct purchases, outsourced, machine, labor, transport, other; recorded margin |
| Workers | Worker profiles, stage work and separate monthly salary records with editable monthly divisor, Friday and overtime calculation |
| Reports | Production, dispatch, stage work/completions, quoted value, receipts/refunds, recorded expenses, balances and estimated margin; date/filter definitions below |
| Settings | Business name/address/contact, timezone, stage templates, backup controls, data import/export; KWD fixed, English UI |

Reference layout 1440×900, minimum window 1024×700, scrolling rather than clipped content. At smaller widths collapse the detail column. Keyboard-operable tables/actions, labels, field errors, visible focus, focus trap/return, Escape close, and unsaved-change confirmation. Do not use color alone for status. Common components: AppShell, PageHeading, StatCard, DataTable, SearchFilters, StatusBadge, MoneyField, QuantityField, FormDialog, ConfirmDialog, EmptyState, ErrorBanner, HistoryList. Cmd+N new order and Cmd+F current-screen search; Cmd+S saves the active form.

## 5. Orders, quotes and lifecycle

An order has a stable UUID, unique sequential PF-000001 number, client reference, name, due date, notes, and one or more box items. Each item has a description/type, dimensions as free text, positive integer ordered boxes, and an optional unit selling price. Do not interpret dimensions or descriptions as machine specifications. Customer edits never break existing order references.

Pricing modes: Unpriced, Fixed total, Per box type. Fixed mode stores only one order quote amount; per-box mode calculates the sum of each item's ordered boxes multiplied by its unit selling price. Every required price must exist for a per-box quote; zero is a valid price and differs from missing. No editable derived grand total in per-box mode. Stage expenses never change customer pricing. Additional customer charges, discounts and taxes are deferred; revise the explicit quote when needed.

Every quote change creates a dated revision, with old/new mode and values plus a reason after the first priced revision. Payments stay allocated to the order, not to a quote revision. Changes to quantities/pricing after dispatch are permitted with a reason, provided each quantity remains at least its net finished and dispatched quantity. Existing item identities remain stable; items with history cannot be deleted. Client reassignment is blocked after the first payment or dispatch; use a correction discussion outside v1 rather than silently moving balances.

Lifecycle is Open, Closed, Cancelled; archive is a separate flag. Closing requires all applicable stages completed and every item fully dispatched, but does not require payment. Closed orders may receive payment/refund or expense corrections; new production/work/dispatch requires explicit reopen. Reopen is audited. Cancellation requires reason and an explicit final customer amount owed, preserving the former quote in history; keep expenses, production, dispatches and receipts. It does not create a refund or undo dispatched boxes. Reopening a cancelled order restores its last non-cancellation quote through a new revision. Archive hides an order from operational screens but retains reporting and balances; defaults permit archive only for closed/cancelled orders. Financial reports always retain historical archived/cancelled entries.

## 6. Configurable stage management

Initial template order: Lamination → Impose → Diecut → Finish. Impose means printing/impression. Stages are ordinary configurable records, not hardcoded enum values; labels do not trigger business logic. Settings supports add, edit name/default notes, drag reorder, archive, and delete only unused templates. At order creation, copy selected active templates into independent order-stage snapshots. Later template changes never update existing orders.

Per-order stage CRUD permits add/rename/reorder/remove before work starts. After a stage has work/cost/history, retire it with a reason rather than deleting; retired stages remain visible in history and costs and are excluded from progress. A stage with live work/cost entries cannot be retired until those entries are reversed or explicitly accepted as retained sunk work/cost in the same audited operation. At least one applicable stage is required. Reordering or adding a stage after work starts requires a reason; adding an incomplete stage reopens a closed order explicitly. No arbitrary custom field designer in v1: all stages use the common management form.

### Stage management dialog

Use a wide dialog with Overview, Work, Costs, History tabs. Header: order number/name, stage snapshot label, current state and summary recorded cost. Save status/notes separately from posting work or expenses, with clear action buttons; changing a checkbox must not post costs.

| Input / section | Meaning and default |
| --- | --- |
| State | Not started, In progress, Completed; manual and audited |
| Stage notes / dimensions | Optional descriptive text; dimensions free text; sheet quantity can be written in description |
| Assigned workers | Multi-select active worker profiles; old assignments remain visible |
| Work record | Business date, optional box item, description, input/output/waste quantities in Unit, optional worker and worker minutes, machine description and machine minutes |
| Material/direct purchase row | Business date, optional item, material description, optional supplier/reference, quantity Unit, pricing basis Unit cost or Total amount |
| Other cost row | Category Outsourced, Machine, Labor, Transport or Other; description/date, optional worker/item/supplier, quantity Unit, unit cost or total amount |
| Posting preview | Exact line total in KWD, stage/order cost after posting; no stock balance or automatic quote change |
| History | Work, costs, status, assignments and corrections, with action actor/time and reasons |

Work quantities are informational; input/output/waste have no automatic conversion or conservation rule because materials and output can differ. Blank is unknown; zero is known zero. Use Unit everywhere for material/work quantities; no sheets-to-box conversion, stock receipt, machine integration, or production inference. Time is entered as hours/minutes and stored as whole minutes. Material/work quantities accept up to three fractional digits; final boxes remain integers.

Multiple work/cost rows support batches and mixed box items. Cost rows may link to a work record but time never calculates money. Each expense amount is entered manually or calculated from an explicitly entered quantity and unit cost; monthly salary calculations do not allocate wages to stage work. Waste has no extra auto-charge: include its purchase cost once in the material row. A direct purchase is recorded expense incurred, not a supplier payment ledger. Zero-cost work is valid. Changing or reopening stage status does not reverse expenses or production.

Out-of-order stage updates are allowed after a visible warning and recorded acknowledgement/reason if earlier applicable stages are incomplete. Never auto-complete previous stages. Stage progress = completed applicable stage count / applicable stage count. A stage can complete without costs; a missing cost is not proof that no expense occurred.

## 7. Finished boxes and dispatches

Maintain three distinct truths: ordered boxes per item; finished boxes from production events; dispatched boxes from dispatch events. Stage Unit values are not finished-box counts. Add a Record finished boxes action inside stage Work and order Production; both use the same command/event model. This accommodates a renamed/reordered final stage without hidden label-based behavior. Require explicit item quantities and date; warn if applicable stages remain incomplete. No double submission or silent full-quantity fill on stage completion.

Per item invariant: 0 ≤ net dispatched ≤ net finished ≤ ordered. Dispatch defaults require finished quantities already recorded; record missing production first. Dispatch records contain date, item quantities, recipient/address snapshot, optional carrier/reference and notes. One dispatch can contain several items. Fulfillment is Not dispatched, Partially dispatched, Fully dispatched from net quantities only; there is no separately editable fulfillment total or Shipped stage. Keep payment independent.

Correction: append a linked reversal (with positive reversal quantities and reason), then optional replacement, atomically. Reversal may be partial but cannot cumulatively exceed the original. Production reversals cannot reduce finished below dispatched. Dispatch reversals represent data corrections, not a complete physical returns workflow. Corrections use their actual business date so period reports show the adjustment when recorded; original dates stay intact.

## 8. Customer payments and cost reporting

13 October client statistics amendment: Clients gains rankings by non-cancelled order count, net customer receipts, or current priced order value; follow-up statistics for partial balances, multiple receipts per order, overdue balances and orders with receipts after an agreed deadline; and a connected order/payment review. Use a separate optional payment due date, explicitly agreed per order and edited in Payments. Production due dates never imply payment deadlines. Changing/clearing the payment date is versioned and audited. New orders, including copied orders, start without a payment deadline. No deadline means lateness is unknown, and unpriced or zero-value orders are excluded from payment-rate denominators. “Receipts after deadline” counts distinct orders with a receipt after the currently recorded agreed date, including orders now settled; it does not claim a historical credit score or exact settlement delay. Partial payments/deposits are review signals rather than a poor-client label. Credit cannot offset an unrelated order's unpaid balance. Statistics retain archived-order history, exclude cancelled orders from order performance, retain their receipt/refund history, and omit future-dated payment events from today's view. Ranking ties are deterministic and visible metrics replace an opaque overall score.

Implementation amendment approved 12 October 2026: the new-order name combobox offers existing orders by number/name/client. Selecting one copies its client, box items, quantities/dimensions, base pricing, notes, due date, and still-active templates for review. Saving creates a distinct order, number, item IDs and fresh not-started stage snapshots. Costs, payments, outside services, production/dispatch and history are not copied. A manually typed new name remains supported.

Payments also supports explicit outside services with description, business date, supplier/reference, customer charge and supplier cost in KWD. Save both atomically: one immutable service charge linked to one canonical outsourced expense. Keep the base quote snapshot unchanged. Current customer amount owed = base quote + service charges; unpriced base quotes keep the total and balance unknown. A zero charge or zero supplier cost is valid. Never also generate a second stage expense for the same service. This explicit user-requested charge mechanism supersedes the earlier restriction against automatically charging for ordinary expense rows; stage expenses still do not create charges. Customer receipt records remain independent of stage progress.

The initial live Reports page displays current non-archived/non-cancelled portfolio values, exact recorded-cost margins and stage distributions; date-filtered cost/receipt activity and expense categories retain historical archived/cancelled events. The UI labels these different scopes. Full production/dispatch/refund analytics, exports and additional report filters remain later acceptance work.

Payments/refunds contain order, business date, exact amount, method (Cash, Bank transfer, Card, Other), optional reference and notes. Support deposits before production/dispatch, multiple receipts, and overpayments. Refunds explicitly allocate to original payment records; total refund allocation cannot exceed each payment's remaining refundable amount. Reject duplicate requests and zero/negative amounts. Incorrect receipts are corrected by explicit reversal/refund and replacement, never overwritten. No transfer of funds occurs. Credit remains attached to the order; cross-order credit transfers are deferred.

Net received = receipts − refunds. Balance = max(current amount owed − net received, 0); credit = max(net received − current amount owed, 0). Unpriced orders show quote/balance as unknown; receipts may still be recorded and displayed without claiming a final credit. Priced zero orders show No payment due if net is zero and Overpaid if positive. Otherwise Unpaid / Partially paid / Fully paid / Overpaid. All nine fulfillment/payment combinations are valid.

Expenses are canonical rows shared by stage dialogs and Costs. Order-level expenses may omit stage; each expense can apply to one item or the whole order. Do not allocate whole-order expenses automatically between items. Stage totals and order totals are grouping views over the same rows, never extra expenses. Post immutable expense entries; corrections reverse original rows and post replacements. Preserve purchase/unit-cost snapshots. Estimated margin = current quote/final cancellation amount − net recorded expenses, independent of receipts. Unpriced margin is unknown. This is operational recorded-cost margin, not statutory profit or accrual revenue.

## 9. Metrics and report definitions

Default business timezone Asia/Kuwait (assumption; independent of this user's OS/chat timezone), English locale, Monday-start week and calendar month. Setup displays timezone for review. Event business dates select reporting periods; created_at UTC selects activity ordering. Date ranges are inclusive in the UI and evaluated as start-inclusive/end-exclusive boundaries. Filter dates default to current business week/month.

| Metric | Source / inclusion / denominator |
| --- | --- |
| Active orders / overdue | Open, unarchived orders; overdue if due date before business today and not fully dispatched |
| Boxes remaining to finish | Sum ordered minus net finished on open unarchived orders |
| Production / dispatch quantities | Signed production / dispatch event quantities by business date; include historical closed, cancelled, archived records |
| Current stage completion | Current applicable stage states; count completed / all applicable on filtered orders |
| Stage completions in period | First-ever completion action per stage by business date; exclude subsequent reopen/recomplete edits from completion count |
| Fully dispatched orders in period | First full-dispatch milestone per order, event dated; corrections do not generate second first-completion milestones |
| On-time first dispatch completion | First full-dispatch milestones in period on/before due-date snapshot / all first full-dispatch milestones in period; N/A for zero denominator |
| Quoted order value | Latest priced quote on orders created in period; label as current quote snapshot, not revenue |
| Cash received / refunds / net | Payment/refund ledger business dates; no quote amount added to cash totals |
| Recorded expenses | Signed expense ledger by business date; stage totals are not added again |
| Current balances / credits | Latest quote and lifetime net receipts as of now; clearly labelled current, not historical period balance |
| Estimated order margin | Latest quote minus lifetime net expenses on selected order cohort; unknown for unpriced orders |
| Worker effort / stage output | Work records, signed for corrections, by date; no automated wage or payroll metric |

Reports expose date range, client, order, stage/category where applicable, and lifecycle/archive filters. Historical event reports include all lifecycles by default. Production completion milestones are historical facts and snapshot due date; current fulfillment remains independently derived after reversals. Omit daily comparative percentages until enough real records exist; show empty/N/A states instead of seeded values.

## 10. Imports, exports and local documents

Production v1 supports CSV/XLSX exports for clients, order/items, work, expenses, dispatches, payment/refund records, balances and reports. Imports: clients; orders/items with quote and stage-template selection; direct expenses referencing existing orders/stages. Payment/dispatch bulk import is deferred to protect ledger correctness. Each supported dataset has a downloadable template; multi-item orders group by external order reference. File preview requires mapping, row errors, FK checks, duplicate checks and explicit confirmation. Import is all-or-nothing, insert-only; no silent overwrite. Duplicate strategy is reject with row errors; batches use content hash plus dataset to prevent accidental repeat, with explicit audited override only after review.

Use UTF-8 CSV, ISO YYYY-MM-DD business dates and decimal point in templates; explicitly choose decimal/date parsing profile in mapping preview for other inputs. Never guess ambiguous dates. Preserve Unicode. Neutralize spreadsheet-leading =, +, -, @ and control/whitespace-prefixed equivalents on untrusted text; XLSX text uses string cells, never formulas. Numeric fields remain typed numeric values. Default limit 10 MiB/10,000 rows; reject encrypted workbooks, formulas in imported business fields, malformed references and non-finite values. XLSX input uses file/expansion limits and a worker; unsupported oversized files fail without DB writes.

PDF v1: A4 reports and internal order summary (client, box items, quote, dispatch totals; no tax/invoice heading). Generate locally with pdf-lib and bundled fonts, stable sorting, business date/currency formatting and snapshot data. English UI and Latin PDF text are the baseline; Arabic text rendering/layout requires a separate validated font/shaping extension and is deferred rather than exported as broken glyphs. CSV/XLSX preserve Unicode. Printing means saving PDFs and opening through the OS after user action; no dependence on WKWebView print-to-PDF. Label dimensions/printers remain undefined, so labels and direct printing are deferred.

## 11. Delivery, risks and defaults

Target macOS 14+ on Apple Silicon and Intel, with universal .app/.dmg, direct internal distribution. This is a chosen support floor, not a claim that older macOS cannot run Tauri; it provides a modern WKWebView baseline for Tailwind 4. macOS build/test host is required; current workspace is Windows. Signing/notarization requires Developer ID credentials not assumed present. An unsigned internal test artifact may be produced, but is not a signed production release. No updater, LaunchAgent, cloud backup or App Store submission in v1.

One shared trusted workspace with selectable local actor profiles; initial Manager actor. Profiles identify history but provide no authentication or access control. No multi-user security guarantees; audit records improve traceability but are not tamper-proof. No unnecessary services or framework backend. Development data is separate from production; sample records only in opt-in demo mode. Stable production bundle identifier default com.packflow.desktop; development com.packflow.desktop.dev, finalized before the first live dataset.

| Risk | Mitigation / release gate |
| --- | --- |
| Stage template changes damage history | Per-order snapshots, retired stages and explicit audited changes |
| Mixed quantities or manual status misstate fulfillment | Item-level production/dispatch events and native invariant checks |
| Floating point or duplicate postings | Fils/scaled integers, checked arithmetic, idempotency and atomic writes |
| Direct purchases mistaken for stock/accounting | No inventory code; label recorded expenses; no claims of supplier payment or statutory accounting |
| One-Mac expectations expand | No shared/cloud-synced SQLite file; revisit architecture before multi-device work |
| Backup inaccessible or corrupt | Consistent SQLite backup, checksums/integrity, failure banner, safety copy and restore drill |
| Newly released dependency combinations | Exact candidate pins; compile/IPC/backup/build compatibility spike before UI expansion |
| WKWebView/offline/printing differences | Real Mac checks, offline fonts, local PDF generation, native test harness feasibility gate |
| Salary calculation scope expands to accounting | Keep monthly calculations separate; salary payments, deductions, advances and order allocation remain deferred |
| Hardware/signing availability | Mac verification required; signing credentials are a delivery prerequisite, not an assumed asset |

Remaining review preferences have defaults, not hidden implementation choices: timezone Asia/Kuwait, English UI, macOS 14+, both CPU targets, direct distribution, A4 documents, single main window, generic stage fields. No labels or payroll decisions block v1. First implementation task: milestone M0 in ACCEPTANCE_CRITERIA.md, using an isolated spike dataset and preserving prototype files.


## Stage customer pricing amendment

New orders enter customer prices separately beneath every selected production stage. Each stage supports not priced yet, fixed total quote in KWD, price per box type (each box type's ordered quantity × that stage's selling rate), or price per Unit (independent quantity in generic Unit × selling rate). Unit quantities accept up to three decimals; each Unit line rounds half up to one fils before stage totals are summed. Production expenses stay separate. Deselecting a stage removes its contribution from the new quote; reselecting restores the unsaved inputs. Removing a box type removes its matching rate from every stage.

The footer sums selected stage totals. A missing stage price means an incomplete full quote, with the known stage subtotal displayed separately. Zero-priced stages require an explicit fixed zero or zero selling rate, rather than treating unknown prices as free. Complete stage totals form the saved base quote; outside-service customer charges are still added separately, once. Payments, client statistics, reports and margin calculations consume that saved base quote.

Stage pricing is stored as an immutable quote breakdown linked to the snapshotted order stages and box items. Copying a new order copies the stage pricing inputs into fresh stage/item identities without receipts or costs. Existing orders retain their original commercial totals. When copying an older order without a breakdown, the form places its original quote on the first available stage and explicit zeroes on the other available stages, with a message requiring review of that allocation. Existing saved orders are never retroactively assigned prices to individual stages. Editing saved quote revisions remains a later workflow.


## Monthly salary amendment — 14 October 2026

The user has requested monthly salary management in Workers and selected an editable divisor per month. Each worker/month stores basic KWD salary, normal days worked, normal hours/day, whole Fridays worked, overtime hours and notes. New month divisor defaults to 26 and is editable from 1–31. Normal days default to that month's divisor and are entered separately from Friday days. Formulas follow the inspected Salaries sheet: normal pay = basic / divisor × normal days; Friday pay = basic / divisor × Fridays × 1.5; overtime pay = basic / divisor / hours-per-day × overtime hours × 1.25. Keep rates unrounded, round each payable component half up to integer fils, then sum components.

Save one record per worker/month. Preserve a worker-name snapshot, monthly inputs, calculated components and change history. Edit, archive and restore are audited/versioned; archived rows remain visible on request and are excluded from totals. Saving a changed month divisor atomically recalculates that month's records and increments their versions; all other months remain unchanged. Reject duplicate worker/month records, stale writes, invalid ranges and overflow before commit. Worker profile edits never silently rewrite monthly inputs or saved names. No salaries are imported automatically from the reference workbook. Salary payments, deductions, advances, payslips and automatic stage/order wage allocation remain deferred. The formulas represent the requested workbook workflow.


## Separate workspace costs amendment — 14 October 2026

Costs includes a separate expense list alongside the existing order cost review. Separate entries are workspace expenses with no order/stage/customer association. Inputs are category (Machine, Finish, Others), exact KWD amount, required description and business date (default today). The entry dialog saves one expense immediately with Save & add another, remains open, clears amount/description and keeps category/date. It shows the entries saved in that session. Done returns to Costs; closing may discard only the currently unsaved amount/description. Previous entries remain committed. Failed saves retain inputs for retry; double submissions are guarded and native requests are deduplicated.

Show category totals and an all-time total with the latest 200 entries, while totals include every entry. Reports includes separate expenses once in date-filtered financial activity, period cost totals and distinct category labels. Order quotes, stage costs, customer balances, client statistics, salaries and individual order margins remain derived from their own records. Saved cost correction/reversal workflows remain a later milestone.


## Repeated stage cost entry amendment — 15 October 2026

Every order stage's Costs tab uses Save & add another and Done, matching the separate-cost entry workflow. A successful save commits one canonical expense immediately and keeps the stage dialog open on Costs. Preserve category, business date, supplier, box item and entry basis; clear expense description, reference, quantity, unit purchase cost and total amount. Show the saved-entry count and refreshed stage cost/ledger. Failed validation or native saves keep the current inputs and do not count an entry as saved. Closing discards only unsaved form changes; committed expenses remain. Prevent repeated submits and tab changes while saving. The existing native post_stage_entry transaction, audit and deduplication remain unchanged.


## Order list search and pagination amendment — 15 October 2026

Orders, Payments order balances and Reports order performance use database-backed search and pagination across the entire relevant order set. Search matches literal text in order number, order name and client name; typing resets the page to 1. Inputs are capped at 200 characters and briefly debounced. Pages show 10, 25, 50 or 100 rows (default 25), Previous/Next buttons, current/total pages and matching-record range/count. Ordering is newest creation first, then descending unique order number. The database validates inputs and clamps pages beyond the last matching page. No-result screens keep usable search controls and disable page navigation.

Orders includes all order history, matching its previous scope. Balance/performance lists include non-archived, non-cancelled orders (including closed orders), matching their previous scope. Filtering or changing pages leaves selected order identity stable, and row links query details by ID, even when the order is outside the legacy latest-200 suggestion list. Report totals and charts represent their existing full workspace/date scopes; searching the performance list does not narrow those totals. The Reports UI requests aggregate portfolio summary values rather than downloading every portfolio row. Legacy ListOrders and full Reports portfolio responses remain available for existing integrations/fixtures. No data migration is required.


## 15 October 2026 — reliability and local-account amendment

Implemented schema 7 full-entry corrections (stage work/cost, receipt, outside service and separate costs), manual portable backup/restore with integrity checks and a pre-restore safety backup, and a separate local owner account with Argon2id password hashing, offline recovery and macOS Keychain Remember me. Native business commands require authentication; the local development HTTP adapter is not part of the desktop release. Restoring business records preserves the device's account and revokes its active/remembered session. Reports post reversals on their correction date. Current financial totals exclude voided service charges; corrected receipts no longer inflate late/instalment statistics.

See docs/OPERATIONS.md and docs/MAC_HANDOFF.md for behavior, verification evidence and the on-device acceptance checklist. Real Mac installer/Keychain/file-dialog/update/restore checks are still pending. Automatic backup scheduling, order/item editing and quote revisions, partial refunds, dispatches and imports/exports remain deferred. Phone work remains deferred. Do not mark production acceptance complete based on Linux or browser checks.
