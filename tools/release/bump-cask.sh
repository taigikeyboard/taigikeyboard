#!/usr/bin/env bash
# Point the Homebrew cask at a published macOS package.
#
# Usage: bump-cask.sh <version> <sha256>
#
# Sets `version` and `sha256` in Casks/taigikeyboard.rb of
# taigikeyboard/homebrew-tap through the contents API, nothing else. The values
# are announce-release.sh's `macos_version` / `macos_sha256`: the sha256 of the
# package it downloaded and checked against the staged receipt and the R2 copy
# the cask's url names, so run this only after a successful announcement.
# `announce-release.yml` runs it with the release App's token; after a local
# `make desktop-announce`, `gh` logged in with write access to the tap does.
#
# Idempotent: a re-run finds the cask already there. Refuses a downgrade and a
# second sha256 for the same version. The PUT carries the blob sha it read, so a
# concurrent change to the file fails it rather than being overwritten.

set -euo pipefail

CASK_PATH=repos/taigikeyboard/homebrew-tap/contents/Casks/taigikeyboard.rb
VERSION_PATTERN='^[0-9]+(\.[0-9]+)+$'
SHA256_PATTERN='^[0-9a-f]{64}$'

fail() { echo "error: $1" >&2; exit 1; }

[[ $# -eq 2 ]] || { echo "usage: bump-cask.sh <version> <sha256>" >&2; exit 2; }
version="$1" sha256="$2"
[[ "$version" =~ $VERSION_PATTERN ]] || fail "version '$version' is not dotted integers"
[[ "$sha256" =~ $SHA256_PATTERN ]] || fail "sha256 '$sha256' is not 64 lowercase hex digits"

scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

gh api "$CASK_PATH" > "$scratch/contents.json"
blob="$(python3 -c 'import json, sys; print(json.load(sys.stdin)["sha"])' < "$scratch/contents.json")"
# To a file: `$(…)` would strip the cask's final newline.
python3 -c 'import base64, json, sys; sys.stdout.buffer.write(base64.b64decode(json.load(sys.stdin)["content"]))' \
    < "$scratch/contents.json" > "$scratch/old.rb"

# Two matching lines would join into one value with a newline, which the
# patterns below reject.
old_version="$(sed -n 's/^  version "\([^"]*\)"$/\1/p' "$scratch/old.rb")"
old_sha256="$(sed -n 's/^  sha256 "\([^"]*\)"$/\1/p' "$scratch/old.rb")"
[[ "$old_version" =~ $VERSION_PATTERN && "$old_sha256" =~ $SHA256_PATTERN ]] ||
    fail "Casks/taigikeyboard.rb no longer has exactly one version and one sha256 line — bump it by hand"

if [[ "$old_version" == "$version" ]]; then
    [[ "$old_sha256" == "$sha256" ]] ||
        fail "the cask already names $version with sha256 $old_sha256, not $sha256"
    echo "cask already at $version"
    exit 0
fi
printf '%s\n' "$old_version" "$version" | sort -VC ||
    fail "the cask names $old_version, newer than $version — not downgrading it"

sed -e "s/^  version \"[^\"]*\"$/  version \"$version\"/" \
    -e "s/^  sha256 \"[^\"]*\"$/  sha256 \"$sha256\"/" "$scratch/old.rb" > "$scratch/new.rb"

gh api "$CASK_PATH" -X PUT \
    -f message="chore: bump taigikeyboard cask $old_version -> $version" \
    -f content="$(base64 < "$scratch/new.rb" | tr -d '\n')" \
    -f sha="$blob" \
    --jq '"bumped the cask to \(.content.path) @ \(.commit.sha[0:7]): \(.commit.html_url)"'
