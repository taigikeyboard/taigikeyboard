#!/usr/bin/env bash
# Stage the desktop installers on this version's draft release: the package
# built here, the Windows installer and the Linux packages built on
# GitHub-hosted runners.
#
# Usage: stage-desktop.sh              a full release — every platform
#        stage-desktop.sh <platform>   a patch release — macos | windows | linux
#
# The builds cannot share a machine — one needs Xcode and a Developer ID, one
# MSVC and Inno Setup, one a Linux with the Fcitx5 headers — so this runs the
# first here and dispatches the others to CI, then waits for each. All build
# from this commit.
#
# Nothing here reaches a user: every half stages on a DRAFT release
# (`docs/architecture/desktop-release.md`). Publishing stays a person's.
#
# Every run stages ALL installers from one commit, and starts from a clean
# draft: an existing one for this version is deleted first (USER 2026-09-10 —
# "I want a repeated release to be atomic, every run building from scratch";
# 2026-09-11 — "I do not want --skip-macos or skip-windows; I want both built together in one go").
#
# A PATCH release is its own version holding ONE platform's installers (USER
# 2026-09-24 — "let each desktop version take a patch on its own, without running the release
# pipeline from the top"): 3.7.0 ships every platform, 3.7.1 only the platform it fixes.
# That is not a way to stage one half of a release — the old escapes that
# stranded a draft with installers from two commits stay gone. The draft still
# starts clean and everything on it is built from one commit; it just names
# fewer platforms, and announcing leaves the others' downloads and update
# manifests on the version they have.
#

set -euo pipefail

fail() {
    echo "error: $*" >&2
    exit 1
}

REPOSITORY_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

case "$#:${1:-}" in
    0:) PLATFORMS=(macos windows linux) ;;
    1:macos | 1:windows | 1:linux) PLATFORMS=("$1") ;;
    *)
        echo "usage: stage-desktop.sh [macos|windows|linux]" >&2
        echo "No argument stages a full release; one platform stages a patch release of it alone." >&2
        exit 2
        ;;
esac

stages() {
    [[ " ${PLATFORMS[*]} " == *" $1 "* ]]
}

SOURCE_COMMIT="$(git -C "$REPOSITORY_DIR" rev-parse HEAD)"
[[ -z "$(git -C "$REPOSITORY_DIR" status --porcelain --ignore-submodules=none)" ]] ||
    fail "the working tree is dirty — every half refuses it, and the hosted builds would run a commit that is not what is here"
git -C "$REPOSITORY_DIR" fetch --quiet origin main
git -C "$REPOSITORY_DIR" merge-base --is-ancestor "$SOURCE_COMMIT" FETCH_HEAD ||
    fail "HEAD (${SOURCE_COMMIT:0:7}) is not on origin/main — push it first; the hosted runners can only build what origin has"

# The release object's names — DESKTOP_TAG, RELEASE_REPOSITORY and each
# platform's asset — from the one library every release script reads, so the
# names this checks cannot drift from the ones the halves stage.
# Emptied so the library reads the version from the checkout, never from an
# inherited environment variable.
SHORT_VERSION=""
# shellcheck source=lib/desktop-release.sh
source "$REPOSITORY_DIR/scripts/lib/desktop-release.sh"

echo "==> Staging $DESKTOP_TAG (${PLATFORMS[*]}) from ${SOURCE_COMMIT:0:7}"

# A draft for this version is the previous attempt; it goes, so this run builds
# both halves from one commit. A PUBLISHED release cannot be re-cut — its
# installers are downloadable and its tag is what people already have.
existing_state="$(gh release view "$DESKTOP_TAG" --repo "$RELEASE_REPOSITORY" --json isDraft --jq '.isDraft|tostring' 2>&1)" || existing_state=""
case "$existing_state" in
    true)
        echo "  removing the previous draft — every run stages every half afresh"
        gh release delete "$DESKTOP_TAG" --repo "$RELEASE_REPOSITORY" --yes ||
            fail "cannot remove the existing draft $DESKTOP_TAG"
        ;;
    false)
        fail "$DESKTOP_TAG is already published — it cannot be re-cut. Bump to the next version (make version-desktop) and stage that"
        ;;
    *)
        [[ "$existing_state" == *"release not found"* || "$existing_state" == *"HTTP 404"* || -z "$existing_state" ]] ||
            fail "cannot read $DESKTOP_TAG in $RELEASE_REPOSITORY: $existing_state"
        ;;
esac

if stages macos; then
    echo ""
    echo "==> macOS — build, sign, notarize, stage (this Mac)"
    make -C "$REPOSITORY_DIR" macos-release
fi

