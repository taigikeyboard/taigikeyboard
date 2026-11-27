#!/usr/bin/env bash
# Generate the macOS-side SwiftProtobuf bindings for the desktop shell
# (`macos/crates/taigi-macos-ffi/proto/desktop_shell.proto`).
#
# Output (SwiftProtobuf):
#   macos/Sources/TaigiInputMethodCore/DesktopCore/Generated/*.pb.swift  desktop shell
#
# Separate from `gen-platform-protos.sh` because the macOS IME owns its own
# generated-proto directory and does not need that script's Java block +
# per-file post-process pass. Both run from `make build`, so the macOS tree
# cannot go stale behind a `.proto` change; only its regenerated output has to
# be committed (`docs/contributing/rust-migration-policy.md` §4).
# The proto set is globbed, so adding a `.proto` needs no edit here.
#
# Prerequisites (install once, locally):
#   mise install

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SHELL_PROTO_DIR="$REPO_ROOT/macos/crates/taigi-macos-ffi/proto"
SHELL_SWIFT_OUT="$REPO_ROOT/macos/Sources/TaigiInputMethodCore/DesktopCore/Generated"

if ! command -v protoc >/dev/null 2>&1; then
    echo "error: protoc not found. Install via: mise install" >&2
    exit 1
fi

if ! command -v protoc-gen-swift >/dev/null 2>&1; then
    echo "error: protoc-gen-swift not found. Install via: mise install" >&2
    exit 1
fi

# Clear before generating so a deleted `.proto` cannot leave an orphan
# `.pb.swift` that keeps compiling into the SwiftPM target.
rm -rf "$SHELL_SWIFT_OUT"
mkdir -p "$SHELL_SWIFT_OUT"

# `Visibility=Public` matches the iOS generation so the same bridge code shape
# ports across platforms.
protoc \
    --proto_path="$SHELL_PROTO_DIR" \
    --swift_out="$SHELL_SWIFT_OUT" \
    --swift_opt=Visibility=Public \
    "$SHELL_PROTO_DIR"/*.proto

echo "generated:"
ls -1 "$SHELL_SWIFT_OUT"
