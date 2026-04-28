#!/usr/bin/env bash
# Generate platform-side protobuf code for D9.2.
#
# Outputs:
#   ios/Sources/TaigiKeyboard/Engine/Generated/*.pb.swift  (SwiftProtobuf)
#   android/app/src/main/java/com/siansiansu/taigikeyboard/engine/proto/*.java
#     (protobuf-javalite; package derived from `option java_package` in
#     `engine/protos/proto/*.proto`)
#
# Prerequisites (install once, locally):
#   brew install protobuf swift-protobuf

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PROTO_DIR="$REPO_ROOT/engine/protos/proto"
SWIFT_OUT="$REPO_ROOT/ios/Sources/TaigiKeyboard/Engine/Generated"
JAVA_OUT="$REPO_ROOT/android/app/src/main/java"

if ! command -v protoc >/dev/null 2>&1; then
    echo "error: protoc not found. Install via: brew install protobuf" >&2
    exit 1
fi

if ! command -v protoc-gen-swift >/dev/null 2>&1; then
    echo "error: protoc-gen-swift not found. Install via: brew install swift-protobuf" >&2
    exit 1
fi

mkdir -p "$SWIFT_OUT" "$JAVA_OUT"

# Swift output: --swift_opt=Visibility=Public so the bridge module can import
# the generated types.
protoc \
    --proto_path="$PROTO_DIR" \
    --swift_out="$SWIFT_OUT" \
    --swift_opt=Visibility=Public \
    "$PROTO_DIR/envelope.proto" \
    "$PROTO_DIR/phonetics.proto" \
    "$PROTO_DIR/lexicon.proto"

# Java output: --java_out=lite for protobuf-javalite runtime. The
# `option java_package` in the .proto files puts files under
# com/siansiansu/taigikeyboard/engine/proto/ inside $JAVA_OUT.
protoc \
    --proto_path="$PROTO_DIR" \
    --java_out=lite:"$JAVA_OUT" \
    "$PROTO_DIR/envelope.proto" \
    "$PROTO_DIR/phonetics.proto" \
    "$PROTO_DIR/lexicon.proto"

echo "generated:"
ls -1 "$SWIFT_OUT"
ls -1 "$JAVA_OUT/com/siansiansu/taigikeyboard/engine/proto"
