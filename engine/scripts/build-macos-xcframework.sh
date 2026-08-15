#!/usr/bin/env bash
# Build the macOS `RustTaigi.xcframework` (aarch64-apple-darwin) and stage it,
# with the swift-bridge generated `RustTaigi.swift` / `SwiftBridgeCore.swift`,
# into the committed stable path `macos/RustEngine/`.
#
# The macOS IME (SwiftPM executable, see docs/architecture/macos-roadmap.md D1/D2)
# consumes the xcframework as a `binaryTarget` named `RustTaigi`; the two Swift
# wrappers are compiled as sources of the executable target.
#
# `engine/swift-ffi` is shared with iOS; `make build` runs this script and
# `build-xcframework.sh` together so both platforms' artefacts stay in step.
#
# Repeats swift-bridge post-processing that build-xcframework.sh also has;
# extracting a shared helper is a tracked follow-up (macos-roadmap.md D1).

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ENGINE_DIR="$REPO_ROOT/engine"
# Staging dir is macOS-specific so a concurrent iOS build (target/d9.2-out)
# cannot race this script's `rm -rf`.
OUT_DIR="$ENGINE_DIR/target/macos-out"
TARGET_TRIPLE="aarch64-apple-darwin"
FRAMEWORK_NAME="RustTaigi"
LIB_NAME="librust_taigi.a"
DEST_DIR="$REPO_ROOT/macos/RustEngine"

cd "$ENGINE_DIR"

# Deterministic clean per invocation, matching the iOS script's model.
rm -rf "$OUT_DIR"
mkdir -p "$OUT_DIR"

cargo build --release -p swift-ffi --target "$TARGET_TRIPLE"

DEVICE_LIB="$ENGINE_DIR/target/$TARGET_TRIPLE/release/$LIB_NAME"

# Locate swift-bridge's OUT_DIR for the build we just produced. Cargo keeps
# multiple `swift-ffi-<fingerprint>/out` directories across rebuilds, so pick
# the newest by mtime. The search is scoped to the darwin target tree, so an
# iOS build's output can never be selected.
BRIDGE_OUT_DIR="$(
    find "$ENGINE_DIR/target/$TARGET_TRIPLE/release/build" \
        -type d -name 'out' -path '*/swift-ffi-*' \
        -exec stat -f '%m %N' {} + \
    | sort -rn | head -n 1 | cut -d' ' -f2-
)"
if [[ -z "$BRIDGE_OUT_DIR" ]]; then
    echo "error: swift-bridge OUT_DIR not found under target/$TARGET_TRIPLE" >&2
    exit 1
fi

# `set -e` already aborts on a missing file at `cp` time; this loop exists for
# the cases `cp` accepts — an empty file, or an OUT_DIR the mtime heuristic
# picked that never finished writing — before anything is staged.
for required in \
    "$BRIDGE_OUT_DIR/SwiftBridgeCore.h" \
    "$BRIDGE_OUT_DIR/SwiftBridgeCore.swift" \
    "$BRIDGE_OUT_DIR/$FRAMEWORK_NAME/$FRAMEWORK_NAME.h" \
    "$BRIDGE_OUT_DIR/$FRAMEWORK_NAME/$FRAMEWORK_NAME.swift"
do
    if [[ ! -s "$required" ]]; then
        echo "error: missing or empty swift-bridge output: $required" >&2
        exit 1
    fi
done

HEADERS_DIR="$OUT_DIR/Headers"
mkdir -p "$HEADERS_DIR"
# swift-bridge nests per-bridge artefacts under `<OUT>/<FRAMEWORK_NAME>/`:
#   <OUT>/SwiftBridgeCore.{h,swift}
#   <OUT>/<FRAMEWORK_NAME>/<FRAMEWORK_NAME>.{h,swift}
cp "$BRIDGE_OUT_DIR/SwiftBridgeCore.h" "$HEADERS_DIR/"
cp "$BRIDGE_OUT_DIR/$FRAMEWORK_NAME/$FRAMEWORK_NAME.h" "$HEADERS_DIR/"

