#!/usr/bin/env bash
# Publish a built Windows installer (roadmap W8): attach it to this version's
# desktop release (`scripts/lib/desktop-release.sh` — one release per desktop
# version in this repository, holding both platforms' installers), verified
# reachable WITHOUT credentials, then ONE committed file,
# `_data/windows_release.json` (the site's download button; the site renders
# `appcast/windows.json` — what every installed copy polls,
# windows/updates/README.md — from it), and a wait until the live manifest
# serves the new version, its URL and its SHA-256. Mirror of
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
# shellcheck source=../../scripts/lib/release-site.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/../../scripts/lib/release-site.sh"
# shellcheck source=../../scripts/lib/desktop-release.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/../../scripts/lib/desktop-release.sh"

# The one file a release writes to the website repository. The site's Windows download button
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

# The shared half first, so a dirty tree or an uncommitted changelog stops
# before signtool is asked anything.
desktop_release_preflight
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
# (taigi-windows-update::verify::admit): the published digest (read back by
# `publish_desktop_asset`), this product, this version, and — when the release
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

publish_desktop_asset "$installer_path"

# `downloadURL` reaches the input method as the manifest's `packageURL` and
# `sha256` as its `packageSHA256`: together they are what lets an installed
# copy fetch and admit the installer itself instead of sending the user to a
# browser. BOTH are required for that — a manifest naming a package with no
# digest offers the download page, because an unsigned copy has nothing else
# to hold the download against (`taigi-windows-update::verify::admit`). A
# signed copy checks its Authenticode signature on top.
SITE_RELEASE_JSON="$(printf '{\n  "version": "%s",\n  "tag": "%s",\n  "downloadURL": "%s",\n  "sha256": "%s",\n  "releasePageURL": "%s"\n}\n' \
    "$SHORT_VERSION" "$DESKTOP_TAG" "$ASSET_URL" "$PUBLISHED_SHA256" "$RELEASE_PAGE_URL")"

announce_desktop_platform Windows "$SITE_RELEASE_PATH" "$MANIFEST_URL" "$SITE_RELEASE_JSON" \
    "version=$SHORT_VERSION" "packageURL=$ASSET_URL" "packageSHA256=$PUBLISHED_SHA256"

if [[ "$allow_unsigned" == true ]]; then
    echo ""
    echo "  ⚠ published UNSIGNED. SmartScreen typically warns (其他資訊 → 仍要執行) and Win11 Smart App"
    echo "    Control can refuse it. Installed copies DO still update in-app, on the published SHA-256."
fi
