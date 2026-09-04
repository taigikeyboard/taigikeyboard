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

# --- protoc version gate ------------------------------------------------------
# The generated Java carries the compiler version in its header and must match
# the runtime pinned in build.gradle.kts (javalite `4.X.Y` is emitted by
# `libprotoc X.Y`). Nothing else enforced that, so a local protoc that differs
# silently rewrote all ~260 .java files on the next `make build` — a downgrade
# that no longer matches the pinned runtime, landing in whatever commit the
# round happened to be making. Observed 2026-09-04: local libprotoc 27.5 vs
# committed gencode 4.36.0.
#
# The pin lives in ONE place (build.gradle.kts); the committed gencode header is
# cross-checked against it so the two cannot drift apart unnoticed either.
GRADLE_FILE="$REPO_ROOT/android/app/build.gradle.kts"
JAVA_PROTO_DIR="$JAVA_OUT/com/siansiansu/taigikeyboard/engine/proto"

runtime_pin="$(sed -n 's/.*protobuf-javalite:\([0-9][0-9.]*\)".*/\1/p' "$GRADLE_FILE" | head -1)"
if [[ -z "$runtime_pin" ]]; then
    echo "error: could not read the protobuf-javalite pin from $GRADLE_FILE" >&2
    exit 1
fi
required_protoc="${runtime_pin#4.}"
actual_protoc="$(protoc --version | awk '{print $2}')"

# The committed gencode must already agree with the pin; if it does not, the
# repo is inconsistent and regenerating would hide it.
sample_java="$JAVA_PROTO_DIR/Start.java"
if [[ -f "$sample_java" ]]; then
    committed_gencode="$(sed -n 's|^// Protobuf Java Version: \(.*\)$|\1|p' "$sample_java" | head -1)"
    if [[ -n "$committed_gencode" && "$committed_gencode" != "$runtime_pin" ]]; then
        echo "error: committed Java gencode is $committed_gencode but $GRADLE_FILE pins protobuf-javalite:$runtime_pin." >&2
        echo "       Regenerate with the matching protoc, or fix the pin — do not paper over it." >&2
        exit 1
    fi
fi

if [[ "$actual_protoc" != "$required_protoc" && "${TAIGI_ALLOW_PROTOC_DRIFT:-0}" != "1" ]]; then
    # Skip rather than abort. The committed output already matches the pin (the
    # cross-check above proved it), so regenerating can only damage it, while
    # the rest of `make build` — xcframework, jniLibs — is unaffected by protoc.
    cat >&2 <<EOF

!! protoc version drift — SKIPPING platform proto regeneration.

   local protoc:  libprotoc $actual_protoc
   required:      libprotoc $required_protoc   (protobuf-javalite:$runtime_pin, $GRADLE_FILE)

   The committed bindings already match the pinned runtime and are left alone.
   Regenerating with a mismatched protoc would rewrite every generated .java
   with gencode the runtime does not match, and that churn is easy to commit by
   accident (.claude/rules/rust-migration-policy.md section 4).

   IF YOU CHANGED A .proto THIS ROUND, the bindings are now STALE — install the
   matching compiler before trusting any platform build:
     brew upgrade protobuf        # then re-check: protoc --version

   To DELIBERATELY move to a different protoc, set TAIGI_ALLOW_PROTOC_DRIFT=1
   and, in the SAME commit, bump protobuf-javalite in $GRADLE_FILE to
   4.<your version> and re-run the Android debug / unit-test / release-R8 gates.

EOF
    exit 0
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
    "$PROTO_DIR/composing.proto" \
    "$PROTO_DIR/lexicon.proto" \
    "$PROTO_DIR/nextword.proto" \
    "$PROTO_DIR/case.proto"

# Java output: --java_out=lite for protobuf-javalite runtime. The
# `option java_package` in the .proto files puts files under
# com/siansiansu/taigikeyboard/engine/proto/ inside $JAVA_OUT.
protoc \
    --proto_path="$PROTO_DIR" \
    --java_out=lite:"$JAVA_OUT" \
    "$PROTO_DIR/envelope.proto" \
    "$PROTO_DIR/phonetics.proto" \
    "$PROTO_DIR/composing.proto" \
    "$PROTO_DIR/lexicon.proto" \
    "$PROTO_DIR/nextword.proto" \
    "$PROTO_DIR/case.proto"

# Post-process generated Java: protoc-gen-java emits trailing whitespace and
# an extra blank line at EOF that fail `git diff --check` and dirty the tree
# on every `make build`. There is no protoc flag to disable this. Strip in
# place so consecutive rebuilds produce a clean diff. Swift output via
# protoc-gen-swift does not have this issue, so only Java is processed.
for f in "$JAVA_PROTO_DIR"/*.java; do
    perl -i -pe 's/[ \t]+$//' "$f"
    perl -i -e 'local $/; $_ = <>; s/\n+\z/\n/; print' "$f"
done

echo "generated:"
ls -1 "$SWIFT_OUT"
ls -1 "$JAVA_OUT/com/siansiansu/taigikeyboard/engine/proto"