# The installer and the package are built by `.github/workflows/windows-build.yml`
# and `linux-build.yml` on clean GitHub-hosted runners, never on the
# maintainer's Windows box (`windows-release.md` § Building on a GitHub-hosted
# runner says why); the Linux half has no machine of its own at all.
#
# workflow_dispatch takes a branch or tag, never a bare SHA, so the commit being
# staged has to be what `main` points at — which it is, since the release prep
# was just pushed there. Both halves are dispatched together, right after that
# check, so main has no wait to move in; the Linux attach job still refuses any
# commit but the one it was handed.
git -C "$REPOSITORY_DIR" fetch --quiet origin main
[[ "$SOURCE_COMMIT" == "$(git -C "$REPOSITORY_DIR" rev-parse FETCH_HEAD)" ]] ||
    fail "HEAD (${SOURCE_COMMIT:0:7}) is not origin/main's tip — the hosted builds run from main, so push this commit first"

# Dispatches `$1` (a workflow file) on main and prints the id of the run it
# started. `gh workflow run` returns nothing that identifies the run, so the one
# built from this commit and started after the dispatch is looked up.
dispatch() {
    local workflow="$1" what="$2" run_id="" dispatched_at
    shift 2
    dispatched_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    gh workflow run "$workflow" --repo "$RELEASE_REPOSITORY" --ref main "$@" > /dev/null ||
        fail "cannot dispatch the $what build"
    for _ in $(seq 1 30); do
        run_id="$(gh run list --repo "$RELEASE_REPOSITORY" --workflow "$workflow" \
            --json databaseId,headSha,createdAt,event \
            --jq "[.[] | select(.headSha == \"$SOURCE_COMMIT\" and .event == \"workflow_dispatch\" and .createdAt >= \"$dispatched_at\")] | first | .databaseId // empty")"
        [[ -n "$run_id" ]] && break
        sleep 5
    done
    [[ -n "$run_id" ]] ||
        fail "the dispatched $what run never appeared — check $RELEASE_REPOSITORY's Actions tab"
    echo "$run_id"
}

wait_for() {
    local run_id="$1" what="$2"
    echo ""
    echo "==> $what — https://github.com/$RELEASE_REPOSITORY/actions/runs/$run_id"
    gh run watch "$run_id" --repo "$RELEASE_REPOSITORY" --exit-status ||
        fail "the $what build failed — its log is at the URL above; fix it and run this again, which re-stages every half"
}

# The hosted attach steps join a draft and never create one; without the
# macOS half, the draft is made here, on this commit, before they run.
if ! stages macos; then
    echo ""
    desktop_release_preflight
    ensure_desktop_draft
fi

windows_run=""
linux_run=""
if stages windows || stages linux; then
    echo ""
    echo "==> Hosted runners — dispatched together, waited on in turn"
fi
stages windows && windows_run="$(dispatch windows-build.yml Windows)"
# The commit travels with the dispatch: the Linux attach job refuses any other.
stages linux && linux_run="$(dispatch linux-build.yml Linux -f "source_sha=$SOURCE_COMMIT")"
[[ -z "$windows_run" ]] || wait_for "$windows_run" Windows
[[ -z "$linux_run" ]] || wait_for "$linux_run" Linux

# The draft's own page, from the API: a draft has no tag, so its URL is not the
# `releases/tag/<tag>` address a published release has. It is where the
# maintainer downloads what was staged and, when it passes, presses Publish.
DRAFT_URL="$(gh release view "$DESKTOP_TAG" --repo "$RELEASE_REPOSITORY" --json url --jq .url)" ||
    fail "cannot read the draft $DESKTOP_TAG back"
# Every installer and its digest. Checked at the end, because a run that
# reports success and attaches nothing is exactly what happened the first time
# this script drove a second machine.
_read_staged_asset_names || fail "the draft $DESKTOP_TAG is gone — run this again"
expected_assets=()
stages macos && expected_assets+=("$MACOS_ASSET")
stages windows && expected_assets+=("$WINDOWS_ASSET")
stages linux && expected_assets+=("$LINUX_ASSET" "$LINUX_RPM_ASSET" "$LINUX_ARCH_ASSET")
for asset in "${expected_assets[@]}"; do
    for name in "$asset" "$asset.sha256"; do
        grep -qxF "$name" <<< "$STAGED_ASSET_NAMES" ||
            fail "every half reported success but $name is not on the draft — read the logs above, then run this again"
    done
done

echo ""
echo "✓ ${expected_assets[*]} staged on the draft for ${SOURCE_COMMIT:0:7}"
echo ""
echo "  Open the draft, download the assets, install and test them:"
echo "    $DRAFT_URL"
echo ""
echo "  When they pass, press \"Publish release\" on that page. That is the whole"
echo "  remaining step: it tags the commit and announces the release itself."
