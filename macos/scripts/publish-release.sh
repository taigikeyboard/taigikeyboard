#!/usr/bin/env bash
# Publish an already-built, notarized package: attach it to this version's
# desktop release, prove it is anonymously downloadable, then point the update
# manifest at it. Split from release-app.sh because the two fail for unrelated
# reasons and rebuilding costs a notarization round trip — a failed upload must
# not mean waiting through that again.
#
# Usage: publish-release.sh [--pkg <path>]
#
#   --pkg <path>  Package to publish (default: the release-named pkg for the
#                 version in App/Info.plist).
#
# The release itself — one per desktop version, holding both platforms'
# installers — is `scripts/lib/desktop-release.sh`. This script owns what only
# macOS can say: that the package is notarized, is this app, and is this
# version, and what the Mac's own update manifest then announces.
#
# Re-running after a failure is safe and is the intended recovery: an existing
# release is added to, never deleted, because the manifest may already be
# pointing at it, and an asset already published is kept rather than replaced.

set -euo pipefail

# shellcheck source=lib/bundle-identity.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib/bundle-identity.sh"
# shellcheck source=../../scripts/lib/release-site.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/../../scripts/lib/release-site.sh"
# shellcheck source=../../scripts/lib/desktop-release.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/../../scripts/lib/desktop-release.sh"

# The one file a release writes to the website repository. The site's macOS download button
# links straight at the package, so its URL carries the version and changes
# every release; it is committed as site data rather than written into the page,
# which keeps this script the only thing that edits it — and keeps the button
# off `/releases/latest`, which resolves repository-wide and would hand a Mac
# whatever was released last, Windows installer included.
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

# ---------------------------------------------------------------------------
# Preflight. The shared half first, so a dirty tree or an uncommitted changelog
# stops before Gatekeeper is asked anything.
# ---------------------------------------------------------------------------

desktop_release_preflight

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
# In the publish's scratch dir — `desktop-release.sh` owns the only EXIT trap.
PACKAGE_IDENTITY_DIR="$RELEASE_TEMP_DIR/package-identity"
mkdir -p "$PACKAGE_IDENTITY_DIR"
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

publish_desktop_asset "$pkg_path"

# `downloadURL` reaches the app as the manifest's `packageURL`, which is what
# lets it fetch the installer itself instead of sending the user to a browser;
# an install that reads it still verifies the package's own Developer ID
# signature, so the URL is a convenience rather than something trusted.
SITE_RELEASE_JSON="$(printf '{\n  "version": "%s",\n  "tag": "%s",\n  "downloadURL": "%s",\n  "releasePageURL": "%s"\n}\n' \
    "$SHORT_VERSION" "$DESKTOP_TAG" "$ASSET_URL" "$RELEASE_PAGE_URL")"

announce_desktop_platform macOS "$SITE_RELEASE_PATH" "$MANIFEST_URL" "$SITE_RELEASE_JSON" \
    "version=$SHORT_VERSION" "packageURL=$ASSET_URL"
