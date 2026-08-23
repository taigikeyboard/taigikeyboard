#!/usr/bin/env bash
# Build the universal macOS `RustTaigi.xcframework` (arm64 + x86_64) and stage it,
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
# The swift-bridge post-processing itself lives in
# scripts/lib/swift-bridge-artifacts.sh, shared with the iOS build.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ENGINE_DIR="$REPO_ROOT/engine"

# Header/modulemap layout + Swift wrapper patching are shared with the iOS
# build; both platforms link the same swift-ffi crate.
source "$SCRIPT_DIR/lib/swift-bridge-artifacts.sh"
# Staging dir is macOS-specific so a concurrent iOS build (target/d9.2-out)
# cannot race this script's `rm -rf`.
OUT_DIR="$ENGINE_DIR/target/macos-out"
# One `.pkg` serves both Mac architectures, so both are shipped ABIs — declared
# in engine/rust-toolchain.toml, not installed on the fly. The first is the one
# swift-bridge artefacts are taken from; they are source-level and identical
# across architectures.
TARGET_TRIPLES=(aarch64-apple-darwin x86_64-apple-darwin)
BRIDGE_TRIPLE="${TARGET_TRIPLES[0]}"
FRAMEWORK_NAME="RustTaigi"
LIB_NAME="librust_taigi.a"
DEST_DIR="$REPO_ROOT/macos/RustEngine"
# Matches `.macOS(.v14)` in macos/Package.swift and LSMinimumSystemVersion in
# macos/App/Info.plist. Set explicitly because rustc's per-target default is
# lower and differs between the two architectures — left implicit, the slices
# would claim to support macOS versions the app does not, and the final link
# would have to reconcile two different deployment targets.
export MACOSX_DEPLOYMENT_TARGET=14.0

cd "$ENGINE_DIR"

# Deterministic clean per invocation, matching the iOS script's model.
rm -rf "$OUT_DIR"
mkdir -p "$OUT_DIR"

THIN_LIBS=()
for triple in "${TARGET_TRIPLES[@]}"; do
    cargo build --release -p swift-ffi --target "$triple"
    thin_lib="$ENGINE_DIR/target/$triple/release/$LIB_NAME"
    # Each archive is checked before lipo rather than after: `lipo -create` is
    # happy to fatten two archives of the same architecture, and the result
    # would then fail the set assertion below with nothing to point at.
    thin_arch="$(lipo -archs "$thin_lib")"
    expected_arch="${triple%%-*}"
    # rustc's aarch64 spells itself arm64 in Mach-O.
    [[ "$expected_arch" == "aarch64" ]] && expected_arch="arm64"
    if [[ "$thin_arch" != "$expected_arch" ]]; then
        echo "error: $triple produced a '$thin_arch' archive, expected $expected_arch" >&2
        exit 1
    fi
    THIN_LIBS+=("$thin_lib")
done

# One fat archive, not one xcframework slice per architecture: arm64 and x86_64
# macOS are the same platform, and `-create-xcframework` rejects two libraries
# that resolve to it ("represent two equivalent library definitions"). The
# universal slice is `macos-arm64_x86_64`, and there is still exactly one.
DEVICE_LIB="$OUT_DIR/$LIB_NAME"
lipo -create "${THIN_LIBS[@]}" -output "$DEVICE_LIB"

# Same argv as the build above, so cargo resolves the OUT_DIR of the exact
# fingerprint that produced the archive for $BRIDGE_TRIPLE. The generated header,
# modulemap and Swift wrappers are a source-level interface, so one architecture
# is the whole story — building them twice would only give them room to differ.
BRIDGE_OUT_DIR="$(swift_bridge_find_out_dir "$ENGINE_DIR" \
    build --release -p swift-ffi --target "$BRIDGE_TRIPLE")"

# `set -e` already aborts on a missing file at `cp` time; this loop exists for
# the cases `cp` accepts — an empty or never-finished swift-bridge output —
# before anything is staged.
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
swift_bridge_stage_headers "$BRIDGE_OUT_DIR" "$HEADERS_DIR" "$FRAMEWORK_NAME"

xcodebuild -create-xcframework \
    -library "$DEVICE_LIB" -headers "$HEADERS_DIR" \
    -output "$OUT_DIR/$FRAMEWORK_NAME.xcframework"

swift_bridge_stage_wrappers "$BRIDGE_OUT_DIR" "$OUT_DIR" "$FRAMEWORK_NAME"

# Verify the xcframework really carries both macOS architectures before staging
# it. `-create-xcframework` infers platform from the archive's Mach-O metadata,
# so a wrong-triple build would otherwise be caught only at link time — and a
# half-universal framework only at install time, on a Mac nobody here owns.
XCFRAMEWORK_PLIST="$OUT_DIR/$FRAMEWORK_NAME.xcframework/Info.plist"
# The whole set, sorted and compared exactly: a subset check would pass a
# framework that lost x86_64, and indexing `.0` would validate half of one.
SLICE_COUNT="$(plutil -extract AvailableLibraries raw -o - "$XCFRAMEWORK_PLIST")"
SUPPORTED_PLATFORM="$(plutil -extract AvailableLibraries.0.SupportedPlatform raw -o - "$XCFRAMEWORK_PLIST")"
SUPPORTED_ARCHS="$(plutil -extract AvailableLibraries.0.SupportedArchitectures json -o - "$XCFRAMEWORK_PLIST" |
    python3 -c 'import json,sys; print(",".join(sorted(json.load(sys.stdin))))')"
if [[ "$SLICE_COUNT" != "1" || "$SUPPORTED_PLATFORM" != "macos" || "$SUPPORTED_ARCHS" != "arm64,x86_64" ]]; then
    echo "error: expected exactly 1 macos slice carrying arm64,x86_64 — got $SLICE_COUNT slice(s), first = $SUPPORTED_PLATFORM/$SUPPORTED_ARCHS" >&2
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

echo "ok — macOS $FRAMEWORK_NAME.xcframework ($SUPPORTED_PLATFORM/$SUPPORTED_ARCHS) staged at $DEST_DIR"
