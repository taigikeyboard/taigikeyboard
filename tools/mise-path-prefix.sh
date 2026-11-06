#!/bin/sh
# Prints the bin directories of the tools mise.toml pins, each followed by a colon,
# for a caller to put in front of PATH. Prints nothing when mise is not installed
# or will not read the file, and leaves out a tool it has not installed, so the
# caller keeps whatever its PATH already resolves. Always exits 0.
#
# Makefiles, hooks and GUI tools run in shells where mise was never activated, so
# without this a bare `swiftformat` is whichever one PATH finds first.
#
# Named tools only. An unscoped `mise bin-paths`, `mise env` or `mise exec` also
# applies the contributor's global tools: a global `rust = "nightly"` makes
# `mise exec` set RUSTUP_TOOLCHAIN, which overrides each workspace's
# rust-toolchain.toml.

cd "$(dirname "$0")/.." || exit 0

# Where mise's installer puts it, which only an interactive shell's config adds.
PATH="$PATH:$HOME/.local/bin"
command -v mise >/dev/null 2>&1 || exit 0

tools=$(awk -F= '/^\[/ { in_tools = ($0 == "[tools]"); next }
                 in_tools && /^[a-z"]/ { gsub(/[" ]/, "", $1); print $1 }' mise.toml)
[ -n "$tools" ] || exit 0

# A changed mise.toml is untrusted until `mise trust`. Say so: the tools PATH finds
# instead are what this script exists to avoid, and nothing else would point at it.
# shellcheck disable=SC2086 # one word per tool
if ! paths=$(mise bin-paths $tools 2>/dev/null); then
  echo "⚠ mise cannot read mise.toml, so its pinned tools are not on PATH. Run: mise trust" >&2
  exit 0
fi

# Never a lone colon: an empty PATH entry means the current directory.
[ -z "$paths" ] || printf '%s\n' "$paths" | tr '\n' ':'
