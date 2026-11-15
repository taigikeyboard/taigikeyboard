#!/usr/bin/env bash
# The nightly channel: unsigned desktop + ChromeOS builds and the browser
# extension of `main`, overwritten under fixed names at the root of
# https://nightly.taigikeyboard.tw/ every night
# (.github/workflows/desktop-nightly.yml, docs/architecture/nightly-builds.md).
#
# Usage: nightly-release.sh check <source sha>
#        nightly-release.sh publish <artifacts dir> <source sha>
#
#   check    No credentials. Prints `skip` when the live build.json already
#            names <source sha>, `build` otherwise — a 404 (first run), a
#            network error or an unreadable file all build: the publish step's
#            own anonymous read-back is what catches a broken host.
#   publish  R2_ACCESS_KEY_ID / R2_SECRET_ACCESS_KEY / R2_ACCOUNT_ID of the
#            nightly bucket's token. <artifacts dir> holds one directory per
#            workflow artifact, named as the build workflows name them. Renames
#            each installer to its fixed nightly name, mirrors it with a
#            receipt, then writes build.json last, so `check` only ever sees a
#            complete upload.
#
# Never touches the release bucket: the token in the `nightly` environment is
# scoped to the nightly bucket alone, and this script names only that bucket.

set -euo pipefail

fail() {
    echo "error: $*" >&2
    exit 1
}

REPOSITORY_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

# shellcheck source=lib/release-site.sh
source "$REPOSITORY_DIR/tools/release/lib/release-site.sh"
# shellcheck source=lib/desktop-release.sh
source "$REPOSITORY_DIR/tools/release/lib/desktop-release.sh"

# After the sources, which set the release bucket and host unconditionally.
R2_BUCKET="taigikeyboard-nightly"
DOWNLOAD_HOST_URL="https://nightly.taigikeyboard.tw"
# The bucket is the channel's own, so its files sit at the root.
NIGHTLY_URL="$DOWNLOAD_HOST_URL"
# Fixed names overwritten every night: the CDN must revalidate, not serve
# yesterday's bytes.
NIGHTLY_CACHE_CONTROL="no-cache"

# One "<artifact dir>|<file glob>|<fixed name>" row per installer. The globs are
# what each build workflow uploads (macos-build.yml's bundle is packaged by the
# nightly workflow into `macos-pkg-nightly`).
NIGHTLY_ROWS=(
    "macos-pkg-nightly|*.pkg|TaigiKeyboard-nightly.pkg"
    "windows-installer-unsigned|*.exe|TaigiKeyboard-nightly.exe"
    "linux-deb|*.deb|taigikeyboard-nightly_amd64.deb"
    "linux-rpm|*.rpm|taigikeyboard-nightly.x86_64.rpm"
    "linux-arch|*.pkg.tar.zst|taigikeyboard-nightly-x86_64.pkg.tar.zst"
    "chromeos-zip|*.zip|TaigiKeyboard-nightly-chromeos.zip"
    "extension-zip|*.zip|TaigiKeyboard-nightly-extension.zip"
)

require_sha() {
    [[ "$1" =~ ^[0-9a-f]{40}$ ]] || fail "source sha '$1' is not 40 hex characters"
}

check() {
    local source_sha="$1" live
    require_sha "$source_sha"
    live="$(anonymous_curl --fail --max-time 30 "$NIGHTLY_URL/build.json" 2> /dev/null |
        python3 -c 'import json, sys; print(json.load(sys.stdin).get("commit", ""))' 2> /dev/null)" || live=""
    if [[ "$live" == "$source_sha" ]]; then
        echo skip
    else
        echo build
    fi
}

# The single file a glob matches inside one artifact directory.
only_match() {
    local dir="$1" pattern="$2"
    local -a matches=()
    local path
    for path in "$dir"/$pattern; do
        [[ -f "$path" ]] && matches+=("$path")
    done
    [[ ${#matches[@]} -eq 1 ]] ||
        fail "$dir holds ${#matches[@]} files matching $pattern, not exactly one"
    echo "${matches[0]}"
}

publish() {
    local artifacts_dir="$1" source_sha="$2"
    require_sha "$source_sha"
    require_r2
    RELEASE_TEMP_DIR="$(mktemp -d)"
    trap 'rm -rf "$RELEASE_TEMP_DIR"' EXIT

    # Every installer located before anything is uploaded: a missing platform
    # fails the night with the live channel untouched.
    local row dir pattern name source
    local -a staged=()
    for row in "${NIGHTLY_ROWS[@]}"; do
        IFS='|' read -r dir pattern name <<< "$row"
        source="$(only_match "$artifacts_dir/$dir" "$pattern")"
        mv "$source" "$RELEASE_TEMP_DIR/$name"
        staged+=("$name")
    done

    local sha256
    for name in "${staged[@]}"; do
        echo "==> $name"
        sha256="$(release_sha256 "$RELEASE_TEMP_DIR/$name")"
        echo "$sha256  $name" > "$RELEASE_TEMP_DIR/$name.sha256"
        mirror_asset "$RELEASE_TEMP_DIR/$name" "$RELEASE_TEMP_DIR/$name.sha256" \
            "$name" "$sha256" "" "$NIGHTLY_CACHE_CONTROL"
    done

    local manifest="$RELEASE_TEMP_DIR/build.json"
    # Plain printf: the sha is checked hex, the version dotted digits.
    printf '{\n  "commit": "%s",\n  "version": "%s",\n  "built": "%s"\n}\n' \
        "$source_sha" "$SHORT_VERSION" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$manifest"
    echo "==> build.json"
    r2 s3 cp --only-show-errors --cache-control "$NIGHTLY_CACHE_CONTROL" \
        --content-type application/json "$manifest" "s3://$R2_BUCKET/build.json" ||
        fail "cannot upload build.json to R2 bucket $R2_BUCKET"
    local attempt
    for attempt in 1 2 3 4 5; do
        [[ "$(check "$source_sha")" == skip ]] && break
        [[ $attempt -eq 5 ]] && fail "$NIGHTLY_URL/build.json does not name $source_sha after the upload"
        echo "  build.json not served yet — retrying in 5s"
        sleep 5
    done

    echo ""
    echo "✓ nightly ${source_sha:0:7} ($SHORT_VERSION) — $NIGHTLY_URL/"
}

mode="${1:-}"
case "$mode:$#" in
    check:2) check "$2" ;;
    publish:3) publish "$2" "$3" ;;
    *)
        echo "usage: nightly-release.sh check <source sha>" >&2
        echo "       nightly-release.sh publish <artifacts dir> <source sha>" >&2
        exit 2
        ;;
esac
