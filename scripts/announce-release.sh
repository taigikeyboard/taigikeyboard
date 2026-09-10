#!/usr/bin/env bash
# Announce a desktop release that a person has already published: prove each
# platform's installer is anonymously downloadable, then point the website — and
# through it every installed copy — at it.
#
# Usage: announce-release.sh [--version <x.y.z>]
#
# This is the second half of the desktop release. The first half
# (`make macos-release` / `make windows-release`, via
# `scripts/lib/desktop-release.sh`) stages each platform's installer on a DRAFT
# release, which no user can reach. Between the two halves the maintainer
# downloads what was staged, tests it, and publishes the release by hand. Only
# then does anything reach a user, and this is what tells them:
#
#   1. refuse while the release is still a draft — a manifest that named a draft
#      would point every installed copy at a 404
#   2. for each platform's installer on the release: download it anonymously and
#      require its SHA-256 to equal the `.sha256` receipt staged beside it, which
#      was written from the bytes that were built and tested
#   3. write both platforms' `_data/*_release.json` into the website repository
#      in one commit
#   4. wait until each live appcast serves what was written
#
# It runs anywhere with `gh`, `curl` and `python3` — the Mac, the Windows box,
# or neither: everything it needs is on the release. Re-running it after a
# failure is the intended recovery.

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
# The release must be published, and must be the one that was staged.
# ---------------------------------------------------------------------------

release_json="$(gh release view "$DESKTOP_TAG" --repo "$RELEASE_REPOSITORY" \
    --json isDraft,targetCommitish,assets 2>&1)" ||
    fail "no release $DESKTOP_TAG in $RELEASE_REPOSITORY: $release_json"
# One read, every fact this needs. Asset names come back one per line so a name
# is matched whole: `TaigiKeyboard-3.6.8.pkg` and `TaigiKeyboard-3.6.8.pkg.sha256`
# differ only by a suffix.
{
    read -r is_draft
    read -r recorded_commit
    staged_assets="$(cat)"
} < <(printf '%s' "$release_json" | python3 -c '
import json, sys
release = json.load(sys.stdin)
print("true" if release["isDraft"] else "false")
print(release["targetCommitish"])
print("\n".join(asset["name"] for asset in release["assets"]))')

[[ "$is_draft" == false ]] ||
    fail "$DESKTOP_TAG is still a draft — test what it holds, then publish it (gh release edit $DESKTOP_TAG --repo $RELEASE_REPOSITORY --draft=false) and run this again"

# Publishing is what creates the tag, from the commit `--target` recorded when
# the first platform staged its installer — the commit the staging step refused
# to let drift. Compare the two rather than trusting either: a tag created by
# hand between staging and publishing would otherwise announce a release whose
# tag does not describe what is in it.
tagged_commit="$(desktop_tag_commit)"
[[ -n "$tagged_commit" ]] ||
    fail "$DESKTOP_TAG is published but carries no tag — publish it again from the release page"
[[ "$tagged_commit" == "$recorded_commit" ]] ||
    fail "$DESKTOP_TAG names commit ${tagged_commit:0:7}, but the release was staged from ${recorded_commit:0:7} — the tag was moved or created by hand; do not announce it"
echo "==> $DESKTOP_TAG is published, tagging ${tagged_commit:0:7}"

# ---------------------------------------------------------------------------
# Per platform: prove the download, against the digest staged before testing.
# ---------------------------------------------------------------------------

# A desktop release usually carries both installers, but one platform can lag,
# and announcing the platform that is there beats making it wait.
announced_platforms=()

# Sets ASSET_URL + ASSET_SHA256 and returns 0 when this platform's installer is
# on the release; returns 1 when it is not.
verify_platform_asset() {
    local platform="$1" asset_name="$2"
    local asset_url receipt_url downloaded receipt staged_sha256 published_sha256 attempt

    if ! grep -qxF "$asset_name" <<< "$staged_assets"; then
        echo "==> $platform: no $asset_name on $DESKTOP_TAG — leaving its manifest alone"
        return 1
    fi

    asset_url="https://github.com/$RELEASE_REPOSITORY/releases/download/$DESKTOP_TAG/$asset_name"
    receipt_url="$asset_url.sha256"
    downloaded="$RELEASE_TEMP_DIR/$asset_name"
    receipt="$RELEASE_TEMP_DIR/$asset_name.sha256"

    echo "==> $platform: reading $asset_name back without credentials"
    # Anonymously, because that is how every user and every installed copy will
    # reach it, and whole, because the digest about to be published has to be
    # the one this URL actually serves. GitHub can take a moment to serve a
    # freshly published asset, so an unreachable one is retried.
    for attempt in 1 2 3 4 5; do
        if anonymous_download "$receipt_url" "$receipt" &&
            anonymous_download "$asset_url" "$downloaded"; then
            break
        fi
        [[ $attempt -eq 5 ]] &&
            fail "$asset_url is not anonymously reachable — is $RELEASE_REPOSITORY public, and did the publish finish?"
        echo "  not reachable yet — retrying in 5s"
        sleep 5
    done

    staged_sha256="$(cut -d' ' -f1 < "$receipt")"
    [[ "$staged_sha256" =~ ^[0-9a-f]{64}$ ]] ||
        fail "$receipt_url does not hold a SHA-256 ('$staged_sha256') — it is not the receipt this flow staged"
    published_sha256="$(release_sha256 "$downloaded")"
    [[ -n "$published_sha256" ]] || fail "cannot hash the download from $asset_url"
    # The receipt was written from the bytes that were staged, before anyone
    # could test them, so this catches an asset replaced afterwards — the case a
    # digest recomputed from GitHub's own record cannot see. What binds those
    # bytes to "tested" is the maintainer having tested this release's download;
    # re-staging a version after testing it defeats that, which is why a staged
    # asset is never replaced.
    [[ "$published_sha256" == "$staged_sha256" ]] ||
        fail "$asset_url serves $published_sha256, but $asset_name.sha256 on the release says $staged_sha256 — the asset is not the one that was staged and tested; do not announce it"
    echo "  sha256 $published_sha256"

    ASSET_SHA256="$published_sha256"
    ASSET_URL="$asset_url"
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

# Each platform's results are bound as soon as they are produced: the verifier
# reports through globals, and two calls sharing them must not depend on order.
if verify_platform_asset macOS "$MACOS_ASSET"; then
    macos_url="$ASSET_URL"
    site_files+=("$MACOS_SITE_PATH" "$(site_release_json "$macos_url")")
    macos_manifest_fields=("version=$SHORT_VERSION" "packageURL=$macos_url")
fi
if verify_platform_asset Windows "$WINDOWS_ASSET"; then
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
# Only now announce it. Everything above proved the download is reachable; a
# manifest published before that points every installed copy at a 404, and the
# maintainer's own browser, being authenticated, cannot see it happen.
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
