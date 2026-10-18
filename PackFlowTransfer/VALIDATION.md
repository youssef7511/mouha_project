# Transfer validation — 19 October 2026

Implementation: `2b1149d`, with mobile test synchronization fix `6357904` and cold-simulator startup handling `d539d65` (desktop 0.4.0, mobile 0.2.0, workbook format 1, desktop schema 8).

- Nine real cross-app tests export actual phone workbooks and import them using the desktop engine. They cover exact mixed pricing, deposit relationships, explicit contact matching, repeat imports after restart, safe quote revisions, Mac work conflicts, receipt corrections, stale previews, invalid amounts, worksheet/formula validation, renamed workbook parts, date-range dependencies, historical edits across Kuwait midnight, unpriced versus zero quotes and safety-backup restore undo.
- Existing desktop core/account/report/payroll/migration tests passed with schema 8. Legacy migration fixtures were updated to remove the new schema when constructing earlier-version test databases.
- Frontend build/type checks and native-command permission tests passed for both apps. Both native hosts compiled in the isolated Linux check containers.
- The desktop browser suite passed 15 tests (the separately enabled preview-auth browser scenario was skipped); the phone suite passed all five tests. Import preview was retested after its search/pagination changes. The phone test waits for revision-dialog dismissal before checking the saved quote, avoiding ambiguous form/summary locators.
- No local business database or workbook is included in source or bundles. Generated examples use temporary test data only.

CI evidence for the final implementation:

- [Phone-to-Mac transfer checks](https://github.com/youssef7511/mouha_project/actions/runs/39176661468)
- [Mac installer builds](https://github.com/youssef7511/mouha_project/actions/runs/39176661418)
- [Mobile checks and iOS simulator](https://github.com/youssef7511/mouha_project/actions/runs/39178011094)

The final transfer workflow, both Mac installer builds and the mobile checks/iOS simulator startup passed. Simulator startup confirmed zero clients, orders and payments plus four default stage templates. iOS simulator ZIPs cannot be installed on a physical phone. Signed iPhone installation, Files/AirDrop export, import into the intended Mac workspace and backup/undo acceptance remain device checks; automated evidence does not claim those physical-device passes.
