# Validation record

18 October 2026, mobile version 0.1.0, implementation commit `e0302d8`.

- Production frontend build and TypeScript checks passed.
- Four frontend/native capability tests passed.
- Nine Rust workflow tests passed; Rust formatting and Clippy passed with warnings rejected.
- Four Chromium phone workflow tests passed locally and on GitHub Actions, including mixed stage pricing, atomic deposit creation, quote revisions, receipt reversals, native-call fixture saves with browser networking disabled, backup/restore, safety undo, damaged-file rejection, unsaved-change confirmation and stage snapshot preservation.
- Linux native host compilation passed using the isolated native-check container.
- GitHub macOS runner built an iOS simulator application, installed/launched it and received its first snapshot through the native Tauri bridge. The fresh snapshot contained zero clients, orders and payments, and four stage templates. The workflow saved a startup screenshot and simulator ZIP.
- The separate local browser preview was started on port 1430 with an empty mobile development workspace.

[Build and test evidence](https://github.com/youssef7511/mouha_project/actions/runs/39150757571).

The simulator artifact is not a physical-iPhone installer. Signing/provisioning, real-phone Files backup/restore, keyboard/viewport behaviour, airplane-mode restart and install/update persistence still require physical-device acceptance. No physical-device pass or production distribution is claimed. See [acceptance](ACCEPTANCE.md).
