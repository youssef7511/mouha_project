# PackFlow Mobile

Standalone offline iPhone application, kept separately from `../PackFlow`. React/TypeScript provides the phone interface; Tauri 2 embeds it with a Rust domain core and a private SQLite database. Installed builds do not need Docker, a server, the Mac, or internet access.

## Implemented

- Clients: contacts, search, editing and archive.
- Orders: multiple box types, free-text dimensions, quantities, production and payment due dates, notes and draft/confirmed status.
- Stage customer pricing: unpriced, fixed total, individual box-type rates, or quantity × price per Unit. Editable/reorderable default stages: Lamination, Impose (printing), Diecut, Finish. Each saved order keeps its stage names and quote history.
- Exact KWD calculations: integer fils, three decimal places, checked totals and half-up line rounding. Unknown prices stay unknown rather than becoming zero.
- Order-linked deposits and receipts, balances/credits, search, and audited full receipt reversals. A deposit and its new order commit together.
- Order revisions require a reason and preserve prior quotes. Optimistic versions reject stale updates. Archives retain orders, debts and payment history.
- Mobile-only `.pfmobile` backups, validated restore, and a safety backup before replacement. On iPhone, saved backups are accessible through Files → On My iPhone → PackFlow Mobile → PackFlow Mobile Backups. Copy backups outside the phone regularly.

First launch creates an empty workspace plus the four default stage definitions. Test fixtures are temporary and are not included in the application.

## Run the browser preview on Windows

From this directory, with Docker running and Node 24 installed:

```powershell
npm ci
docker compose up -d core
npm run dev
```

Open http://localhost:1430 and use a phone viewport. Preview data is isolated in this directory's ignored `.dev-data` folder. The loopback HTTP adapter is a development feature only; installed iOS builds call the same core through local Tauri commands.

## Checks

```powershell
npm run build
npm test
npm run format:check
npm run test:core
docker compose --profile test run --rm native-check
```

For browser checks, start a fresh disposable core and wait for `/api/query` to respond before running Playwright:

```powershell
docker compose --profile test up -d --force-recreate core-e2e
npx playwright install chromium
npm run test:browser
docker compose --profile test stop core-e2e
```

Browser checks use port 1431 and the disposable database on port 4329. They must start with a fresh test service. `PACKFLOW_CHROMIUM_PATH` optionally selects an existing Chromium executable. Core tests cover transaction rollback, duplicate requests, exact pricing, revisions, corrections, archive guards and backup safety undo. The GitHub mobile workflow also builds and launches a fresh iOS simulator application and checks its first native snapshot.

## iPhone build and installation

iOS builds require a Mac with full Xcode, its command-line tools, Node 24 and Rust. Open Xcode once to install components and an iOS simulator runtime. From this directory:

```bash
npm ci
npm run ios:init
npm run ios:dev
```

`scripts/ios-simulator.sh` produces an unsigned debug **simulator** ZIP and checks native startup. A simulator ZIP cannot be installed on a physical iPhone.

For a real phone, connect the phone to the Mac, select an Apple development team/provisioning profile in the generated Xcode project, enable Developer Mode when required, and build for that device. Configure your own signing team; credentials and provisioning files must stay out of Git. TestFlight/App Store distribution requires the corresponding Apple developer setup. Do not send a `.dmg` to an iPhone. See [Tauri iOS prerequisites](https://v2.tauri.app/start/prerequisites/#ios) and [iOS signing](https://v2.tauri.app/distribute/sign/ios/).

## Boundaries

This version has its own records and uses `MOB-<device>-<sequence>` numbers. It does not connect, transfer or sync with the Mac. Its balance covers this phone workspace only. Desktop `.pfbackup` files and Excel files are not mobile restore files. Future transfer needs explicit ID mapping and duplicate-safe payment import; UUIDs, workspace/device IDs and operation IDs are retained for that work.

Protection currently relies on the iPhone's screen lock; an app PIN, Face ID, cloud accounts and remote recovery are not implemented. Backups contain business data and are not encrypted. Keep them in a protected location. Losing the phone without an external backup loses its independent records.

Production expenses, stock, payroll, desktop reports, Excel import/export, fulfillment tracking and Mac login are outside this first phone version. Physical-device testing (including the keyboard, Files backup/restore, airplane-mode restart and signed install/update) is required before daily use. See [architecture](docs/ARCHITECTURE.md) and the [acceptance checklist](docs/ACCEPTANCE.md).
