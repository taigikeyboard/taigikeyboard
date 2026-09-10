#!/usr/bin/env bash
# Announce a desktop release that has been published: point the website — and
# through it every installed copy — at its installers.
#
# Usage: announce-release.sh [--version <x.y.z>]
#
# This is the second half of the desktop release. The first half
# (`make macos-release` / `make windows-release`, via
# `scripts/lib/desktop-release.sh`) stages each platform's installer on a DRAFT
# release, which no user can reach. Between the two halves the maintainer
# downloads what was staged, tests it, and publishes the release by hand.
# Publishing is what fires `.github/workflows/announce-release.yml`, which runs
# this; `make desktop-announce` is the same thing by hand.
#
#   1. refuse while the release is still a draft — a manifest naming a draft
#      points every installed copy at a download that does not exist
#   2. for each platform's installer on the release: download it anonymously and
#      hash it — the digest the Windows manifest publishes has to be the one the
#      URL actually serves
#   3. write both platforms' `_data/*_release.json` into the website repository
#      in one commit
#   4. wait until each live appcast serves what was written
#
# It runs anywhere with `gh`, `curl` and `python3` — everything it needs is on
# the release. Re-running it after a failure is the intended recovery.

set -euo pipefail

fail() {
    echo "error: $*" >&2
    exit 1
}

REPOSITORY_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

