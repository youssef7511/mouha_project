# PackFlow

The application source is in [PackFlow/](PackFlow/README.md).

## Mac test installers

Open this repository's **Actions** tab, select **Build Mac test installers**, then open a successful run. Download the `PackFlow-macOS-AppleSilicon-test` or `PackFlow-macOS-Intel-test` artifact to match the Mac. Extract the artifact ZIP and open its DMG.

These builds are named **PackFlow Test**, use a separate application identifier and data directory, and are ad-hoc signed for testing. They are not notarized production releases. No local database, owner account, workbook or business records are shipped. A new installation asks the owner to create an account and starts with empty business lists plus default stage templates. Reinstalling/updating preserves records already entered on that Mac.

Follow [the Mac acceptance checklist](PackFlow/docs/MAC_HANDOFF.md) and [operation guide](PackFlow/docs/OPERATIONS.md). Production signing/notarization and on-device acceptance remain required before a production handoff.

`PackFlow/scripts/mac-build.sh` performs the same test build manually on a Mac; GitHub Actions calls it automatically. Your friend does not need Node, Rust or that script to install the downloaded DMG.
