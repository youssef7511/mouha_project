#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
test "$(uname -s)" = Darwin
xcodebuild -version
npm ci
npm run build
npm test
rustup target add aarch64-apple-ios-sim
npm run tauri -- ios init --ci
npm run tauri -- ios build --target aarch64-sim --debug --no-sign --ci
mkdir -p artifacts
app_path=$(find src-tauri/gen/apple -type d -name '*.app' -print -quit)
test -n "$app_path"
ditto -c -k --keepParent "$app_path" artifacts/PackFlow-Mobile-iOS-Simulator.zip
device_id=$(xcrun simctl list devices available -j | node -e 'let s="";process.stdin.on("data",d=>s+=d).on("end",()=>{const d=Object.values(JSON.parse(s).devices).flat().find(d=>d.isAvailable&&d.name.startsWith("iPhone"));if(!d)process.exit(1);process.stdout.write(d.udid)})')
xcrun simctl boot "$device_id" || true
xcrun simctl bootstatus "$device_id" -b
xcrun simctl install "$device_id" "$app_path"
SIMCTL_CHILD_PACKFLOW_MOBILE_SMOKE=1 xcrun simctl launch "$device_id" com.packflow.mobile
container_path=$(xcrun simctl get_app_container "$device_id" com.packflow.mobile data)
marker=""
for attempt in {1..30}; do
  marker=$(find "$container_path" -name mobile-smoke-ok.json -print -quit)
  if [ -n "$marker" ]; then break; fi
  sleep 1
done
xcrun simctl io "$device_id" screenshot artifacts/phone-launch.png
test -n "$marker"
cp "$marker" artifacts/native-startup-check.json
node -e 'const d=JSON.parse(require("fs").readFileSync("artifacts/native-startup-check.json","utf8"));for(const [k,v] of Object.entries({clients:0,orders:0,payments:0,templates:4}))if(d[k]!==v)throw Error(`Unexpected fresh workspace: ${k}`);console.log("iOS native startup and fresh workspace passed")'
shasum -a 256 artifacts/PackFlow-Mobile-iOS-Simulator.zip
