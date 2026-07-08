#!/usr/bin/env bash
# Build the Swift StoreKit bridge for every Apple Rust target and stage the
# archives under crate-ios/prebuilt/<rust-triple>/libstorekit_bridge.a.
#
# Must run on macOS with Xcode (swiftc + the iOS/macOS SDKs). The archives
# are shipped in the npm package so non-Mac hosts (Linux CI, perry's Linux
# builders) can produce iOS/macOS libraries without a Swift toolchain —
# crate-ios/build.rs links these when swiftc is unavailable.
#
# Keep the swiftc invocation in lockstep with compile_bridge_from_source()
# in crate-ios/build.rs.
set -euo pipefail

cd "$(dirname "$0")/.."

SWIFT_SOURCE=crate-ios/swift/storekit_bridge.swift
PREBUILT_DIR=crate-ios/prebuilt

# rust-triple:swift-target:sdk
TARGETS=(
  "aarch64-apple-ios:arm64-apple-ios16.0:iphoneos"
  "aarch64-apple-ios-sim:arm64-apple-ios16.0-simulator:iphonesimulator"
  "x86_64-apple-ios:x86_64-apple-ios16.0-simulator:iphonesimulator"
  "aarch64-apple-darwin:arm64-apple-macosx13.0:macosx"
  "x86_64-apple-darwin:x86_64-apple-macosx13.0:macosx"
)

for entry in "${TARGETS[@]}"; do
  IFS=: read -r triple swift_target sdk <<<"$entry"
  out_dir="$PREBUILT_DIR/$triple"
  mkdir -p "$out_dir"

  echo "==> $triple (swiftc -target $swift_target, sdk $sdk)"
  xcrun --sdk "$sdk" swiftc \
    -emit-library -static \
    -module-name StoreKitBridge \
    -target "$swift_target" \
    -sdk "$(xcrun --sdk "$sdk" --show-sdk-path)" \
    -framework StoreKit \
    -framework Foundation \
    -o "$out_dir/libstorekit_bridge.a" \
    "$SWIFT_SOURCE"
done

echo
echo "Prebuilt bridges:"
ls -la "$PREBUILT_DIR"/*/libstorekit_bridge.a
