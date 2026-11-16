#!/usr/bin/env bash
# The Rust version the repository pins (root rust-toolchain.toml).
#
# Usage: rust-toolchain.sh          print the root file's `channel`
#        rust-toolchain.sh check    fail unless every workspace's
#                                   rust-toolchain.toml names that channel, and
#                                   the rustc this shell resolves is that version
#
# rustup reads only the nearest rust-toolchain.toml, so each workspace repeats
# the root's channel beside its own targets; `check` (run by `make lint`) keeps
# the copies equal. CI reads the version through `.github/actions/setup-rust`.

set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

channel_of() {
    sed -n 's/^channel *= *"\(.*\)"[[:space:]]*$/\1/p' "$1"
}

root="$(channel_of rust-toolchain.toml)"
[[ "$root" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || {
    echo "error: rust-toolchain.toml channel '$root' is not an x.y.z version" >&2
    exit 1
}

case "${1:-}" in
    "") echo "$root" ;;
    check)
        status=0
        for file in */rust-toolchain.toml; do
            channel="$(channel_of "$file")"
            if [[ "$channel" != "$root" ]]; then
                echo "error: $file channel '$channel' is not rust-toolchain.toml's '$root'" >&2
                status=1
            fi
        done
        # RUSTUP_TOOLCHAIN or a `rustup override` beats every file, and would
        # lint with another clippy than CI's.
        resolved="$(rustc --version 2>/dev/null | awk '{ print $2 }')" || resolved=""
        if [[ "$resolved" != "$root" ]]; then
            echo "error: this shell resolves rustc '${resolved:-none}', not the pinned $root" \
                "(unset RUSTUP_TOOLCHAIN, or \`rustup override unset\`)" >&2
            status=1
        fi
        exit "$status"
        ;;
    *)
        echo "usage: rust-toolchain.sh [check]" >&2
        exit 2
        ;;
esac
