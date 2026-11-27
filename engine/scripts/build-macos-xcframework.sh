#!/usr/bin/env bash
# Build the universal macOS `RustTaigi.xcframework` (arm64 + x86_64) and stage it,
# with the swift-bridge generated `RustTaigi.swift` / `SwiftBridgeCore.swift`,
# into the stable path `macos/RustEngine/`.
#
# The macOS IME (SwiftPM executable, see docs/architecture/macos-roadmap.md D1/D2)
# consumes the xcframework as a `binaryTarget` named `RustTaigi`; the Swift
# wrappers are compiled as sources of the `RustTaigiSwift` target.
#
# The archive is `macos/crates/taigi-macos-ffi` (its own workspace, `macos/`),
# hosting the desktop shell seam and logger sink over `taigi-desktop-core` and
# `dispatch` in ONE static library so the engine's process-wide state exists
# once (docs/architecture/macos-desktop-core-roadmap.md D1).
#
# The swift-bridge post-processing itself lives in
# scripts/lib/swift-bridge-artifacts.sh, shared with the iOS build.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
MACOS_DIR="$REPO_ROOT/macos"

# Header/modulemap layout + Swift wrapper patching are shared with the iOS
# build.
source "$SCRIPT_DIR/lib/swift-bridge-artifacts.sh"
source "$SCRIPT_DIR/lib/e2e-trace-guard.sh"
# Under the macOS workspace's own target dir, so a concurrent iOS build
# (engine/target/d9.2-out) cannot race this script's `rm -rf`.
OUT_DIR="$MACOS_DIR/target/xcframework-out"
# One `.pkg` serves both Mac architectures, so both are shipped ABIs — declared
# in macos/rust-toolchain.toml, not installed on the fly. The first is the one
# swift-bridge artefacts are taken from; they are source-level and identical
# across architectures.
#
# TAIGI_MACOS_TARGETS (space-separated triples) narrows the set for a test-only
# build: .github/workflows/macos.yml sets it to the runner's aarch64-apple-darwin,
# since `swift test` links one architecture. Never set it for a release — the
# universal link in macos/scripts/bundle-app.sh then fails on the missing slice.
read -r -a TARGET_TRIPLES <<< "${TAIGI_MACOS_TARGETS:-aarch64-apple-darwin x86_64-apple-darwin}"
BRIDGE_TRIPLE="${TARGET_TRIPLES[0]}"
FRAMEWORK_NAME="RustTaigi"
CRATE="taigi-macos-ffi"
LIB_NAME="libtaigi_macos_ffi.a"
DEST_DIR="$REPO_ROOT/macos/RustEngine"
# Matches `.macOS(.v14)` in macos/Package.swift and LSMinimumSystemVersion in
# macos/App/Info.plist. Set explicitly because rustc's per-target default is
# lower and differs between the two architectures — left implicit, the slices
# would claim to support macOS versions the app does not, and the final link
# would have to reconcile two different deployment targets.
export MACOSX_DEPLOYMENT_TARGET=14.0

cd "$MACOS_DIR"

# Deterministic clean per invocation, matching the iOS script's model.
rm -rf "$OUT_DIR"
mkdir -p "$OUT_DIR"

THIN_LIBS=()
BUILT_ARCHS=()
for triple in "${TARGET_TRIPLES[@]}"; do
    cargo build --release -p "$CRATE" --target "$triple"
    thin_lib="$MACOS_DIR/target/$triple/release/$LIB_NAME"
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
    BUILT_ARCHS+=("$expected_arch")
done
# The slice's architecture set, spelled as the check below reads it from the
# plist: sorted, comma-joined ("arm64,x86_64" for the default pair).
EXPECTED_ARCHS="$(printf '%s\n' "${BUILT_ARCHS[@]}" | LC_ALL=C sort | paste -sd, -)"

