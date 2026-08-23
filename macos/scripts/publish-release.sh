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
# release is added to, never deleted. Nothing here ever takes a published
# download away, because the manifest may already be pointing at it.

set -euo pipefail

# shellcheck source=lib/bundle-identity.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib/bundle-identity.sh"

# The website repository, not this one. This one is private, so nothing served
# from it — raw file or releases page — is reachable without credentials, and
# the developer's own authenticated browser cannot see that. Release assets are
# stored outside git, so hosting them alongside the site costs it neither the
# 1 GB GitHub Pages size limit nor its bandwidth allowance.
PUBLISH_REPOSITORY="taigikeyboard/taigikeyboard.github.io"
MANIFEST_PATH="appcast/macos.json"
# The domain is the project's own, so the hosting underneath it can change
# without stranding installs that have UpdateChecker.publishedURL baked in.
MANIFEST_URL="https://taigikeyboard.tw/$MANIFEST_PATH"
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
NOTES_FILE="$REPOSITORY_DIR/changelog/v$SHORT_VERSION.md"
declare -a NOTES_ARGS
if [[ -f "$NOTES_FILE" ]]; then
    NOTES_ARGS=(--notes-file "$NOTES_FILE")
else
    echo "  note: no changelog/v$SHORT_VERSION.md — publishing with a minimal note"
    NOTES_ARGS=(--notes "TaigiKeyboard for macOS $SHORT_VERSION")
fi

# ---------------------------------------------------------------------------
# Upload, then prove an anonymous visitor can actually reach it.
# ---------------------------------------------------------------------------

# Adding to an existing release rather than replacing it is what makes a
# re-run after a half-finished publish safe. Deleting and recreating would take
# the download away for as long as the second attempt takes — and leave it gone
# for good if that attempt fails — while the manifest still points at it.
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
# Only now announce it. The manifest is what every installed copy polls, so
# publishing it before the download exists points all of them at a 404.
# ---------------------------------------------------------------------------

echo "==> Publishing the update manifest"
MANIFEST_JSON="$(printf '{\n  "version": "%s",\n  "downloadPageURL": "%s"\n}\n' \
    "$SHORT_VERSION" "$RELEASE_PAGE_URL")"
python3 -c 'import json,sys; json.loads(sys.stdin.read())' <<< "$MANIFEST_JSON" ||
    fail "generated manifest is not valid JSON: $MANIFEST_JSON"

MANIFEST_API="repos/$PUBLISH_REPOSITORY/contents/$MANIFEST_PATH"
declare -a CONTENT_ARGS=(
    -X PUT
    -f "message=chore: macOS update manifest -> $SHORT_VERSION"
    -f "content=$(printf '%s' "$MANIFEST_JSON" | base64 | tr -d '\n')"
)
# Updating an existing file requires the blob it replaces; creating one must not
# send a sha at all.
if EXISTING_SHA="$(gh api "$MANIFEST_API" --jq .sha 2>/dev/null)"; then
    CONTENT_ARGS+=(-f "sha=$EXISTING_SHA")
fi
gh api "$MANIFEST_API" "${CONTENT_ARGS[@]}" --jq '.commit.html_url'

echo "==> Waiting for $MANIFEST_URL to serve $SHORT_VERSION"
# GitHub Pages has to build and the CDN has to expire what it holds. Polling the
# real URL is the only thing that proves the release is actually announced;
# everything before this only proves it was committed.
for attempt in $(seq 1 30); do
    live_version="$(anonymous_curl --header 'Cache-Control: no-cache' "$MANIFEST_URL" 2>/dev/null |
        python3 -c 'import json,sys
try:
    print(json.load(sys.stdin).get("version") or "")
except Exception:
    print("")' || true)"
    [[ "$live_version" == "$SHORT_VERSION" ]] && break
    [[ $attempt -eq 30 ]] &&
        fail "manifest still serving '${live_version:-nothing}' after 5 minutes — check the Pages deployment"
    sleep 10
done

echo ""
echo "✓ published $SHORT_VERSION"
echo "  release   $RELEASE_PAGE_URL"
echo "  download  $ASSET_URL"
echo "  manifest  $MANIFEST_URL"
