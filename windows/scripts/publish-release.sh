#!/usr/bin/env bash
# Publish a built Windows installer (roadmap W8): a GitHub release on the
# website repository tagged windows-v<version>, verified reachable WITHOUT
# credentials, then ONE committed file, `_data/windows_release.json` (the
# site's download button; the site renders `appcast/windows.json` — what every
# installed copy polls, windows/updates/README.md — from it), and a wait until
# the live manifest serves the new version. Mirror of
# macos/scripts/publish-release.sh; the order is the point — a manifest
# published before its download is reachable points every checker at a 404.
#
#   bash windows/scripts/publish-release.sh [--installer <path>]

set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib/identity.sh"

PUBLISH_REPOSITORY="taigikeyboard/taigikeyboard.github.io"
# The one file a release writes over there. The site's Windows download button
# links straight at the installer, so its URL carries the version and changes
# every release; it is committed as site data rather than written into the
# page, which keeps this script the only thing that edits it.
#
# The update manifest at `appcast/windows.json` is *rendered* from this file
# by the site's own build, not written here. The macOS flow learned this the
# hard way (macos/updates/README.md § One published fact, one committed file):
# two files meant two commits seconds apart, each Pages run deploys the tree of
# its own commit, and the run for the earlier commit finishing last served a
# manifest one release behind for a day. One published fact, one committed
# file, nothing to race.
SITE_RELEASE_PATH="_data/windows_release.json"
# The domain is the project's own, so the hosting underneath it can change
# without stranding installs that have manifest::PUBLISHED_URL baked in.
MANIFEST_URL="https://taigikeyboard.tw/appcast/windows.json"
# A tag names what it versions, and that repository is a website: a bare
# `v3.6.5` there would not say which artifact it belongs to.
TAG="windows-v$SHORT_VERSION"
RELEASE_PAGE_URL="https://github.com/$PUBLISH_REPOSITORY/releases/tag/$TAG"

