#!/usr/bin/env bash
# Publish a built Windows installer (roadmap W8): a GitHub release on the
# website repository tagged windows-v<version>, verified reachable WITHOUT
# credentials, then ONE committed file, `_data/windows_release.json` (the
# site's download button; the site renders `appcast/windows.json` — what every
# installed copy polls, windows/updates/README.md — from it), and a wait until
# the live manifest serves the new version, its URL and its SHA-256. Mirror of
# macos/scripts/publish-release.sh; the order is the point — a manifest
# published before its download is reachable points every checker at a 404.
#
#   bash windows/scripts/publish-release.sh [--installer <path>] [--allow-unsigned]
#
# An installer without a trusted Authenticode signature is refused unless
# --allow-unsigned says so out loud. That flag is how this project ships today
# (docs/architecture/windows-release.md § Signing status); release-app.sh
# passes it down when it was itself run with --skip-sign, so a direct
# invocation of this script cannot publish unsigned by accident.

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
allow_unsigned=false
while [[ $# -gt 0 ]]; do
    case "$1" in
        --installer)
            [[ $# -ge 2 ]] || { echo "error: --installer needs a path" >&2; exit 2; }
            installer_path="$2"
            shift
            ;;
        --allow-unsigned) allow_unsigned=true ;;
        *)
            echo "error: unknown argument '$1'" >&2
            echo "usage: publish-release.sh [--installer <path>] [--allow-unsigned]" >&2
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
[[ "$allow_unsigned" == false || -z "${WINDOWS_SIGNING_THUMBPRINT:-}" ]] ||
    fail "--allow-unsigned and WINDOWS_SIGNING_THUMBPRINT contradict: a certificate is named, so sign the installer instead of publishing it unsigned"
[[ "$INSTALLER_NAME" == "$APP_NAME-$SHORT_VERSION.exe" ]] ||
    fail "$INSTALLER_NAME is not the release name for $SHORT_VERSION (a -dirty build is not publishable)"
declare -a REQUIRED_TOOLS=(curl base64 sha256sum python3 powershell.exe)
[[ "$allow_unsigned" == true ]] || REQUIRED_TOOLS+=(signtool)
for tool in "${REQUIRED_TOOLS[@]}"; do
    command -v "$tool" > /dev/null || fail "$tool is not on PATH"
done
if [[ "$allow_unsigned" == true ]]; then
    echo "  ⚠ --allow-unsigned: the Authenticode gate is skipped for this release"
else
    run_windows_tool signtool verify /pa /q "$(windows_path "$installer_path")" > /dev/null ||
        fail "$INSTALLER_NAME carries no Authenticode signature Windows trusts — the in-app updater would refuse it (pass --allow-unsigned to publish it anyway)"
fi
# What every installed copy checks the download against
# (taigi-windows-update::verify::admit): the published digest (read back from
# the asset below), this product, this version, and — when the release
# certificate is named — signed by exactly it. A renamed file signed by anyone
# else never reaches the manifest.
require_version_info "$installer_path"
if [[ -n "${WINDOWS_SIGNING_THUMBPRINT:-}" ]]; then
    signer="$(signer_thumbprint_of "$installer_path")"
    [[ "${signer^^}" == "${WINDOWS_SIGNING_THUMBPRINT^^}" ]] ||
        fail "$INSTALLER_NAME is signed by '${signer:-nobody}', not the release certificate $WINDOWS_SIGNING_THUMBPRINT"
elif [[ "$allow_unsigned" == false ]]; then
    echo "  note: WINDOWS_SIGNING_THUMBPRINT is not set — the signer is trusted but not pinned to the release certificate"
fi

NOTES_FILE="$REPOSITORY_DIR/changelog/desktop-v$SHORT_VERSION.md"
WINDOWS_NOTES_FILE="$(mktemp)"
PUBLISHED_INSTALLER="$(mktemp)"
trap 'rm -f "$WINDOWS_NOTES_FILE" "$PUBLISHED_INSTALLER"' EXIT
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

# Anonymously, because that is how every user and every installed copy will
# reach it — and WHOLE, because the digest the manifest is about to publish
# has to be the one this URL actually serves. Those are different facts the
# moment an upload truncates, a `--clobber` replaces the file, or the wrong
# build was handed to `--installer`, and this is the one step that catches
# them before anybody's settings window downloads it. GitHub can take a
# moment to make a fresh asset reachable, so it is the retried step.
echo "==> Reading the release back without credentials"
LOCAL_SHA256="$(sha256_of "$installer_path")"
for attempt in 1 2 3 4 5; do
    page_status="$(anonymous_status "$RELEASE_PAGE_URL")"
    if [[ "$page_status" == "200" ]] &&
        anonymous_curl --output "$PUBLISHED_INSTALLER" "$ASSET_URL"; then
        PUBLISHED_SHA256="$(sha256_of "$PUBLISHED_INSTALLER")"
        [[ "$LOCAL_SHA256" == "$PUBLISHED_SHA256" ]] ||
            fail "$ASSET_URL serves $PUBLISHED_SHA256, but $INSTALLER_NAME here is $LOCAL_SHA256 — the upload did not land whole"
        break
    fi
    [[ $attempt -eq 5 ]] &&
        fail "release not anonymously reachable (page $page_status) — is $PUBLISH_REPOSITORY public?"
    echo "  page $page_status, asset unreadable — retrying in 5s"
    sleep 5
done
echo "  page 200, sha256 $PUBLISHED_SHA256"

# ---------------------------------------------------------------------------
# Only now announce it. The file below names a download that has just been
# proven reachable, and both readers of it — every visitor in the download
# button's case, every installed copy in the manifest's — would otherwise be
# pointed at a 404.
# ---------------------------------------------------------------------------

# `downloadURL` reaches the input method as the manifest's `packageURL` and
# `sha256` as its `packageSHA256`: together they are what lets an installed
# copy fetch and admit the installer itself instead of sending the user to a
# browser. BOTH are required for that — a manifest naming a package with no
# digest offers the download page, because an unsigned copy has nothing else
# to hold the download against (`taigi-windows-update::verify::admit`). A
# signed copy checks its Authenticode signature on top.
SITE_RELEASE_JSON="$(printf '{\n  "version": "%s",\n  "tag": "%s",\n  "downloadURL": "%s",\n  "sha256": "%s",\n  "releasePageURL": "%s"\n}\n' \
    "$SHORT_VERSION" "$TAG" "$ASSET_URL" "$PUBLISHED_SHA256" "$RELEASE_PAGE_URL")"

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
# All three published fields are checked, not just the version. `packageURL`
# and `packageSHA256` are what let an installed copy fetch and admit the
# installer itself, and a manifest missing either still reads as a perfectly
# valid update — the user is sent to a browser instead. So a render that
# dropped one would satisfy a version-only poll and quietly cost every install
# the in-app download; there is no later signal that it happened. The digest
# is compared against what the published asset actually served, above.
for attempt in $(seq 1 30); do
    live_manifest="$(anonymous_curl --header 'Cache-Control: no-cache' "$MANIFEST_URL" 2>/dev/null |
        python3 -c 'import json,sys
try:
    manifest = json.load(sys.stdin)
    print(" ".join((manifest.get(field) or "-") for field in ("version", "packageURL", "packageSHA256")))
except Exception:
    print("-")' || true)"
    [[ "$live_manifest" == "$SHORT_VERSION $ASSET_URL $PUBLISHED_SHA256" ]] && break
    [[ $attempt -eq 30 ]] &&
        fail "manifest still serving '$live_manifest' after 5 minutes, wanted '$SHORT_VERSION $ASSET_URL $PUBLISHED_SHA256' — check the Pages deployment"
    sleep 10
done

echo ""
echo "✓ published $SHORT_VERSION"
echo "  release   $RELEASE_PAGE_URL"
echo "  download  $ASSET_URL"
echo "  manifest  $MANIFEST_URL"
echo "  website   https://taigikeyboard.tw/#download"
if [[ "$allow_unsigned" == true ]]; then
    echo ""
    echo "  ⚠ published UNSIGNED. SmartScreen typically warns (其他資訊 → 仍要執行) and Win11 Smart App"
    echo "    Control can refuse it. Installed copies DO still update in-app, on the published SHA-256."
fi
