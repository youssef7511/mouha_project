# PackFlow — Mac test-build handoff

Reviewed 15 October 2026. Status: suitable for handing source to a Mac developer for a test build. Production release acceptance is incomplete; no Mac `.app` or `.dmg` has been built or run in this Windows workspace.

## Current evidence

- Frontend production build and TypeScript check passed.
- Latest Rust/SQLite/account suite: see the verification addendum below.
- Latest browser suite: nine tests passed against the real core, including search/pagination beyond 200 orders.
- Frontend arithmetic suite: six tests passed.
- Desktop Rust/Tauri/config compile check was rerun successfully in Linux on 15 October. This is not a macOS packaging or runtime test.
- Tauri configuration includes `.app` and `.dmg`, bundled icons and macOS 14.0 minimum. Production data uses the app's own data directory. Docker/Vite are development tools and are not intended runtime dependencies of the desktop release.

## What to send

Send the source project and lockfiles. Exclude generated `node_modules/`, `target/`, `dist/`, `test-results/` and development `.dev-data/`, along with any private environment files. Keep the existing development database/journal/backups separately. A source handoff should not silently include business records or demo data. The prepared source-only ZIP is a test-build handoff; business records must remain local.

## Build on the friend's Mac

Use macOS 14 or later. Install Node/npm compatible with the project's pins, Rust via rustup (the repository selects its toolchain), and Xcode Command Line Tools for desktop development. Full Xcode will also be required for the later iPhone target. First-time dependency installation requires internet.

From the PackFlow project root:

```sh
npm ci
npm run build
cargo test --locked -p packflow-core
npm run tauri -- dev
```

For a test installer built for that Mac's CPU:

```sh
npm run tauri -- build --bundles app,dmg --config src-tauri/tauri.test.conf.json
```

With the default Cargo target directory in this workspace, artifacts are under `target/release/bundle/macos/` and `target/release/bundle/dmg/`. Custom target-directory settings may change that location. A native build for one CPU does not establish compatibility with the other Mac CPU family. Universal packaging and verification remain a release task.

For ad-hoc signed private test distribution, Tauri documents `-` as the signing identity. This does not provide Developer ID notarization or remove all macOS installation warnings:

```sh
APPLE_SIGNING_IDENTITY="-" npm run tauri -- build --bundles app,dmg --config src-tauri/tauri.test.conf.json
```

Broader distribution requires an appropriate signing/notarization setup. No signing credentials are included or assumed.

## Mac review before daily business use

Install from the generated DMG, open from Applications, verify native database commands, and create test clients/orders/quotes/stage expenses/receipts/monthly salaries. Close and reopen the application and confirm identical amounts and data. Repeat offline with Docker/Vite stopped. Verify dialogs, keyboard behavior, fonts, pagination, second-launch focus and an application update preserving data. Debug and packaged production builds use different databases, so an empty packaged workspace is expected unless data is deliberately initialized.

Manual backup/restore, full-entry ledger corrections and local login are implemented and need the real Mac acceptance drill below. Quote/order editing remains incomplete, including completing phone-created drafts. Finished quantities, dispatches, refunds, imports/exports and the remaining release gates are tracked in `planning/ACCEPTANCE_CRITERIA.md`.

## Official references

