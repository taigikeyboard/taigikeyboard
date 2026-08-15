#!/usr/bin/env bash
# Generate the macOS-side protobuf bindings for the IME.
#
# Output:
#   macos/Sources/TaigiInputMethod/Engine/Generated/*.pb.swift  (SwiftProtobuf)
#
# Separate from `gen-platform-protos.sh` because the macOS IME owns its own
# generated-proto directory and does not need that script's Java block +
# per-file post-process pass. Both run from `make build`, so the macOS tree
# cannot go stale behind an `engine/protos/proto/` change; only its regenerated
# output has to be committed (`.claude/rules/rust-migration-policy.md` §4).
# The proto set is globbed, so adding a `.proto` needs no edit here.
#
# Prerequisites (install once, locally):
#   brew install protobuf swift-protobuf

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PROTO_DIR="$REPO_ROOT/engine/protos/proto"
SWIFT_OUT="$REPO_ROOT/macos/Sources/TaigiInputMethod/Engine/Generated"

if ! command -v protoc >/dev/null 2>&1; then
    echo "error: protoc not found. Install via: brew install protobuf" >&2
    exit 1
fi

if ! command -v protoc-gen-swift >/dev/null 2>&1; then
    echo "error: protoc-gen-swift not found. Install via: brew install swift-protobuf" >&2
    exit 1
fi

# Clear before generating so a deleted `.proto` cannot leave an orphan
# `.pb.swift` that keeps compiling into the SwiftPM target.
rm -rf "$SWIFT_OUT"
mkdir -p "$SWIFT_OUT"

# `Visibility=Public` matches the iOS generation so the same bridge code shape
# ports across platforms.
protoc \
    --proto_path="$PROTO_DIR" \
    --swift_out="$SWIFT_OUT" \
    --swift_opt=Visibility=Public \
    "$PROTO_DIR"/*.proto

echo "generated:"
ls -1 "$SWIFT_OUT"
