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

# Locate swift-bridge's OUT_DIR for the build that just ran. Cargo keeps several
# `swift-ffi-<fingerprint>/out` directories per target (one per feature set /
# config), so pick the newest by mtime. Scoped to one target's build tree, so a
# sibling platform's output can never be selected.
swift_bridge_find_out_dir() {
    local target_build_dir="$1"
    local out_dir
    # `|| return` is load-bearing: the caller assigns this function's output via
    # command substitution, which clears `errexit` inside the subshell. Without
    # it a failing `find`/`stat` (permissions, filesystem race) that still
    # printed a path would be masked by the trailing `printf`'s exit 0.
    out_dir="$(
        find "$target_build_dir" \
            -type d -name 'out' -path '*/swift-ffi-*' \
            -exec stat -f '%m %N' {} + \
        | sort -rn | head -n 1 | cut -d' ' -f2-
    )" || return $?
    if [[ -z "$out_dir" ]]; then
        # Parity correction (cross-platform-alignment.md §1b): iOS previously
        # printed no search path, macOS printed a repo-relative one. Unified on
        # the absolute path — the more diagnosable of the two. Exit status is
        # unchanged on both platforms.
        echo "error: swift-bridge OUT_DIR not found under $target_build_dir" >&2
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