- [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)
- [DMG packaging on a Mac](https://v2.tauri.app/distribute/dmg/)
- [macOS signing, notarization and ad-hoc signing](https://v2.tauri.app/distribute/sign/macos/)


## Reliability and account acceptance addendum — 15 October 2026

The source now includes schema 7, verified portable backups with automatic pre-restore safety copies, full-entry reversals for stage work/costs, receipts, outside services and separate costs, and a native local owner account. Read [OPERATIONS.md](OPERATIONS.md) before the drill. The browser development adapter has a separate local preview account and no native file dialogs; it is excluded from desktop release builds.

On the real Mac, record pass/fail for each item before approving daily use:

- Fresh installation asks for a personal account before any records are accessible. Save its recovery code; verify wrong-password rejection and correct login. Direct native query/mutate calls while logged out must fail.
- Quit/reopen without Remember me: login required. Repeat with Remember me: account resumes through Keychain. Log out and restart: remembered session must be invalid. Deny Keychain access: password login remains available. Change password and test the old password/token fail. Recover offline using the code; the old recovery code must no longer work.
- Create representative clients, mixed-item orders, stage costs/work, a service, a receipt and monthly salaries. Save a portable backup to a separate location. Add more data. Restore the backup; verify original amounts/history and that later additions are absent. Restore the automatically created safety backup and verify those later additions return. Restart after both restores.
- Cancel both file dialogs; verify nothing changes. Reject damaged/truncated and newer backups without changing current data. A backup to an unwritable folder must show failure, not success. Attempting to save over an existing filename is rejected.
- Correct a stage expense/work entry, receipt, service and separate cost. Verify original and correction remain visible, duplicate reversal is rejected, negative adjustments appear on their correction date, and stage progress is unchanged. Verify reversing a service removes both its charge and supplier cost.
- Run offline from Applications with Docker and Vite stopped. Test second launch, keyboard/dialog behavior, both intended CPU families, and a manual application update preserving both account and business data.

Not yet accepted: real Mac Keychain/file dialogs, signed installer/runtime tests, both CPU families and an on-device restore drill. No `.dmg` is produced by the Windows handoff. This source package is for completing those checks; daily-use approval remains pending them. Automatic backups, partial refunds, order/quote edits, dispatch and imports/exports are still deferred.


## Verification recorded for this handoff

- Production frontend build and TypeScript: passed.
- Rust/account/SQLite tests: 42 passed (including backup round-trip and safety undo, invalid-checksum rejection, correction idempotency, password recovery and persisted login cooldown).
- Browser regression tests: nine existing flows passed; two additional correction/account-screen flows passed. The account-screen test uses an IPC fixture, not a real Keychain.
- Frontend arithmetic: six tests.
- Linux native adapter/config compile: passed; this does not compile or exercise the macOS-specific Keychain branch.
- Core Clippy with warnings denied: passed.

On the Mac, after installing the prerequisites, the repeatable build command is `bash scripts/mac-build.sh`. It uses ad-hoc signing by default for a private test build and builds for the host CPU. This does not establish notarization or compatibility with the other CPU family.


### Web preview login — 15 October 2026

The local browser preview now asks for account setup/login and shows a Log out button above the workspace. Preview credentials use the same Rust password/recovery implementation, stored separately in `.dev-data/preview-account.sqlite`; existing business records stay intact. Protected preview reads/writes require a valid HttpOnly, SameSite=Strict cookie. Remember me on the browser uses a 30-day cookie; ordinary login uses a session cookie (browser session-restore settings may preserve it). Logout/password reset revoke the token. This does not change the Mac Keychain implementation. Backup/restore file dialogs remain desktop-only.

The automated business-flow fixture may skip authentication only in a debug preview process explicitly configured with `PACKFLOW_TEST_NO_AUTH=1` and `/test-data` as its disposable data directory. The normal `/data` preview does not accept this bypass. The separate `preview-auth.spec.ts` test exercises real login and API rejection with authentication enabled on a fresh isolated server.


## GitHub Actions test installers

The repository workflow `.github/workflows/build-mac.yml` runs the same `scripts/mac-build.sh` on Apple Silicon and Intel Mac runners. Open the repository Actions tab, select a successful **Build Mac test installers** run, and download the artifact matching the Mac CPU. Extract its ZIP, open the DMG, and copy **PackFlow Test** into Applications. Node, Rust and Docker are not required on the recipient's Mac.

These builds use `tauri.test.conf.json`, with identifier `com.packflow.desktop.test`. Both business data and Keychain credentials are separate from the future production identifier `com.packflow.desktop`. A fresh installation requires creating a personal account and has empty clients, orders, payments, expenses, workers and salaries. Only default stages/settings are initialized. Development databases, spreadsheets and credentials are excluded and checked before packaging. Installing an update preserves the existing installation's data; it does not reset it.

The manual script remains useful when building directly on a Mac or diagnosing CI. It installs dependencies, runs frontend/core checks, and builds an ad-hoc signed host-CPU test installer. Production Developer ID signing/notarization and the real-device acceptance drill remain pending. Do not use the test workspace for permanent business records.
