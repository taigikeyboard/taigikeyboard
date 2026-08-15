#!/usr/bin/env bash
# Build `RustTaigi.xcframework` and copy it (plus the swift-bridge generated
# `RustTaigi.swift`) to the committed stable path `ios/RustEngine/`.
#
# Usage:
#   build-xcframework.sh           # release (no panic-injector)
#   build-xcframework.sh --dev     # dev (with --features panic-injector for T1)
#
# Run the release variant before any TestFlight build; run the dev variant
# before `xcodebuild test` so RustEngineBridgeTests.test_T1_* can drive the
# Rust catch_unwind boundary.

set -euo pipefail

DEV=0
if [[ "${1:-}" == "--dev" ]]; then
    DEV=1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ENGINE_DIR="$REPO_ROOT/engine"

# Header/modulemap layout + Swift wrapper patching are shared with the macOS
# build; both platforms link the same swift-ffi crate.
source "$SCRIPT_DIR/lib/swift-bridge-artifacts.sh"
OUT_DIR="$ENGINE_DIR/target/d9.2-out"
FRAMEWORK_NAME="RustTaigi"
LIB_NAME="librust_taigi.a"
DEST_DIR="$REPO_ROOT/ios/RustEngine"

cd "$ENGINE_DIR"

# Deterministic clean per plan v3 H3 — replaces freshness checks with one
# full rebuild per invocation (xcframework build is < 60s on M-series).
rm -rf "$OUT_DIR"
mkdir -p "$OUT_DIR"

CARGO_FLAGS=(build --release -p swift-ffi)
if [[ $DEV -eq 1 ]]; then
    CARGO_FLAGS+=(--features panic-injector)
fi

for triple in aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios; do
    cargo "${CARGO_FLAGS[@]}" --target "$triple"
done

# lipo the two simulator slices into a single fat archive.
SIM_LIB="$OUT_DIR/librust_taigi-sim.a"
lipo -create \
    "$ENGINE_DIR/target/aarch64-apple-ios-sim/release/$LIB_NAME" \
    "$ENGINE_DIR/target/x86_64-apple-ios/release/$LIB_NAME" \
    -output "$SIM_LIB"

DEVICE_LIB="$ENGINE_DIR/target/aarch64-apple-ios/release/$LIB_NAME"

# Same argv as the device build above, so cargo resolves the OUT_DIR of the
# exact fingerprint that produced $DEVICE_LIB (release vs panic-injector).
BRIDGE_OUT_DIR="$(swift_bridge_find_out_dir "$ENGINE_DIR" "${CARGO_FLAGS[@]}" --target aarch64-apple-ios)"

HEADERS_DIR="$OUT_DIR/Headers"
swift_bridge_stage_headers "$BRIDGE_OUT_DIR" "$HEADERS_DIR" "$FRAMEWORK_NAME"

xcodebuild -create-xcframework \
    -library "$DEVICE_LIB"  -headers "$HEADERS_DIR" \
    -library "$SIM_LIB"     -headers "$HEADERS_DIR" \
    -output "$OUT_DIR/$FRAMEWORK_NAME.xcframework"

swift_bridge_stage_wrappers "$BRIDGE_OUT_DIR" "$OUT_DIR" "$FRAMEWORK_NAME"

# Idempotent copy to the committed stable path.
mkdir -p "$DEST_DIR"
rm -rf "$DEST_DIR/$FRAMEWORK_NAME.xcframework"
cp -R "$OUT_DIR/$FRAMEWORK_NAME.xcframework" "$DEST_DIR/$FRAMEWORK_NAME.xcframework"
cp "$OUT_DIR/$FRAMEWORK_NAME.swift" "$DEST_DIR/$FRAMEWORK_NAME.swift"
cp "$OUT_DIR/SwiftBridgeCore.swift" "$DEST_DIR/SwiftBridgeCore.swift" 2>/dev/null || true

# `panic_for_test` symbol is always present in the swift-bridge surface
# because the macro does not allow `#[cfg]` on bridge items. The function
# body is `#[cfg]`-gated: in release it returns FAIL_INVARIANT instead of
# panicking. Production app code never references the symbol; the test
# target does. The Android side enforces strict symbol absence in release.
echo "release/dev symbol surface (informational):"
nm "$DEVICE_LIB" 2>/dev/null | grep -E '(process_request_bytes|install_logger_sink|set_log_level|panic_for_test)' || true

if [[ $DEV -eq 1 ]]; then
    echo "ok — DEV RustTaigi.xcframework (panic-injector ON) staged at $DEST_DIR"
else
    echo "ok — RELEASE RustTaigi.xcframework staged at $DEST_DIR"
fi
