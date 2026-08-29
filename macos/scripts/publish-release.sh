#!/usr/bin/env bash
# Publish an already-built, notarized package: upload it as a GitHub release
# asset, prove it is anonymously downloadable, then point the update manifest at
# it. Split from release-app.sh because the two fail for unrelated reasons and
# rebuilding costs a notarization round trip — a failed upload must not mean
# waiting through that again.
#
# Usage: publish-release.sh [--pkg <path>]
#
#   --pkg <path>  Package to publish (default: the release-named pkg for the
#                 version in App/Info.plist).
#
# Re-running after a failure is safe and is the intended recovery: an existing
# release is added to, never deleted, because the manifest may already be
# pointing at it. Re-publishing a version whose asset is already uploaded is the
# one exception — see the note on --clobber below.

set -euo pipefail

# shellcheck source=lib/bundle-identity.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib/bundle-identity.sh"

# The website repository, not this one. This one is private, so nothing served
# from it — raw file or releases page — is reachable without credentials, and
# the developer's own authenticated browser cannot see that. Release assets are
# stored outside git, so hosting them alongside the site costs it neither the
# 1 GB GitHub Pages size limit nor its bandwidth allowance.
PUBLISH_REPOSITORY="taigikeyboard/taigikeyboard.github.io"
# The one file a release writes over there. The site's macOS download button
# links straight at the package, so its URL carries the version and changes
# every release; it is committed as site data rather than written into the page,
# which keeps this script the only thing that edits it — and keeps the button
# off `/releases/latest`, which resolves repository-wide on a repository that is
# a website rather than this app's release channel.
#
# The update manifest at `appcast/macos.json` is *rendered* from this file by
# the site's own build, not written here. It used to be a second literal file
# this script committed separately, and two commits seconds apart raced: each
# Pages run deploys the tree of its own commit, so on 2026-08-28 the run for the
# earlier commit finished last and served a manifest one release behind for a
# day, while the download button was already current. One published fact, one
# committed file, nothing to race.
SITE_RELEASE_PATH="_data/macos_release.json"
# The domain is the project's own, so the hosting underneath it can change
# without stranding installs that have UpdateChecker.publishedURL baked in.
MANIFEST_URL="https://taigikeyboard.tw/appcast/macos.json"
# A tag names what it versions, and that repository is a website: a bare
# `v3.6.5` there would not say which artifact it belongs to.
TAG="macos-v$SHORT_VERSION"
RELEASE_PAGE_URL="https://github.com/$PUBLISH_REPOSITORY/releases/tag/$TAG"

