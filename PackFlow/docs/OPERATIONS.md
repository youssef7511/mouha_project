# PackFlow — local operation guide

## First launch and signing in

Create a username, display name and password (at least 10 characters). There is no shared default password. Save the recovery code separately from the Mac, then sign in. Remember me stores a revocable session secret in the Mac Keychain. Without it, quitting requires another sign-in. Settings offers Change password and Log out. Forgotten passwords can be reset offline with the recovery code; reset produces a new recovery code and revokes remembered access.

This is a single-owner local account. Historical business audit entries retain the existing Manager actor. There is no cloud account or email reset. The login controls application access; it does not encrypt the SQLite file or exported backups. Use the Mac's account and disk protection for files on disk.

## Back up regularly

In Settings, choose Save backup and a **new filename** on an external drive or another safe location. Existing backup files are not overwritten. Keep dated copies, including one before updating the app. Wait for “Verified backup saved” and its path. The portable `.pfbackup` contains the business database plus a checksum. It includes orders, clients, expenses, receipts, salary records and audit history. It excludes this Mac's login credentials and Keychain session.

Backups are manual in this release. Save one at the end of each business day and before major corrections or an update. A copy on the same disk is useful for undoing mistakes but does not protect against losing that disk. Automatic scheduling/retention remains a later milestone.

## Restore

1. Save a fresh backup of the current workspace if possible.
2. In Settings, read the replacement warning, type RESTORE, then choose a `.pfbackup` file.
3. PackFlow checks the file checksum, database integrity, foreign keys and migration history. Invalid or newer backups are rejected before changing live data. Older supported backups are migrated in a temporary copy.
4. A verified safety backup of the current workspace is saved under the app's `data/backups/` directory. If it cannot be saved, restore stops.
5. SQLite replaces the business database atomically. Sign in again using this Mac's existing account, then check totals and representative records. The completion message gives the safety-backup path.

To undo a restore, restore that safety backup through the same workflow. Portable backups have a 512 MiB import limit. A restore brings back records as they were when the backup was made; later changes are available only in the safety backup. Never merge two SQLite files by hand.

If database damage prevents startup, quit PackFlow completely. Preserve the **entire** application data folder, including SQLite WAL/SHM files, before attempting recovery. On the Mac, use Finder's Go to Folder and open `~/Library/Application Support/com.packflow.desktop/` (debug builds use `.desktop.dev`). Rename `data` to a unique name such as `data-damaged-2026-10-15`; do not delete it. Reopen PackFlow to create an empty business workspace, sign in with the existing account, and restore a known-good portable backup in Settings. Retain the damaged folder for diagnosis. If the account file itself is damaged or the location differs, stop and obtain support rather than deleting account files.

## Correct an entry

- Stage Costs or Work: use Correct entry beside the original, enter a correction date and reason, then Confirm reversal. For a work entry with linked costs, reverse those costs first.
- Payments: Correct entry beside a receipt offsets its remaining recorded amount. This is a bookkeeping correction; it does not send money to the customer. Enter the correct receipt afterward if needed.
- Outside services: Correct entry reverses both the customer charge and supplier cost together.
- Separate costs: Correct entry removes the original's financial effect while keeping the row and reason.

The original remains visible. Reversals cannot precede the original date or be repeated. Reports record the negative adjustment on the correction date; current balances/margins recalculate. Stage progress does not change. Salary records already support editing with version checks and history. Stage status/notes and client profiles remain editable through their existing dialogs.

Reversal and replacement are two deliberate steps. Order/item editing, quote revisions, partial refunds, finished quantities and dispatch workflows remain outside this change; do not assume those functions are available.

## Mac acceptance before daily use

Use the checklist in MAC_HANDOFF.md with disposable records. Keep regular backups after acceptance. The Windows browser preview now has its own local account/login; native backup file dialogs still require the desktop app. The preview account is separate from the Mac account.


### Web preview login — 15 October 2026

The local browser preview now asks for account setup/login and shows a Log out button above the workspace. Preview credentials use the same Rust password/recovery implementation, stored separately in `.dev-data/preview-account.sqlite`; existing business records stay intact. Protected preview reads/writes require a valid HttpOnly, SameSite=Strict cookie. Remember me on the browser uses a 30-day cookie; ordinary login uses a session cookie (browser session-restore settings may preserve it). Logout/password reset revoke the token. This does not change the Mac Keychain implementation. Backup/restore file dialogs remain desktop-only.

The automated business-flow fixture may skip authentication only in a debug preview process explicitly configured with `PACKFLOW_TEST_NO_AUTH=1` and `/test-data` as its disposable data directory. The normal `/data` preview does not accept this bypass. The separate `preview-auth.spec.ts` test exercises real login and API rejection with authentication enabled on a fresh isolated server.
