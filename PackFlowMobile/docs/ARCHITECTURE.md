# Mobile architecture and data rules

## Ownership and runtime

`src/` is a phone-specific React interface with four primary tabs. Fonts and assets are bundled locally. `src-tauri/` is the iOS/native host. Its four local commands (query, mutate, backup, restore) are explicitly declared in the native manifest and granted to the local `main` window. Remote pages have no command capability. The installed application has no HTTP server or cloud dependency.

`crates/mobile-core/` owns validation, SQLite, pricing, transactions and backup validation. A mutex serializes operations; native database work runs off the UI thread. Preview HTTP is gated behind the Cargo `preview` feature. It binds loopback by default; Docker publishes only loopback. Browser preview is not a hosted mobile product or synchronization endpoint.

SQLite uses foreign keys, WAL, FULL synchronization and a busy timeout. Native data lives in the application data directory under the `com.packflow.mobile` identity. User-accessible backups live under Documents/PackFlow Mobile Backups. This is independent of all desktop application paths, migrations, databases and login data.

## Data

| Table | Purpose |
| --- | --- |
| meta | Schema fingerprint, workspace/device UUIDs, workspace name and next mobile number |
| clients | Contact details, optimistic version and archive flag |
| templates | Editable stage definitions, order, version and archive flag |
| orders | Current document with stable box UUIDs, selected stage snapshots, computed quote and version |
| revisions | Saved quote documents, reason, timestamp and originating device |
| payments | Positive receipt entries and full linked correction entries; no destructive editing |
| requests | Operation UUID, canonical action hash and committed response for retry deduplication |
| audit | Before/after records, action, reason, device and timestamp |

Commands recalculate prices in Rust rather than trusting frontend totals. Quote preview uses matching TypeScript integer arithmetic and parity fixtures. A stage can be unpriced (unknown), fixed, per box type or per Unit. Item rates are associated with stable box UUIDs; removing/reordering a box cannot silently attach its price to another box. A complete order quote exists only when every selected stage is priced; a zero fixed price is a deliberate valid quote.

KWD amounts use integer fils. Unit quantities have three decimals represented as scaled integers. Multiplication uses wider intermediate integers and rounds each line half up to a fils; sums and public inputs have checked bounds. Orders support up to 100 box types and 50 selected stages; box quantity is a positive integer up to 1,000,000. Sheets and dimensions remain descriptive text.

## Saves and corrections

Mutations use an immediate transaction containing all affected records, audit entries and the operation response. An identical request ID/action returns its committed response; reusing that ID with a different action is rejected. The UI keeps a pending ID for an unchanged failed action. Starting a new order and receiving its deposit is one transaction: invalid payment input leaves neither an order nor a consumed number.

Order changes require the current version and a reason. Saved stage names stay unchanged when a template is renamed/archived. The client cannot be replaced after any payment history. Soft archives retain balances and receipts and prevent new receipts on archived orders. Overpayments remain visible customer credit. Unpriced balances stay unknown even when receipts exist. Correcting a receipt appends exactly one full reversal, requires a reason/date and retains the original; a replacement is a new receipt.

Payments attach to an order. Unallocated client credits, partial reversal, deleting receipts and synchronizing balances are not implemented. Mobile operational statuses are draft/confirmed; they are not representations of completed Mac production or fulfillment stages.

## Backup and restore

`.pfmobile` contains a format marker, SHA-256 checksum and SQLite Backup API snapshot, limited to 25 MiB of database data. This is a snapshot format, not Excel, a merge, a sync queue or a desktop backup. The checksum detects corruption; it is not encryption or proof of origin.

Restore requires typing RESTORE, validates the complete candidate before replacement (integrity, foreign keys, application/schema identity, schema catalog/fingerprint, recalculated current quotes and correction relationships), then saves the previous workspace to a distinct safety backup. SQLite's backup transaction replaces the live database. Restore is serialized against all writes. Restoring the safety file provides undo; each restore also creates a new safety file.

Backups preserve workspace/device identity and request history. Restoring copies onto two actively edited devices is not a supported sharing mechanism. Future transfer must specify ownership, number assignment, duplicate detection, archive handling and conflict review before either app imports another device's records.
