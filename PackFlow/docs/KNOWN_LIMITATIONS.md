# Prototype limitations and production migration notes

## This is the design reference

The source uses global JavaScript state, HTML template strings, inline event handlers, and browser localStorage. It demonstrates the intended screens and some domain behavior; it is not a production persistence, authorization, accounting, payroll, or migration architecture.

Keep its visual system and useful behavior. Rebuild the implementation in the agreed production stack instead of wrapping the existing JavaScript as the finished desktop app.

## Data and calculations

- Projects, clients, inventory, and finance are stored under separate localStorage keys. Writes across them are not atomic. Production stock movements and project cost entries must commit or roll back together.
- Current values use GBP with two decimal places and some JavaScript floating-point calculations. Production must configure currency and precision and avoid floating-point accounting. Not all currencies have two minor-unit digits.
- Amounts and sample names are fictional. No actual browser-edited user data is bundled.
- Outside services currently use one editable set of order fields and generate a linked supplier cost in memory. Production needs stable service records and defined behavior for editing, disabling, posting, and reversing them.
- Fulfillment currently uses a cumulative served count and the final Shipped stage. Production should record shipment events or a carefully audited cumulative quantity, with one authoritative source and reversible corrections.
- A project can be fully served even if its earlier stage checkboxes are incomplete. Decide whether to warn, allow, or restrict this without coupling fulfillment to payment.
- The prototype seeds historical material/labor expenses independently of its inventory snapshot. Do not deduct seeded or imported historical quantities again during migration.
- Stock has no purchase/receipt costing history, reservations, returns ledger, or transactional adjustments. Consumption costs currently accept a manual unit price.
- Payroll snapshots are stored per worker/month. Salary payments are records of money paid elsewhere; no transfers occur. There are no statutory tax calculations, timesheet approvals, pay-slip rules, or accounting integrations.
- Salary allocations in project costs must not be added again as another company-wide payroll expense when reporting totals.
- Financial history is simplistic: audit metadata, invoice locking, transaction corrections, duplicate prevention, cancellation handling, and formal invoice documents remain to be planned.

## Screens and exports

- The Reports chart, activity feed, weekly goal, and some trend annotations use sample data. Production must derive them from dated records.
- CSV export is implemented. CSV/XLSX import, XLSX export, PDF generation, label printing, and invoice numbering are not implemented.
- Clients and workers have basic forms. Production needs robust edit/archive workflows, references, validations, empty states, and error recovery.
- Fonts are requested from Google Fonts. Production assets must work offline.
- The layout was designed around 1440 × 900. Browser-based visual verification was unavailable during the prototype build. Verify the React rebuild in browsers and the actual macOS WKWebView at standard and Retina scales.

## Storage and operations

- There is no SQLite database, Rust backend, Tauri shell, backup scheduler, installer, signing, notarization, or automatic updater yet.
- Changing preview origin or clearing browser storage affects prototype data. The production app must use a stable OS app-data directory independent of the executable, development folder, and app updates.
- A locally stored SQLite file does not create a shared multi-Mac application. Confirm whether workers share one Mac or require synchronized independent devices.
- Do not make a live SQLite file a multi-user database by putting it in a shared/cloud-sync folder.
- Backups of a live SQLite database need a consistent snapshot. Copying only the `.db` file while WAL-mode writes are active can miss data.
- Automatic backups while the application is closed require an explicitly planned macOS scheduling component; in-app scheduling alone is insufficient.

Use the existing test cases as regression examples, not as a substitute for repository transaction tests, migration tests, end-to-end tests, restore drills, or native macOS checks.