installer_path=""
while [[ $# -gt 0 ]]; do
    case "$1" in
        --installer)
            [[ $# -ge 2 ]] || { echo "error: --installer needs a path" >&2; exit 2; }
            installer_path="$2"
            shift
            ;;
        *)
            echo "error: unknown argument '$1'" >&2
            echo "usage: publish-release.sh [--installer <path>]" >&2
            exit 2
            ;;
    esac
    shift
done

anonymous_curl() {
    curl -q --netrc-file /dev/null --silent --show-error --location "$@"
}
anonymous_status() {
    anonymous_curl --output /dev/null --write-out '%{http_code}' "$@" || true
}

[[ "$SHORT_VERSION" =~ ^[0-9]+(\.[0-9]+)*$ ]] ||
    fail "version '$SHORT_VERSION' is not dotted integers — the update manifest rejects suffixes"
command -v gh > /dev/null || fail "the GitHub CLI (gh) is not installed"
gh auth status > /dev/null 2>&1 || fail "gh is not authenticated — run 'gh auth login'"
[[ -n "$installer_path" ]] || installer_path="$DISTRIBUTION_DIR/$APP_NAME-$SHORT_VERSION.exe"
[[ -f "$installer_path" ]] || fail "no installer at $installer_path — run 'make windows-release' first"
INSTALLER_NAME="$(basename "$installer_path")"
installer_path="$(cd "$(dirname "$installer_path")" && pwd)/$INSTALLER_NAME"

echo "==> Verifying the installer is publishable"
[[ "$INSTALLER_NAME" == "$APP_NAME-$SHORT_VERSION.exe" ]] ||
    fail "$INSTALLER_NAME is not the release name for $SHORT_VERSION (a -dirty / -throwaway build is not publishable)"
for tool in curl base64 python3 powershell.exe signtool; do
    command -v "$tool" > /dev/null || fail "$tool is not on PATH"
done
run_windows_tool signtool verify /pa /q "$(windows_path "$installer_path")" > /dev/null ||
    fail "$INSTALLER_NAME carries no Authenticode signature Windows trusts — the in-app updater would refuse it"
# What every installed copy checks the download against
# (taigi-windows-update::verify): this product, this version, and — when the
# release certificate is named — signed by exactly it. A renamed file signed
# by anyone else never reaches the manifest.
require_version_info "$installer_path"
if [[ -n "${WINDOWS_SIGNING_THUMBPRINT:-}" ]]; then
    signer="$(signer_thumbprint_of "$installer_path")"
    [[ "${signer^^}" == "${WINDOWS_SIGNING_THUMBPRINT^^}" ]] ||
        fail "$INSTALLER_NAME is signed by '${signer:-nobody}', not the release certificate $WINDOWS_SIGNING_THUMBPRINT"
else
    echo "  note: WINDOWS_SIGNING_THUMBPRINT is not set — the signer is trusted but not pinned to the release certificate"
fi

NOTES_FILE="$REPOSITORY_DIR/changelog/desktop-v$SHORT_VERSION.md"
WINDOWS_NOTES_FILE="$(mktemp)"
trap 'rm -f "$WINDOWS_NOTES_FILE"' EXIT
if [[ -f "$NOTES_FILE" ]]; then
    awk '/^### Windows$/ { inside = 1; next } inside && /^### / { exit } inside' \
        "$NOTES_FILE" > "$WINDOWS_NOTES_FILE"
fi
declare -a NOTES_ARGS
if [[ -s "$WINDOWS_NOTES_FILE" ]]; then
    NOTES_ARGS=(--notes-file "$WINDOWS_NOTES_FILE")
else
    echo "  note: no '### Windows' section in changelog/desktop-v$SHORT_VERSION.md — publishing with a minimal note"
    NOTES_ARGS=(--notes "TaigiKeyboard for Windows $SHORT_VERSION")
fi

if gh release view "$TAG" --repo "$PUBLISH_REPOSITORY" > /dev/null 2>&1; then
    echo "==> Release $TAG exists — uploading the installer into it"
    gh release upload "$TAG" "$installer_path" --repo "$PUBLISH_REPOSITORY" --clobber
else
    echo "==> Creating release $TAG in $PUBLISH_REPOSITORY"
    gh release create "$TAG" \
        --repo "$PUBLISH_REPOSITORY" \
        --title "TaigiKeyboard for Windows $SHORT_VERSION" \
        "${NOTES_ARGS[@]}" \
        "$installer_path"
fi
ASSET_URL="https://github.com/$PUBLISH_REPOSITORY/releases/download/$TAG/$INSTALLER_NAME"

echo "==> Checking the release is reachable without credentials"
for attempt in 1 2 3 4 5; do
    page_status="$(anonymous_status "$RELEASE_PAGE_URL")"
    asset_status="$(anonymous_status --range 0-0 "$ASSET_URL")"
    [[ "$page_status" == "200" && ("$asset_status" == "206" || "$asset_status" == "200") ]] && break
    [[ $attempt -eq 5 ]] &&
        fail "release not anonymously reachable (page $page_status, asset $asset_status) — is $PUBLISH_REPOSITORY public?"
    echo "  page $page_status, asset $asset_status — retrying in 5s"
    sleep 5
done
echo "  page 200, asset $asset_status"

# ---------------------------------------------------------------------------
# Only now announce it. The file below names a download that has just been
# proven reachable, and both readers of it — every visitor in the download
# button's case, every installed copy in the manifest's — would otherwise be
# pointed at a 404.
# ---------------------------------------------------------------------------

# `downloadURL` reaches the input method as the manifest's `packageURL`, which
# is what lets it fetch the installer itself instead of sending the user to a
# browser; an install that reads it still verifies the installer's own
# Authenticode signature against the pinned certificate, so the URL is a
# convenience rather than something trusted.
SITE_RELEASE_JSON="$(printf '{\n  "version": "%s",\n  "tag": "%s",\n  "downloadURL": "%s",\n  "releasePageURL": "%s"\n}\n' \
    "$SHORT_VERSION" "$TAG" "$ASSET_URL" "$RELEASE_PAGE_URL")"

# Create or replace one file in the website repository.
commit_site_file() {
    local path="$1" message="$2" content="$3"
    python3 -c 'import json,sys; json.loads(sys.stdin.read())' <<< "$content" ||
        fail "generated $path is not valid JSON: $content"
    local encoded_content
    encoded_content="$(printf '%s' "$content" | base64 | tr -d '\n')"
    local api="repos/$PUBLISH_REPOSITORY/contents/$path"
    local -a arguments=(
        -X PUT
        -f "message=$message"
        -f "content=$encoded_content"
    )
    local read_result
    if read_result="$(gh api "$api" --jq .sha 2>&1)"; then
        arguments+=(-f "sha=$read_result")
    elif [[ "$read_result" != *"HTTP 404"* ]]; then
        fail "cannot read $path in $PUBLISH_REPOSITORY: $read_result"
    fi
    gh api "$api" "${arguments[@]}" --jq '.commit.html_url'
}

echo "==> Publishing the release data"
commit_site_file "$SITE_RELEASE_PATH" \
    "chore: Windows release -> $SHORT_VERSION" "$SITE_RELEASE_JSON"

echo "==> Waiting for $MANIFEST_URL to serve $SHORT_VERSION"
# GitHub Pages has to build and the CDN has to expire what it holds. Polling the
# real URL is the only thing that proves the release is actually announced;
# everything before this only proves it was committed. It also proves the site
# rendered the manifest from what was committed, which is the one step of the
# announcement this script does not perform itself.
#
# Both published fields are checked, not just the version. `packageURL` is what
# lets the input method fetch the installer itself, and a manifest missing it
# still reads as a perfectly valid update — the user is sent to a browser
# instead. So a render that dropped it would satisfy a version-only poll and
# quietly cost every install the in-app download; there is no later signal that
# it happened.
for attempt in $(seq 1 30); do
    live_manifest="$(anonymous_curl --header 'Cache-Control: no-cache' "$MANIFEST_URL" 2>/dev/null |
        python3 -c 'import json,sys
try:
    manifest = json.load(sys.stdin)
    print((manifest.get("version") or "") + " " + (manifest.get("packageURL") or ""))
except Exception:
    print(" ")' || true)"
    [[ "$live_manifest" == "$SHORT_VERSION $ASSET_URL" ]] && break
    [[ $attempt -eq 30 ]] &&
        fail "manifest still serving '${live_manifest% *}' with package '${live_manifest#* }' after 5 minutes, wanted '$SHORT_VERSION' and '$ASSET_URL' — check the Pages deployment"
    sleep 10
done

echo ""
echo "✓ published $SHORT_VERSION"
echo "  release   $RELEASE_PAGE_URL"
echo "  download  $ASSET_URL"
echo "  manifest  $MANIFEST_URL"
echo "  website   https://taigikeyboard.tw/#download"
