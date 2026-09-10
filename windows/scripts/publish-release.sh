#!/usr/bin/env bash
# Stage a built Windows installer (roadmap W8) on this version's DRAFT desktop
# release — one release per desktop version in this repository, holding both
# platforms' installers (`scripts/lib/desktop-release.sh`).
#
#   bash windows/scripts/publish-release.sh [--installer <path>] [--allow-unsigned]
#
# NOTHING HERE REACHES A USER. The release is a draft: no tag, no public
# download. The maintainer downloads what was staged, tests it, publishes the
# release by hand, and then `scripts/announce-release.sh` (`make
# desktop-announce`) writes `_data/windows_release.json` and waits for the live
# `appcast/windows.json` — the manifest every installed copy polls
# (windows/updates/README.md).
#
# An installer without a trusted Authenticode signature is refused unless
# --allow-unsigned says so out loud. That flag is how this project ships today
# (docs/architecture/windows-release.md § Signing status); release-app.sh
# passes it down when it was itself run with --skip-sign, so a direct
# invocation of this script cannot publish unsigned by accident.

set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib/identity.sh"
# shellcheck source=../../scripts/lib/desktop-release.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/../../scripts/lib/desktop-release.sh"

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
[[ -n "$installer_path" ]] || installer_path="$DISTRIBUTION_DIR/$WINDOWS_ASSET"
[[ -f "$installer_path" ]] || fail "no installer at $installer_path — run 'make windows-release' first"
INSTALLER_NAME="$(basename "$installer_path")"
installer_path="$(cd "$(dirname "$installer_path")" && pwd)/$INSTALLER_NAME"

echo "==> Verifying the installer is publishable"
[[ "$allow_unsigned" == false || -z "${WINDOWS_SIGNING_THUMBPRINT:-}" ]] ||
    fail "--allow-unsigned and WINDOWS_SIGNING_THUMBPRINT contradict: a certificate is named, so sign the installer instead of publishing it unsigned"
[[ "$INSTALLER_NAME" == "$WINDOWS_ASSET" ]] ||
    fail "$INSTALLER_NAME is not the release name for $SHORT_VERSION (a -dirty build is not publishable)"
# Staging needs a digest and PowerShell's file metadata; the announcement's
# tools (curl, python3) are `scripts/announce-release.sh`'s problem, and it can
# run on either machine.
declare -a REQUIRED_TOOLS=(sha256sum powershell.exe)
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
# (taigi-windows-update::verify::admit): the digest staged beside the
# installer, this product, this version, and — when the release
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

stage_desktop_asset "$installer_path"
desktop_draft_summary Windows

if [[ "$allow_unsigned" == true ]]; then
    echo ""
    echo "  ⚠ staged UNSIGNED. SmartScreen typically warns (其他資訊 → 仍要執行) and Win11 Smart App"
    echo "    Control can refuse it — worth checking during the manual test. Installed copies DO"
    echo "    still update in-app, on the published SHA-256."
fi
