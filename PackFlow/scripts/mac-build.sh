#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "Run this script on the Mac that will build the test installer."
  exit 1
fi
command -v node >/dev/null
command -v npm >/dev/null
command -v cargo >/dev/null
xcode-select -p >/dev/null
echo "Building a PackFlow Mac TEST installer for $(uname -m)."
echo "See docs/MAC_HANDOFF.md for the required on-device acceptance checks."
npm ci
npm run build
npm test
cargo test --locked -p packflow-core
# Default is ad-hoc signing for a private test build. This is not notarization.
export APPLE_SIGNING_IDENTITY="${APPLE_SIGNING_IDENTITY:--}"
npm run tauri -- build --bundles app,dmg --config src-tauri/tauri.test.conf.json
echo "Build finished. Default output: target/release/bundle/dmg/"
echo "Install and complete docs/MAC_HANDOFF.md before daily business use."