cat > "$HEADERS_DIR/module.modulemap" <<EOF
module $FRAMEWORK_NAME {
    header "SwiftBridgeCore.h"
    header "$FRAMEWORK_NAME.h"
    export *
}
EOF

xcodebuild -create-xcframework \
    -library "$DEVICE_LIB" -headers "$HEADERS_DIR" \
    -output "$OUT_DIR/$FRAMEWORK_NAME.xcframework"

cp "$BRIDGE_OUT_DIR/SwiftBridgeCore.swift" "$OUT_DIR/"
cp "$BRIDGE_OUT_DIR/$FRAMEWORK_NAME/$FRAMEWORK_NAME.swift" "$OUT_DIR/"

# swift-bridge does not emit `import` statements; both wrappers reference C
# symbols defined in the xcframework's modulemap-exposed `RustTaigi` module.
# Inject the import so the SwiftPM executable target compiles them without a
# bridging header.
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

# Swift 5.9+ warns when a module declares conformance of an imported type to an
# imported protocol unless the conformance is annotated `@retroactive`.
# swift-bridge does not yet emit the annotation; patch it in.
sed -i '' \
    -e 's/^extension RustStr: Identifiable {$/extension RustStr: @retroactive Identifiable {/' \
    -e 's/^extension RustStr: Equatable {$/extension RustStr: @retroactive Equatable {/' \
    "$OUT_DIR/SwiftBridgeCore.swift"

# Verify the xcframework really carries a macOS arm64 slice before staging it.
# `-create-xcframework` infers platform from the archive's Mach-O metadata, so a
# wrong-triple build would otherwise be caught only at PR2 link time.
XCFRAMEWORK_PLIST="$OUT_DIR/$FRAMEWORK_NAME.xcframework/Info.plist"
# Slice count is asserted too: when a second (x86_64) slice is added, indexing
# `.0` alone would silently validate half the framework.
SLICE_COUNT="$(plutil -extract AvailableLibraries raw -o - "$XCFRAMEWORK_PLIST")"
SUPPORTED_PLATFORM="$(plutil -extract AvailableLibraries.0.SupportedPlatform raw -o - "$XCFRAMEWORK_PLIST")"
SUPPORTED_ARCH="$(plutil -extract AvailableLibraries.0.SupportedArchitectures.0 raw -o - "$XCFRAMEWORK_PLIST")"
if [[ "$SLICE_COUNT" != "1" || "$SUPPORTED_PLATFORM" != "macos" || "$SUPPORTED_ARCH" != "arm64" ]]; then
    echo "error: expected exactly 1 macos/arm64 slice, got $SLICE_COUNT slice(s), first = $SUPPORTED_PLATFORM/$SUPPORTED_ARCH" >&2
    exit 1
fi

# Stage into the committed path via a sibling temp dir so the previous good
# artefacts are only removed once the new ones are fully copied — an aborted
# copy must not leave `macos/RustEngine/` half-updated.
STAGE_DIR="$DEST_DIR/.staging"
rm -rf "$STAGE_DIR"
mkdir -p "$STAGE_DIR"
cp -R "$OUT_DIR/$FRAMEWORK_NAME.xcframework" "$STAGE_DIR/$FRAMEWORK_NAME.xcframework"
cp "$OUT_DIR/$FRAMEWORK_NAME.swift" "$STAGE_DIR/$FRAMEWORK_NAME.swift"
cp "$OUT_DIR/SwiftBridgeCore.swift" "$STAGE_DIR/SwiftBridgeCore.swift"

rm -rf "$DEST_DIR/$FRAMEWORK_NAME.xcframework"
mv "$STAGE_DIR/$FRAMEWORK_NAME.xcframework" "$DEST_DIR/$FRAMEWORK_NAME.xcframework"
mv "$STAGE_DIR/$FRAMEWORK_NAME.swift" "$DEST_DIR/$FRAMEWORK_NAME.swift"
mv "$STAGE_DIR/SwiftBridgeCore.swift" "$DEST_DIR/SwiftBridgeCore.swift"
rmdir "$STAGE_DIR"

echo "ok — macOS $FRAMEWORK_NAME.xcframework ($SUPPORTED_PLATFORM/$SUPPORTED_ARCH) staged at $DEST_DIR"
