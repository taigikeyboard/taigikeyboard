#!/usr/bin/env bash
# Build `librust_taigi.so` for arm64-v8a + armeabi-v7a and place each at the
# contractual jniLibs path that `System.loadLibrary("rust_taigi")` resolves.
#
# armeabi-v7a was added back in v3.5.7 to recover the 3,806 32-bit ARM devices
# Google Play Console flagged as losing support after the D9.2 arm64-only cut.
# Play 64-bit policy is still met (arm64-v8a present); shipping 32-bit
# alongside is allowed and per-device delivery via App Bundle keeps end-user
# APK size unchanged.
#
# Usage:
#   build-android-libs.sh          # release (no panic-injector; symbol must be absent)
#   build-android-libs.sh --dev    # dev (--features panic-injector; symbol must be present)
#
# Prerequisites (install once, locally):
#   cargo install cargo-ndk --locked
#   rustup target add aarch64-linux-android armv7-linux-androideabi
#   Android NDK r25+ on PATH (rustup-bundled or sdkmanager)
#
# Output:
#   android/app/src/main/jniLibs/arm64-v8a/librust_taigi.so
#   android/app/src/main/jniLibs/armeabi-v7a/librust_taigi.so

set -euo pipefail

DEV=0
if [[ "${1:-}" == "--dev" ]]; then
    DEV=1
fi

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ENGINE_DIR="$REPO_ROOT/engine"
JNI_LIBS_ROOT="$REPO_ROOT/android/app/src/main/jniLibs"
ABIS=(arm64-v8a armeabi-v7a)

# Guard against ABI drift: x86 / x86_64 are deliberately NOT built (Chromebook
# + emulator demand is negligible for this app). If gradle ever adds them to
# abiFilters, the app would crash with UnsatisfiedLinkError on x86 devices.
GRADLE_ABI="$REPO_ROOT/android/app/build.gradle.kts"
if grep -E 'abiFilters.*"(x86|x86_64)"' "$GRADLE_ABI" >/dev/null 2>&1; then
    echo "error: $GRADLE_ABI declares x86 / x86_64 in abiFilters but this script does not build them." >&2
    echo "       Either drop them from abiFilters or extend the ABIS list in this script." >&2
    exit 1
fi

if ! command -v cargo-ndk >/dev/null 2>&1; then
    echo "error: cargo-ndk not found. Install via: cargo install cargo-ndk --locked" >&2
    exit 1
fi

cd "$ENGINE_DIR"
mkdir -p "$JNI_LIBS_ROOT"

CARGO_FLAGS=(build --release -p android-jni)
if [[ $DEV -eq 1 ]]; then
    CARGO_FLAGS+=(--features panic-injector)
fi

# cargo-ndk accepts -t multiple times and writes per-ABI subdirs under -o.
NDK_TARGET_FLAGS=()
for abi in "${ABIS[@]}"; do
    NDK_TARGET_FLAGS+=(-t "$abi")
done
cargo ndk "${NDK_TARGET_FLAGS[@]}" -o "$JNI_LIBS_ROOT" "${CARGO_FLAGS[@]}"

# Verify each ABI produced a .so + symbol gate matches feature flag.
for abi in "${ABIS[@]}"; do
    so_path="$JNI_LIBS_ROOT/$abi/librust_taigi.so"
    if [[ ! -f "$so_path" ]]; then
        echo "error: expected $so_path to exist" >&2
        exit 1
    fi

    symbol_present=0
    if nm -D "$so_path" 2>/dev/null | grep -q 'panicForTest'; then
        symbol_present=1
    fi
    if [[ $DEV -eq 1 && $symbol_present -eq 0 ]]; then
        echo "error: panicForTest symbol absent from $abi dev .so — feature gate misconfigured" >&2
        exit 1
    fi
    if [[ $DEV -eq 0 && $symbol_present -eq 1 ]]; then
        echo "error: panicForTest symbol present in $abi release .so — panic-injector feature leaked" >&2
        exit 1
    fi
done

mode_label=$([[ $DEV -eq 1 ]] && echo "DEV" || echo "RELEASE")
for abi in "${ABIS[@]}"; do
    so_path="$JNI_LIBS_ROOT/$abi/librust_taigi.so"
    echo "ok — $mode_label $so_path ($(du -h "$so_path" | cut -f1))"
done