# One fat archive, not one xcframework slice per architecture: arm64 and x86_64
# macOS are the same platform, and `-create-xcframework` rejects two libraries
# that resolve to it ("represent two equivalent library definitions"). The
# universal slice is `macos-arm64_x86_64`, and there is still exactly one.
DEVICE_LIB="$OUT_DIR/$LIB_NAME"
lipo -create "${THIN_LIBS[@]}" -output "$DEVICE_LIB"
e2e_trace_assert_absent "$DEVICE_LIB"

# The desktop-shell seam + logger entry points in every slice, and no legacy
# `process_request_bytes` from `swift-ffi`. Apple's nm cannot read the bitcode
# members rustc ships for `compiler_builtins` (built by a newer LLVM) and
# exits 1 after listing every other member; that one error is expected, any
# other is not.
for arch in "${BUILT_ARCHS[@]}"; do
    nm_stderr="$OUT_DIR/nm-$arch.stderr"
    archive_symbols="$(nm -gU -arch "$arch" "$DEVICE_LIB" 2> "$nm_stderr")" || true
    if grep -v 'compiler_builtins-' "$nm_stderr" | grep -q .; then
        echo "error: nm could not read the $arch slice:" >&2
        cat "$nm_stderr" >&2
        exit 1
    fi
    for entry in desktop_request_bytes install_logger_sink set_log_level panic_for_test; do
        if ! grep -q "__swift_bridge__[$]${entry}\$" <<< "$archive_symbols"; then
            echo "error: $arch slice does not export $entry" >&2
            exit 1
        fi
    done
    if grep -q "__swift_bridge__[\$]process_request_bytes\$" <<< "$archive_symbols"; then
        echo "error: $arch slice unexpectedly exports legacy process_request_bytes" >&2
        exit 1
    fi
done

# Same argv as the build above, so cargo resolves the OUT_DIR of the exact
# fingerprint that produced the archive for $BRIDGE_TRIPLE. The generated header,
# modulemap and Swift wrapper are a source-level interface, so one architecture
# is the whole story — building them twice would only give them room to differ.
BRIDGE_OUT_DIR="$(swift_bridge_find_out_dir "$MACOS_DIR" "$CRATE" \
    build --release -p "$CRATE" --target "$BRIDGE_TRIPLE")"

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

# Verify the xcframework really carries every built architecture before staging
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
if [[ "$SLICE_COUNT" != "1" || "$SUPPORTED_PLATFORM" != "macos" || "$SUPPORTED_ARCHS" != "$EXPECTED_ARCHS" ]]; then
    echo "error: expected exactly 1 macos slice carrying $EXPECTED_ARCHS — got $SLICE_COUNT slice(s), first = $SUPPORTED_PLATFORM/$SUPPORTED_ARCHS" >&2
    exit 1
fi

# Stage into the stable path via a sibling temp dir so the previous good
# artefacts are only removed once the new ones are fully copied — an aborted
# copy must not leave `macos/RustEngine/` half-updated.
STAGE_DIR="$DEST_DIR/.staging"
WRAPPERS=("$FRAMEWORK_NAME.swift" "SwiftBridgeCore.swift")
rm -rf "$STAGE_DIR"
mkdir -p "$STAGE_DIR"
cp -R "$OUT_DIR/$FRAMEWORK_NAME.xcframework" "$STAGE_DIR/$FRAMEWORK_NAME.xcframework"
for wrapper in "${WRAPPERS[@]}"; do
    cp "$OUT_DIR/$wrapper" "$STAGE_DIR/$wrapper"
done

rm -rf "$DEST_DIR/$FRAMEWORK_NAME.xcframework"
rm -f "$DEST_DIR/RustTaigiDesktop.swift"
mv "$STAGE_DIR/$FRAMEWORK_NAME.xcframework" "$DEST_DIR/$FRAMEWORK_NAME.xcframework"
for wrapper in "${WRAPPERS[@]}"; do
    mv "$STAGE_DIR/$wrapper" "$DEST_DIR/$wrapper"
done
rmdir "$STAGE_DIR"

echo "ok — macOS $FRAMEWORK_NAME.xcframework ($SUPPORTED_PLATFORM/$SUPPORTED_ARCHS) staged at $DEST_DIR"
