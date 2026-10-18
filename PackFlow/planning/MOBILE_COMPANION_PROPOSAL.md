# PackFlow iPhone companion — proposal

## Manual Excel transfer implemented — 19 October 2026

The user approved one-way phone → Mac Excel transfer with date-range selection. Phone version 0.2 exports a strict versioned workbook; Mac version 0.4 provides Import from phone on Clients, Orders, Payments and Settings. Clients are explicitly matched, new orders receive Mac PF numbers, quotes are recalculated, duplicate source records are skipped and conflicts preserve Mac work. Related older records are included automatically. Preview uses a disposable database; apply requires a current preview and saves a safety backup before an atomic replacement. No cloud or automatic/reverse sync is introduced. See [the transfer contract and usage guide](../../PackFlowTransfer/README.md) for the workbook sheets, limits and conflict rules.

The historical standalone scope below predates this manual transfer addition. Mobile backups remain separate from Excel transfer, and signed physical-iPhone acceptance remains required.

## Confirmed direction — 18 October 2026

The user selected **standalone offline first, transfer later**, then confirmed **full box details and stage pricing** for phone order entry. Version 1 will keep its own on-device records, with Clients, Orders and Customer receipts/balances. Phone orders must support multiple box types, descriptions/dimensions, quantities, selectable stages and stage-by-stage customer pricing. Implementation is now in the separate `../PackFlowMobile` directory, with audited order revisions, receipt reversals and mobile backup/restore. No automatic connection to the Mac, server, cloud hosting, synchronization or transfer bridge is part of this first version. The phone app implementation is present; signed physical-iPhone acceptance remains required. See [the mobile README](../../PackFlowMobile/README.md).

Confirmed order-pricing scope: retain the editable default stage sequence Lamination → Impose (printing) → Diecut → Finish; snapshot the selected stages on each order. Use the desktop pricing modes: unpriced, fixed stage total, per-box prices for each box type, or a unit quantity and unit price. Sum known stage prices, preserve unpriced/unknown totals, and use exact integer-fils arithmetic for KWD. Sheet quantities/dimensions remain entered descriptive text, and generic Unit remains the standard. These are customer quotes; customer receipts remain separate from production expenses.

Recommended phone screen flow: choose/create a client → add one or more box types → select stages and enter their prices → review the quote → save locally → optionally record an order-linked deposit/receipt. The user chose the full order form rather than a minimal description-only draft. Production expense management, payroll and inventory are not implied by this pricing choice. The initial implementation preserves quote revisions with a reason and corrects mistaken receipts through a full linked reversal; replacement payments are new entries.

Plan stable UUIDs and a device identity now so a future transfer can identify the same record without duplication. Phone draft identifiers must remain distinct from official Mac PF numbers until an import/numbering policy is implemented. A standalone phone balance covers that phone's records only and must not be presented as the complete Mac balance. The new desktop Excel export is for analysis, not a duplicate-safe phone transfer/import format.

If online synchronization is added later, both devices retain local databases and send uniquely identified operations to a private authenticated HTTPS server. The phone can save offline, upload when online while the Mac is off, and the Mac downloads accepted changes on reconnect. The shared service deduplicates payments, validates workspace ownership and versions, assigns official numbering and sends acknowledgements. Editing conflicts need review; payments need audited corrections. Hosting, server backups, secure accounts, migrations and ongoing operation become additional responsibilities. Sync does not replace backups or justify sharing a live SQLite file.

Implemented V1 rules: searchable client contacts; full multi-box draft/confirmed orders with optional deposits; order-linked receipts and overpayment credit; audited revisions/full receipt reversals; device screen-lock protection; and independent mobile backups with safety restore. App-specific login/Face ID is deferred. Signed iPhone installation and physical-device acceptance remain distribution tasks. Transfer design can follow separately, but retain provenance/UUIDs from the start.

15 October 2026. Updated direction: defer phone implementation and all synchronization work while completing the Mac application. The user is considering a small standalone offline phone app for rapid entry first, with a connected version later. A standalone phone app would keep its own data; transfer or local pairing with the Mac would require an explicit additional workflow. This paragraph records the earlier deferral. The separate standalone phone app is now implemented; cloud service, pairing and an import bridge remain deferred.

The connected architecture below is retained as a future option, not the current implementation scope. The earlier offline/online-anywhere requirement applies if that connected option is resumed.

## Recommended product

