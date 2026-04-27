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

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ENGINE_DIR="$REPO_ROOT/engine"
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

# Locate swift-bridge's OUT_DIR for the build we just produced. Cargo keeps
# multiple `swift-ffi-<fingerprint>/out` directories across rebuilds (each
# feature set / config gets its own fingerprint), so picking the first match
# can copy stale headers from a previous build. Pick the newest by mtime.
BRIDGE_OUT_DIR="$(
    find "$ENGINE_DIR/target/aarch64-apple-ios/release/build" \
        -type d -name 'out' -path '*/swift-ffi-*' \
        -exec stat -f '%m %N' {} + \
    | sort -rn | head -n 1 | cut -d' ' -f2-
)"
if [[ -z "$BRIDGE_OUT_DIR" ]]; then
    echo "error: swift-bridge OUT_DIR not found" >&2
    exit 1
fi

HEADERS_DIR="$OUT_DIR/Headers"
mkdir -p "$HEADERS_DIR"
# swift-bridge nests per-bridge artefacts under `<OUT>/<FRAMEWORK_NAME>/`:
#   <OUT>/SwiftBridgeCore.{h,swift}
#   <OUT>/<FRAMEWORK_NAME>/<FRAMEWORK_NAME>.{h,swift}
cp "$BRIDGE_OUT_DIR/SwiftBridgeCore.h" "$HEADERS_DIR/"
cp "$BRIDGE_OUT_DIR/$FRAMEWORK_NAME/$FRAMEWORK_NAME.h" "$HEADERS_DIR/"

cat > "$HEADERS_DIR/module.modulemap" <<EOF
module RustTaigi {
    header "SwiftBridgeCore.h"
    header "$FRAMEWORK_NAME.h"
    export *
}
EOF

xcodebuild -create-xcframework \
    -library "$DEVICE_LIB"  -headers "$HEADERS_DIR" \
    -library "$SIM_LIB"     -headers "$HEADERS_DIR" \
    -output "$OUT_DIR/$FRAMEWORK_NAME.xcframework"

cp "$BRIDGE_OUT_DIR/SwiftBridgeCore.swift" "$OUT_DIR/"
cp "$BRIDGE_OUT_DIR/$FRAMEWORK_NAME/$FRAMEWORK_NAME.swift" "$OUT_DIR/"

# swift-bridge does not emit `import` statements; both wrappers reference
# C symbols defined in the xcframework's modulemap-exposed `RustTaigi`
# module. Inject the import at the top of each wrapper so any consumer
# target picks up the C surface without a project-level bridging header.
inject_import() {
    local file="$1"
    if ! grep -q '^import RustTaigi' "$file"; then
        local tmp
        tmp=$(mktemp)
        if grep -q '^import Foundation' "$file"; then
            awk '{ print } /^import Foundation/ && !injected { print "import RustTaigi"; injected=1 }' "$file" > "$tmp"
        else
            { echo "import RustTaigi"; echo ""; cat "$file"; } > "$tmp"
        fi
        mv "$tmp" "$file"
    fi
}
inject_import "$OUT_DIR/$FRAMEWORK_NAME.swift"
inject_import "$OUT_DIR/SwiftBridgeCore.swift"

# Swift 5.9+ warns when an app declares conformance of an imported type to an
# imported protocol unless the conformance is annotated `@retroactive`.
# swift-bridge does not yet emit the annotation; patch it in.
sed -i '' \
    -e 's/^extension RustStr: Identifiable {$/extension RustStr: @retroactive Identifiable {/' \
    -e 's/^extension RustStr: Equatable {$/extension RustStr: @retroactive Equatable {/' \
    "$OUT_DIR/SwiftBridgeCore.swift"

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
