# PackFlow iPhone companion — proposal

15 October 2026. Updated direction: defer phone implementation and all synchronization work while completing the Mac application. The user is considering a small standalone offline phone app for rapid entry first, with a connected version later. A standalone phone app would keep its own data; transfer or local pairing with the Mac would require an explicit additional workflow. No cloud service, local pairing, import bridge or phone app is currently implemented.

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