pkg_path=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --pkg)
            [[ $# -ge 2 ]] || { echo "error: --pkg needs a path" >&2; exit 2; }
            pkg_path="$2"
            shift
            ;;
        *)
            echo "error: unknown argument '$1'" >&2
            echo "usage: publish-release.sh [--pkg <path>]" >&2
            exit 2
            ;;
    esac
    shift
done

# -q must come first: it is what stops curl reading ~/.curlrc, which could
# otherwise switch on the netrc that --netrc-file disables here. Together they
# guarantee these requests carry no credentials, which is the entire point —
# an authenticated check cannot tell a public URL from a private one.
anonymous_curl() {
    curl -q --netrc-file /dev/null --silent --show-error --location "$@"
}

anonymous_status() {
    anonymous_curl --output /dev/null --write-out '%{http_code}' "$@" || true
}

# ---------------------------------------------------------------------------
# Preflight.
# ---------------------------------------------------------------------------

# The manifest only accepts dotted integers — the checker rejects anything with
# a suffix as malformed, and does so silently, so a `3.6.5-beta` here would
# publish a release that every installed copy quietly refuses to read.
[[ "$SHORT_VERSION" =~ ^[0-9]+(\.[0-9]+)*$ ]] ||
    fail "CFBundleShortVersionString '$SHORT_VERSION' is not dotted integers — the update manifest rejects suffixes"

command -v gh > /dev/null || fail "the GitHub CLI (gh) is not installed"
gh auth status > /dev/null 2>&1 || fail "gh is not authenticated — run 'gh auth login'"

# The release build names its output exactly this; anything unpublishable
# (`-dirty`, `-unnotarized`) carries a qualifier and therefore cannot be picked
# up by accident here.
[[ -n "$pkg_path" ]] || pkg_path="$DISTRIBUTION_DIR/$APP_NAME-$SHORT_VERSION.pkg"
[[ -f "$pkg_path" ]] || fail "no package at $pkg_path — run 'make macos-release' first"
PKG_NAME="$(basename "$pkg_path")"
# Absolute from here on: the identity check below reads the package from inside
# a temporary directory, where a relative --pkg would no longer resolve.
pkg_path="$(cd "$(dirname "$pkg_path")" && pwd)/$PKG_NAME"

echo "==> Verifying the package is publishable"
# A package that is not stapled installs only where the notarization ticket can
# be fetched online, and fails closed on a Mac that is offline or behind a
# filter. Checked here rather than trusted from the build, because --pkg can
# point at anything.
xcrun stapler validate "$pkg_path" > /dev/null ||
    fail "$PKG_NAME carries no stapled notarization ticket"
spctl --assess --type install --verbose=2 "$pkg_path" ||
    fail "$PKG_NAME does not pass Gatekeeper assessment"

# Those two prove Apple signed it — not that it is this app at this version. The
# tag and the manifest both take their version from Info.plist, so without this
# `--pkg TaigiKeyboard-3.6.4.pkg` from a 3.6.5 checkout would announce 3.6.4 as
# 3.6.5, and a filename check would not catch it because filenames are typed.
PACKAGE_IDENTITY_DIR="$(mktemp -d)"
trap 'rm -rf "$PACKAGE_IDENTITY_DIR"' EXIT
# The product archive is a xar; its Distribution carries the identifier and
# version productbuild recorded, which is what the Installer itself compares.
(cd "$PACKAGE_IDENTITY_DIR" && xar -xf "$pkg_path" Distribution) 2>/dev/null ||
    fail "$PKG_NAME is not a product archive — no Distribution inside it"
{
    read -r PACKAGE_IDENTIFIER
    read -r PACKAGE_VERSION
} < <(python3 -c '
import sys, xml.etree.ElementTree as ElementTree
root = ElementTree.parse(sys.argv[1]).getroot()
# The referencing pkg-ref inside <choice> carries no version; the one declaring
# the component does.
declaring = [ref for ref in root.iter("pkg-ref") if ref.get("version")]
reference = declaring[0] if len(declaring) == 1 else None
print((reference.get("id") or "") if reference is not None else "")
print((reference.get("version") or "") if reference is not None else "")
' "$PACKAGE_IDENTITY_DIR/Distribution")

[[ "$PACKAGE_IDENTIFIER" == "$BUNDLE_IDENTIFIER" ]] ||
    fail "$PKG_NAME declares '$PACKAGE_IDENTIFIER', not $BUNDLE_IDENTIFIER"
[[ "$PACKAGE_VERSION" == "$BUILD_VERSION" ]] ||
    fail "$PKG_NAME is build $PACKAGE_VERSION, but this checkout is $BUILD_VERSION ($SHORT_VERSION) — it would be announced as the wrong version"

# Release notes come from the changelog when the version has one; the website
# repository's own commit history has nothing to do with this app, so generated
# notes would be noise.
#
# Only its `### macOS` section. That file is shared with iOS and Android, and
# their sections describe work a reader of this page cannot install — the
# mobile What's New reaches them through the stores instead. The heading itself
# is dropped because the release is already titled for macOS.
#
# Written to a file rather than passed as `--notes`: the section is markdown
# whose newlines would not survive being quoted through an argument. This trap
# replaces the one set above and covers both temporaries.
NOTES_FILE="$REPOSITORY_DIR/changelog/desktop-v$SHORT_VERSION.md"
MACOS_NOTES_FILE="$(mktemp)"
trap 'rm -rf "$PACKAGE_IDENTITY_DIR" "$MACOS_NOTES_FILE"' EXIT

if [[ -f "$NOTES_FILE" ]]; then
    # `^### ` cannot match a `#### ` subheading — the fourth character is a
    # hash, not the space the pattern requires — so the section keeps its own
    # subsections and ends at the next sibling.
    awk '/^### macOS$/ { inside = 1; next } inside && /^### / { exit } inside' \
        "$NOTES_FILE" > "$MACOS_NOTES_FILE"
fi

declare -a NOTES_ARGS
# One test for both failures: no changelog for this version, and a changelog
# with no macOS section, are the same situation for this page.
if [[ -s "$MACOS_NOTES_FILE" ]]; then
    NOTES_ARGS=(--notes-file "$MACOS_NOTES_FILE")
else
    echo "  note: no '### macOS' section in changelog/desktop-v$SHORT_VERSION.md — publishing with a minimal note"
    NOTES_ARGS=(--notes "TaigiKeyboard for macOS $SHORT_VERSION")
fi

# ---------------------------------------------------------------------------
# Upload, then prove an anonymous visitor can actually reach it.
# ---------------------------------------------------------------------------

# Adding to an existing release rather than replacing it is what makes a
# re-run after a half-finished publish safe. Deleting and recreating would take
# the download away for as long as the second attempt takes — and leave it gone
# for good if that attempt fails — while the manifest still points at it.
#
# --clobber is narrower than that: re-uploading an asset that already exists
# under the same name removes it first, so re-publishing the *same* version has
# a window where its download 404s. Only a re-run of an already-announced
# version is exposed, and the remedy is the same re-run; a new version writes a
# name nothing points at yet.
if gh release view "$TAG" --repo "$PUBLISH_REPOSITORY" > /dev/null 2>&1; then
    echo "==> Release $TAG exists — uploading the package into it"
    gh release upload "$TAG" "$pkg_path" --repo "$PUBLISH_REPOSITORY" --clobber
else
    echo "==> Creating release $TAG in $PUBLISH_REPOSITORY"
    gh release create "$TAG" \
        --repo "$PUBLISH_REPOSITORY" \
        --title "TaigiKeyboard for macOS $SHORT_VERSION" \
        "${NOTES_ARGS[@]}" \
        "$pkg_path"
fi

ASSET_URL="https://github.com/$PUBLISH_REPOSITORY/releases/download/$TAG/$PKG_NAME"

echo "==> Checking the release is reachable without credentials"
# A newly created release and its asset are not always servable the instant the
# API returns, so a first failure here means "not yet", not "not public".
for attempt in 1 2 3 4 5; do
    page_status="$(anonymous_status "$RELEASE_PAGE_URL")"
    # 206 is a satisfied range request — one byte, rather than tens of
    # megabytes, to learn that the asset is anonymously downloadable. 200 means
    # the server ignored the range and sent the whole package to /dev/null;
    # that is a slow answer to the same question, not a wrong one, so it counts.
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

# `downloadURL` reaches the app as the manifest's `packageURL`, which is what
# lets it fetch the installer itself instead of sending the user to a browser;
# an install that reads it still verifies the package's own Developer ID
# signature, so the URL is a convenience rather than something trusted.
SITE_RELEASE_JSON="$(printf '{\n  "version": "%s",\n  "tag": "%s",\n  "downloadURL": "%s",\n  "releasePageURL": "%s"\n}\n' \
    "$SHORT_VERSION" "$TAG" "$ASSET_URL" "$RELEASE_PAGE_URL")"

# Create or replace one file in the website repository.
commit_site_file() {
    local path="$1" message="$2" content="$3"

    python3 -c 'import json,sys; json.loads(sys.stdin.read())' <<< "$content" ||
        fail "generated $path is not valid JSON: $content"

    # Assigned on its own line: a command substitution inside a `local`
    # declaration reports `local`'s own exit status, which would hide a failure
    # here from `set -e`.
    local encoded_content
    encoded_content="$(printf '%s' "$content" | base64 | tr -d '\n')"

    local api="repos/$PUBLISH_REPOSITORY/contents/$path"
    local -a arguments=(
        -X PUT
        -f "message=$message"
        -f "content=$encoded_content"
    )
    # Updating an existing file requires the blob it replaces; creating one must
    # not send a sha at all. Only a genuine 404 means "creating" — a rate limit
    # or a permission error read as one would turn into a confusing failure from
    # the PUT below instead of the reason it actually stopped.
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
    "chore: macOS release -> $SHORT_VERSION" "$SITE_RELEASE_JSON"

echo "==> Waiting for $MANIFEST_URL to serve $SHORT_VERSION"
# GitHub Pages has to build and the CDN has to expire what it holds. Polling the
# real URL is the only thing that proves the release is actually announced;
# everything before this only proves it was committed. It also proves the site
# rendered the manifest from what was committed, which is the one step of the
# announcement this script no longer performs itself.
#
# Both published fields are checked, not just the version. `packageURL` is what
# lets the app fetch the installer itself, and a manifest missing it still reads
# as a perfectly valid update — the app just sends the user to a browser
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
