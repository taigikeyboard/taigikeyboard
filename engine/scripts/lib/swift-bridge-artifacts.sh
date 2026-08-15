#!/usr/bin/env bash
# Swift-bridge artifact post-processing shared by the iOS and macOS xcframework
# builds. Both link the same `engine/swift-ffi` crate, so the header/modulemap
# layout and the two generated Swift wrappers must be produced identically for
# both platforms — drift between them is a silent cross-platform bug.
#
# Sourced, never executed. Callers set `set -euo pipefail` and source by
# absolute path:
#
#   SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
#   source "$SCRIPT_DIR/lib/swift-bridge-artifacts.sh"
#
# Every function takes explicit absolute-path arguments; nothing here reads a
# caller global or the working directory. What stays in the platform entry
# scripts is the platform-shaped work: which triples to build, lipo/slice
# assembly, feature flags, staging strategy, and validation.
#
# `framework_name` is a parameter for readability, not for generality: it is
# interpolated into a `grep` pattern and an `awk` variable, so the contract is
# the repo's fixed `RustTaigi` module name, not an arbitrary string.

# Ask cargo which OUT_DIR belongs to the swift-ffi build script for a given
# configuration, by replaying the caller's exact cargo argv with
# `--message-format=json` and reading the `build-script-executed` record.
#
# Pass the SAME argv the build used — cargo keeps one `swift-ffi-<fingerprint>/
# out` per feature set / config, and a differing flag would select (and build) a
# different fingerprint. Replaying identical flags is a cache hit, so the extra
# invocation costs a fraction of a second and compiles nothing.
#
# This replaces a "newest `out` directory by mtime" heuristic, which is wrong
# whenever the current configuration's build script did not re-run: after a
# `--dev` build, a subsequent cached release build leaves the panic-injector
# fingerprint newest, and the heuristic staged ITS headers/wrappers against the
# release archive. Observed on 2026-08-15 — mtime picked
# `swift-ffi-0db2fb3c…` while cargo reported `swift-ffi-887d0482…`. Harmless
# only because both fingerprints happened to emit an identical bridge surface.
#
# Usage: swift_bridge_find_out_dir <engine_dir> <cargo_arg>...
swift_bridge_find_out_dir() {
    local engine_dir="$1"
    shift
    local out_dir
    # `|| return` is load-bearing: the caller assigns this function's output via
    # command substitution, which clears `errexit` inside the subshell, so a
    # cargo or parser failure would otherwise be swallowed.
    out_dir="$(
        cd "$engine_dir" \
        && cargo "$@" --message-format=json --quiet \
        | python3 -c '
import json, sys

out_dirs = []
for line in sys.stdin:
    try:
        message = json.loads(line)
    except json.JSONDecodeError:
        continue
    # Cargo package-id format for a path dependency: `path+file:///…/swift-ffi#0.1.0`.
    # The leading slash keeps a hypothetical sibling `my-swift-ffi` from matching.
    # If a future cargo changes this format, no record matches and the run fails
    # loudly below — it never falls back to guessing.
    if message.get("reason") == "build-script-executed" and "/swift-ffi#" in message.get("package_id", ""):
        out_dirs.append(message["out_dir"])
unique = sorted(set(out_dirs))
if len(unique) != 1:
    sys.exit(f"expected exactly 1 swift-ffi build-script-executed record, got {len(unique)}")
print(unique[0])
'
    )" || return $?
    if [[ -z "$out_dir" ]]; then
        echo "error: cargo reported no swift-ffi OUT_DIR for: cargo $*" >&2
        return 1
    fi
    printf '%s\n' "$out_dir"
}

# Copy the C headers into `headers_dir` and write the modulemap that exposes
# them as the `<framework_name>` Clang module. swift-bridge nests per-bridge
# artefacts under `<OUT>/<framework_name>/`:
#   <OUT>/SwiftBridgeCore.{h,swift}
#   <OUT>/<framework_name>/<framework_name>.{h,swift}
swift_bridge_stage_headers() {
    local bridge_out_dir="$1" headers_dir="$2" framework_name="$3"
    mkdir -p "$headers_dir"
    cp "$bridge_out_dir/SwiftBridgeCore.h" "$headers_dir/"
    cp "$bridge_out_dir/$framework_name/$framework_name.h" "$headers_dir/"
    cat > "$headers_dir/module.modulemap" <<EOF
module $framework_name {
    header "SwiftBridgeCore.h"
    header "$framework_name.h"
    export *
}
EOF
}

# swift-bridge does not emit `import` statements; both wrappers reference C
# symbols defined in the xcframework's modulemap-exposed module. Inject the
# import at the top of each wrapper so any consumer target picks up the C
# surface without a project-level bridging header.
_swift_bridge_inject_import() {
    local file="$1" framework_name="$2"
    if ! grep -q "^import $framework_name" "$file"; then
        local tmp
        tmp=$(mktemp)
        if grep -q '^import Foundation' "$file"; then
            awk -v import_line="import $framework_name" \
                '{ print } /^import Foundation/ && !injected { print import_line; injected=1 }' \
                "$file" > "$tmp"
        else
            { echo "import $framework_name"; echo ""; cat "$file"; } > "$tmp"
        fi
        mv "$tmp" "$file"
    fi
}

# Copy the two generated Swift wrappers into `out_dir` and apply both patches
# swift-bridge's output needs before it compiles cleanly.
swift_bridge_stage_wrappers() {
    local bridge_out_dir="$1" out_dir="$2" framework_name="$3"
    cp "$bridge_out_dir/SwiftBridgeCore.swift" "$out_dir/"
    cp "$bridge_out_dir/$framework_name/$framework_name.swift" "$out_dir/"

    _swift_bridge_inject_import "$out_dir/$framework_name.swift" "$framework_name"
    _swift_bridge_inject_import "$out_dir/SwiftBridgeCore.swift" "$framework_name"

    # Swift 5.9+ warns when a module declares conformance of an imported type to
    # an imported protocol unless the conformance is annotated `@retroactive`.
    # swift-bridge does not yet emit the annotation; patch it in.
    sed -i '' \
        -e 's/^extension RustStr: Identifiable {$/extension RustStr: @retroactive Identifiable {/' \
        -e 's/^extension RustStr: Equatable {$/extension RustStr: @retroactive Equatable {/' \
        "$out_dir/SwiftBridgeCore.swift"
}