Build a small iPhone companion with Clients, Orders and Customer receipts/balances. Keep production stages, supplier expenses, monthly salaries, detailed reports and administration in the Mac app initially. Share identities and business rules, while giving the phone its own touch-oriented layout. The current desktop window/layout and desktop adapter need mobile-specific changes; this is not a conversion of a DMG into an iPhone application.

Example: create a client and a draft order during a visit, record a deposit, then open that same draft on the Mac to complete box items, stage pricing and production details. Add a proper draft/edit/quote-revision workflow on the Mac before claiming this handoff works end-to-end.

## Offline plus anywhere sync

Each device saves local data and an outgoing operation queue. A private authenticated HTTPS service accepts operations and makes accepted records available to the other device. The Mac does not need to remain switched on for the phone to save locally or send operations to the service. The Mac receives accepted changes when it reconnects. Initial sign-in/setup and downloading existing records require connectivity; offline access covers records already downloaded and new local drafts.

Sync on launch, manual Sync now and when connectivity becomes available while the app is active. Show queued, synced, rejected/conflicting states and last successful sync. Do not promise that iOS always runs synchronization instantly while the app is suspended.

Receipts are append-only operations with a unique operation ID. The service deduplicates retries, validates order/workspace ownership and amounts, and acknowledges an accepted receipt exactly once. Until acknowledgement, show Pending sync and distinguish the device's provisional balance from confirmed shared balance. Corrections must be explicit audited events, not overwriting financial history. Client/order metadata conflicts need version checks and a review path; do not resolve monetary conflicts by blindly keeping the last edited value.

Use stable UUIDs for drafts and relationships. Define one authoritative assignment of official PF order numbers; devices must not independently assign conflicting sequence numbers offline. Preserve operation IDs, actor/device provenance, currency precision, business dates and server acceptance order. Reuse existing integer-fils/scaled-value fixtures and extract reusable rules from the SQLite-specific domain core where needed.

## Suggested technology direction

Evaluate Tauri 2 iOS first, keeping React/TypeScript and reusable Rust validation/arithmetic. Add a phone interface, iOS bootstrap/configuration and platform-specific plugin handling. iOS builds/testing/signing require a Mac with full Xcode. Keep SQLite local to each device. Introduce a production sync service with actual authentication/workspace access control; the current loopback-only development HTTP adapter and local actor selector are not that service.

A Rust API and a central transactional store are a reasonable direction. Decide whether the server owns a workspace SQLite database or uses PostgreSQL after defining synchronization ownership and deployment needs. Do not commit to a database migration merely because a phone interface is requested. The server must be the authority for accepted shared operations, numbering and duplicate-safe financial events. This is new architecture work, not just a mobile skin on the current desktop repository.

Sync records/operations through the service rather than sharing a live SQLite file through iCloud/Dropbox or a network folder. Backups remain separate from synchronization: replication also propagates mistakes and does not replace recoverable snapshots.

## Alternatives and their limits

| Approach | Fit for the confirmed requirement |
| --- | --- |
| Manual export/import later | Can support disconnected capture, but transfer, identity mapping and duplicate-safe receipt import need new work and user review. No automatic connection. |
| Direct sync to the Mac on local Wi-Fi | Useful without a hosted service, but cannot sync from anywhere while the Mac is off or unreachable. Requires pairing/authentication and a Mac service. |
| Offline local apps with a private cloud sync service | Recommended: capture offline, sync from anywhere when online, and let the Mac receive changes later. Requires hosting, authentication, sync/conflict tests and backups. |

## Delivery sequence

1. Complete Mac reliability essentials: backups/restore, correction workflows and editable drafts; perform a real Mac build/install/restart/restore check.
2. Define shared operation IDs, numbering, authoritative state, authentication, conflicts and receipt acknowledgement rules.
3. Build and test the limited phone UI and durable offline queue alongside the shared service.
4. Add desktop synchronization and draft completion. Test both devices editing, network loss/retry, repeated receipts, one device offline, server rejection and restoration.
5. Distribute a signed iPhone test build and exercise the real daily workflow before expanding mobile features.

## References

- [Tauri iOS prerequisites](https://v2.tauri.app/start/prerequisites/#ios)
- [Tauri iOS signing](https://v2.tauri.app/distribute/sign/ios/)
- [Tauri single-instance plugin platform setup](https://v2.tauri.app/plugin/single-instance/)
- [SQLite WAL requires device-local shared state](https://www.sqlite.org/wal.html)
