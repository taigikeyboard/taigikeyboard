#!/usr/bin/env bash
# Build `librust_taigi.so` for arm64-v8a and place it at the contractual
# jniLibs path that `System.loadLibrary("rust_taigi")` resolves.
#
# Usage:
#   build-android-libs.sh          # release (no panic-injector; symbol must be absent)
#   build-android-libs.sh --dev    # dev (--features panic-injector; symbol must be present)
#
# Prerequisites (install once, locally):
#   cargo install cargo-ndk --locked
#   Android NDK r25+ on PATH (rustup-bundled or sdkmanager)
#
# Output (per plan v4 R3-H1):
#   android/app/src/main/jniLibs/arm64-v8a/librust_taigi.so

set -euo pipefail

DEV=0
if [[ "${1:-}" == "--dev" ]]; then
    DEV=1
fi

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ENGINE_DIR="$REPO_ROOT/engine"
JNI_LIBS_ROOT="$REPO_ROOT/android/app/src/main/jniLibs"
EXPECTED="$JNI_LIBS_ROOT/arm64-v8a/librust_taigi.so"

# Guard against ABI drift: if android/app/build.gradle.kts still declares
# armeabi-v7a or x86_64 in abiFilters, this script would publish only
# arm64-v8a and the app would crash with `UnsatisfiedLinkError` on those
# devices. Either narrow abiFilters to arm64-v8a (per the D9.2 user-action
# checklist) or extend this script to build the full ABI set.
GRADLE_ABI="$REPO_ROOT/android/app/build.gradle.kts"
if grep -E 'abiFilters.*"(armeabi-v7a|x86_64)"' "$GRADLE_ABI" >/dev/null 2>&1; then
    echo "error: $GRADLE_ABI still declares non-arm64 ABIs in abiFilters." >&2
    echo "       Either narrow to listOf(\"arm64-v8a\") or build all ABIs." >&2
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

# cargo-ndk auto-creates `<output>/<abi>/lib<name>.so` under the -o root.
cargo ndk -t arm64-v8a -o "$JNI_LIBS_ROOT" "${CARGO_FLAGS[@]}"

if [[ ! -f "$EXPECTED" ]]; then
    echo "error: expected $EXPECTED to exist" >&2
    exit 1
fi

# Symbol smoke check: panic-injector feature must match symbol presence.
SYMBOL_PRESENT=0
if nm -D "$EXPECTED" 2>/dev/null | grep -q 'panicForTest'; then
    SYMBOL_PRESENT=1
fi

if [[ $DEV -eq 1 && $SYMBOL_PRESENT -eq 0 ]]; then
    echo "error: panicForTest symbol absent from dev .so — feature gate misconfigured" >&2
    exit 1
fi
if [[ $DEV -eq 0 && $SYMBOL_PRESENT -eq 1 ]]; then
    echo "error: panicForTest symbol present in release .so — panic-injector feature leaked" >&2
    exit 1
fi

if [[ $DEV -eq 1 ]]; then
    echo "ok — DEV $EXPECTED ($(du -h "$EXPECTED" | cut -f1))"
else
    echo "ok — RELEASE $EXPECTED ($(du -h "$EXPECTED" | cut -f1))"
fi