version=""
while [[ $# -gt 0 ]]; do
    case "$1" in
        --version)
            [[ $# -ge 2 ]] || { echo "error: --version needs x.y.z" >&2; exit 2; }
            version="$2"
            shift
            ;;
        *)
            echo "error: unknown argument '$1'" >&2
            echo "usage: announce-release.sh [--version <x.y.z>]" >&2
            exit 2
            ;;
    esac
    shift
done

# Empty unless --version was passed: the shared library then reads the desktop
# train's version out of the checkout.
SHORT_VERSION="$version"
[[ -z "$SHORT_VERSION" || "$SHORT_VERSION" =~ ^[0-9]+(\.[0-9]+)*$ ]] ||
    fail "version '$SHORT_VERSION' is not dotted integers — the update manifests reject suffixes"

# shellcheck source=lib/release-site.sh
source "$REPOSITORY_DIR/scripts/lib/release-site.sh"
# The release object: its tag, its repository, the per-platform asset names and
# `release_sha256`. Its staging half (`desktop_release_preflight`,
# `stage_desktop_asset`) is the other script's; nothing here calls it.
# shellcheck source=lib/desktop-release.sh
source "$REPOSITORY_DIR/scripts/lib/desktop-release.sh"

desktop_release_scratch_and_tools

# ---------------------------------------------------------------------------
# The release has to be published before anything is said about it.
# ---------------------------------------------------------------------------

release_json="$(gh release view "$DESKTOP_TAG" --repo "$RELEASE_REPOSITORY" \
    --json isDraft,assets 2>&1)" ||
    fail "no release $DESKTOP_TAG in $RELEASE_REPOSITORY: $release_json"
# One read, both facts. Asset names come back one per line so a name is matched
# whole: `TaigiKeyboard-3.6.8.pkg` and `TaigiKeyboard-3.6.8.pkg.sha256` differ
# only by a suffix.
{
    read -r is_draft
    staged_assets="$(cat)"
} < <(printf '%s' "$release_json" | python3 -c '
import json, sys
release = json.load(sys.stdin)
print("true" if release["isDraft"] else "false")
print("\n".join(asset["name"] for asset in release["assets"]))')

[[ "$is_draft" == false ]] ||
    fail "$DESKTOP_TAG is still a draft — test what it holds, then publish it (gh release edit $DESKTOP_TAG --repo $RELEASE_REPOSITORY --draft=false); publishing runs this automatically"

# ---------------------------------------------------------------------------
# Per platform: the download the website is about to name has to work.
# ---------------------------------------------------------------------------

# A desktop release usually carries both installers, but one platform can lag,
# and announcing the platform that is there beats making it wait.
announced_platforms=()

# Sets ASSET_URL + ASSET_SHA256 and returns 0 when this platform's installer is
# on the release; returns 1 when it is not.
fetch_platform_asset() {
    local platform="$1" asset_name="$2"
    local asset_url downloaded attempt

    if ! grep -qxF "$asset_name" <<< "$staged_assets"; then
        echo "==> $platform: no $asset_name on $DESKTOP_TAG — leaving its manifest alone"
        return 1
    fi

    asset_url="https://github.com/$RELEASE_REPOSITORY/releases/download/$DESKTOP_TAG/$asset_name"
    downloaded="$RELEASE_TEMP_DIR/$asset_name"

    echo "==> $platform: reading $asset_name back without credentials"
    # Anonymously, because that is how every user and every installed copy will
    # reach it — an authenticated check cannot tell a public URL from a private
    # one, which is how the first version of this shipped pointing at a private
    # repository. Whole, because the digest the Windows manifest publishes has to
    # be the one this URL serves. GitHub can take a moment to serve a freshly
    # published asset, so an unreachable one is retried.
    for attempt in 1 2 3 4 5; do
        anonymous_download "$asset_url" "$downloaded" && break
        [[ $attempt -eq 5 ]] &&
            fail "$asset_url is not anonymously reachable — is $RELEASE_REPOSITORY public, and did the publish finish?"
        echo "  not reachable yet — retrying in 5s"
        sleep 5
    done

    ASSET_SHA256="$(release_sha256 "$downloaded")"
    [[ "$ASSET_SHA256" =~ ^[0-9a-f]{64}$ ]] || fail "cannot hash the download from $asset_url"
    ASSET_URL="$asset_url"
    echo "  sha256 $ASSET_SHA256"
    announced_platforms+=("$platform")
}

# The site data the website renders its appcast from. `sha256` is Windows-only:
# an unsigned installer has nothing else to be held against, while a package
# carries Apple's own signature, so it is dropped when no digest is passed.
site_release_json() {
    python3 -c '
import json, sys
version, tag, download_url, page_url, digest = sys.argv[1:6]
release = {"version": version, "tag": tag, "downloadURL": download_url}
if digest:
    release["sha256"] = digest
release["releasePageURL"] = page_url
print(json.dumps(release, indent=2))
' "$SHORT_VERSION" "$DESKTOP_TAG" "$1" "$RELEASE_PAGE_URL" "${2:-}"
}

declare -a site_files=()
declare -a macos_manifest_fields=() windows_manifest_fields=()

# Each platform's results are bound as soon as they are produced: the fetch
# reports through globals, and two calls sharing them must not depend on order.
if fetch_platform_asset macOS "$MACOS_ASSET"; then
    macos_url="$ASSET_URL"
    site_files+=("$MACOS_SITE_PATH" "$(site_release_json "$macos_url")")
    macos_manifest_fields=("version=$SHORT_VERSION" "packageURL=$macos_url")
fi
if fetch_platform_asset Windows "$WINDOWS_ASSET"; then
    windows_url="$ASSET_URL"
    windows_sha256="$ASSET_SHA256"
    site_files+=("$WINDOWS_SITE_PATH" "$(site_release_json "$windows_url" "$windows_sha256")")
    windows_manifest_fields=(
        "version=$SHORT_VERSION" "packageURL=$windows_url" "packageSHA256=$windows_sha256"
    )
fi

[[ ${#site_files[@]} -gt 0 ]] ||
    fail "$DESKTOP_TAG carries neither $MACOS_ASSET nor $WINDOWS_ASSET — there is nothing to announce"

# ---------------------------------------------------------------------------
# Only now announce it. A manifest published before its download is reachable
# points every installed copy at a 404.
# ---------------------------------------------------------------------------

echo "==> Publishing the release data for ${announced_platforms[*]}"
commit_site_files "chore: desktop release -> $SHORT_VERSION (${announced_platforms[*]})" "${site_files[@]}"

[[ ${#macos_manifest_fields[@]} -eq 0 ]] ||
    wait_for_manifest "$MACOS_MANIFEST_URL" "${macos_manifest_fields[@]}"
[[ ${#windows_manifest_fields[@]} -eq 0 ]] ||
    wait_for_manifest "$WINDOWS_MANIFEST_URL" "${windows_manifest_fields[@]}"

echo ""
echo "✓ announced $SHORT_VERSION for ${announced_platforms[*]}"
echo "  release   $RELEASE_PAGE_URL"
echo "  website   https://taigikeyboard.tw/#download"
