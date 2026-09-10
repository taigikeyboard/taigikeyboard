#!/usr/bin/env bash
# Stage both desktop installers on this version's draft release: the package
# here, the installer on the Windows box over ssh.
#
# Usage: stage-desktop.sh [--skip-macos] [--skip-windows]
#
# The two builds cannot share a machine — one needs Xcode and a Developer ID,
# the other MSVC and Inno Setup — so this drives the second over `ssh win`
# rather than pretending they are one build. Everything it runs is the same
# `make macos-release` / `make windows-release` a person would type; what it
# adds is putting the box on the same commit first, and failing loudly when it
# cannot.
#
# Nothing here reaches a user: both halves stage on a DRAFT release
# (`docs/architecture/desktop-release.md`). Publishing stays a person's.
#
# Every run starts from a clean draft: an existing one for this version is
# DELETED first and both installers are built again (USER 2026-09-10 —
# 「我希望重複release的過程是原子性的,每一次都從新的開始建置,避免過多的複雜度」).
# Re-running is how a release is fixed, and a half-finished draft is state a
# script has to reason about — the first run of this script announced two
# installers when the draft held one, because it was reasoning about which
# halves were already there. There is nothing to reason about now.
#
# The box's checkout is moved to this commit with a detached checkout, which is
# what `desktop_release_preflight` requires of it: committed clean, and an
# ancestor of `origin/main`.

set -euo pipefail

fail() {
    echo "error: $*" >&2
    exit 1
}

REPOSITORY_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# The box, and where its clone lives. Overridable for a second machine without
# editing this file.
WINDOWS_SSH_HOST="${WINDOWS_SSH_HOST:-win}"
WINDOWS_REPO_DIR="${WINDOWS_REPO_DIR:-C:\\Users\\minsi\\Workspace\\taigikeyboard}"
WINDOWS_BASH="${WINDOWS_BASH:-C:\\Program Files\\Git\\bin\\bash.exe}"

skip_macos=false
skip_windows=false
while [[ $# -gt 0 ]]; do
    case "$1" in
        --skip-macos) skip_macos=true ;;
        --skip-windows) skip_windows=true ;;
        *)
            echo "error: unknown argument '$1'" >&2
            echo "usage: stage-desktop.sh [--skip-macos] [--skip-windows]" >&2
            exit 2
            ;;
    esac
    shift
done

SOURCE_COMMIT="$(git -C "$REPOSITORY_DIR" rev-parse HEAD)"
[[ -z "$(git -C "$REPOSITORY_DIR" status --porcelain --ignore-submodules=none)" ]] ||
    fail "the working tree is dirty — both halves refuse it, and the box would be put on a commit that is not what is here"
git -C "$REPOSITORY_DIR" fetch --quiet origin main
git -C "$REPOSITORY_DIR" merge-base --is-ancestor "$SOURCE_COMMIT" FETCH_HEAD ||
    fail "HEAD (${SOURCE_COMMIT:0:7}) is not on origin/main — push it first; the box can only fetch what origin has"

DESKTOP_VERSION="$(awk '
    /^\[workspace\.package\]/ { inside = 1; next }
    inside && /^\[/ { exit }
    inside && /^version *=/ { gsub(/[" ]/, "", $3); print $3; exit }
' "$REPOSITORY_DIR/windows/Cargo.toml" | tr -d '\r')"
DESKTOP_TAG="desktop-$DESKTOP_VERSION"
RELEASE_REPOSITORY="taigikeyboard/taigikeyboard"

echo "==> Staging $DESKTOP_TAG from ${SOURCE_COMMIT:0:7}"

# A draft for this version is the previous attempt; it goes, so this run builds
# both halves from one commit. A PUBLISHED release cannot be re-cut — its
# installers are downloadable and its tag is what people already have.
#
# Only a run that stages EVERYTHING may delete: a recovery run staging one half
# would otherwise throw away the half that is already on the draft, which is the
# one thing it was invoked to keep (2026-09-10: --skip-macos deleted a notarized
# package and cost a second notarization round).
existing_state="$(gh release view "$DESKTOP_TAG" --repo "$RELEASE_REPOSITORY" --json isDraft --jq '.isDraft|tostring' 2>&1)" || existing_state=""
case "$existing_state" in
    true)
        if [[ "$skip_macos" == true || "$skip_windows" == true ]]; then
            echo "  keeping the existing draft — this run stages one half into it"
        else
            echo "  removing the previous draft — every full run stages both halves afresh"
            gh release delete "$DESKTOP_TAG" --repo "$RELEASE_REPOSITORY" --yes ||
                fail "cannot remove the existing draft $DESKTOP_TAG"
        fi
        ;;
    false)
        fail "$DESKTOP_TAG is already published — it cannot be re-cut. Bump to the next version (make version-desktop) and stage that"
        ;;
    *)
        [[ "$existing_state" == *"release not found"* || "$existing_state" == *"HTTP 404"* || -z "$existing_state" ]] ||
            fail "cannot read $DESKTOP_TAG in $RELEASE_REPOSITORY: $existing_state"
        ;;
esac

# One half staged alone would contradict the fresh-draft rule the run above
# just applied, so the skips are for recovery, not for routine use.
if [[ "$skip_macos" == true || "$skip_windows" == true ]]; then
    echo "  note: staging one half only — the draft will hold one installer"
fi

if [[ "$skip_macos" == false ]]; then
    echo ""
    echo "==> macOS — build, sign, notarize, stage (this Mac)"
    make -C "$REPOSITORY_DIR" macos-release
else
    echo "  skipping macOS"
fi

if [[ "$skip_windows" == true ]]; then
    echo "  skipping Windows"
    exit 0
fi

echo ""
echo "==> Windows — $WINDOWS_SSH_HOST:$WINDOWS_REPO_DIR"
ssh -o ConnectTimeout=15 -o BatchMode=yes "$WINDOWS_SSH_HOST" "echo ok" > /dev/null 2>&1 ||
    fail "$WINDOWS_SSH_HOST is not reachable — power the box on, or re-run with --skip-windows and stage it later"

# One bash -lc: the box's default shell is PowerShell, and `make` needs Git
# Bash. GIT_SSH_COMMAND is what stops the fetch below from hanging — an ssh
# git spawns inside this ssh session inherits its stdout and both wait
# (reference_windows_dev_box: "nested ssh hangs").
remote_script="$(
    printf 'WINDOWS_REPO_DIR=%s\nSOURCE_COMMIT=%s\n' "$WINDOWS_REPO_DIR" "$SOURCE_COMMIT"
    cat <<'REMOTE'
set -euo pipefail
cd "$(cygpath "$WINDOWS_REPO_DIR")"
export GIT_SSH_COMMAND="ssh -o BatchMode=yes -o ConnectTimeout=15"

# An ssh session is not the interactive shell the box's PATH was set up for:
# the MSVC tools the release gates need (dumpbin's import-table check, and the
# linker behind cargo) are added by the Visual Studio environment, which only a
# developer prompt or a login shell runs. Ask vswhere where the toolchain is
# and put its x64 binaries on PATH rather than hard-coding a version.
if ! command -v dumpbin > /dev/null; then
    vs_root="$('/c/Program Files (x86)/Microsoft Visual Studio/Installer/vswhere.exe' \
        -latest -products '*' -property installationPath | tr -d '\r')"
    [ -n "$vs_root" ] || { echo "vswhere found no Visual Studio installation" >&2; exit 1; }
    msvc_bin="$(ls -d "$(cygpath "$vs_root")"/VC/Tools/MSVC/*/bin/Hostx64/x64 2> /dev/null | sort -V | tail -1)"
    [ -n "$msvc_bin" ] || { echo "no MSVC x64 tools under $vs_root" >&2; exit 1; }
    export PATH="$msvc_bin:$PATH"
fi

# The dev TIP is registered in place from this build tree, so a release build
# has to overwrite a DLL that explorer — or any host that has typed Taigi since
# — still has mapped (`os error 5`). `install-dev.ps1 unlock` renames both names
# the linker writes through out of the way, which Windows allows even for a
# loaded file.
#
# The build stays in the SHARED target directory on purpose. A release-only
# CARGO_TARGET_DIR removes the contention but means every build-script binary is
# newly created, and this box's App Control policy blocks those outright
# (`os error 4551`). Reusing the tree it has already admitted is what works
# here.
powershell.exe -NoProfile -ExecutionPolicy Bypass \
    -File windows/scripts/install-dev.ps1 unlock
git fetch --quiet origin main
git checkout --quiet --detach "$SOURCE_COMMIT"
git status --porcelain --ignore-submodules=none | head -5
make windows-release RELEASE_FLAGS=--skip-sign
REMOTE
)"

# `bash -s` reads the script from stdin. Passing it as an argument instead
# means PowerShell — the box's login shell — parses it first, and it split a
# multi-line script into positional arguments while still exiting 0, so the
# failure read as success (2026-09-10, first run of this script).
ssh "$WINDOWS_SSH_HOST" "& '$WINDOWS_BASH' -s" <<< "$remote_script" ||
    fail "staging on $WINDOWS_SSH_HOST failed — the log above is the box's; re-run with --skip-macos once it is fixed"

# PowerShell's exit status is not proof: ask the release what it now holds.
# Whatever the remote log said, an installer that is not on the draft is not
# staged.
WINDOWS_ASSET_NAME="TaigiKeyboard-$DESKTOP_VERSION.exe"

# The draft's own page, from the API: a draft has no tag, so its URL is not the
# `releases/tag/<tag>` address a published release has. It is where the
# maintainer downloads what was staged and, when it passes, presses Publish.
draft_json="$(gh release view "$DESKTOP_TAG" \
    --repo taigikeyboard/taigikeyboard --json url,assets 2> /dev/null || true)"
DRAFT_URL="$(printf '%s' "$draft_json" | python3 -c 'import json,sys; print(json.load(sys.stdin)["url"])' 2> /dev/null || true)"
printf '%s' "$draft_json" |
    python3 -c 'import json,sys; sys.exit(0 if sys.argv[1] in [a["name"] for a in json.load(sys.stdin)["assets"]] else 1)' \
        "$WINDOWS_ASSET_NAME" 2> /dev/null ||
    fail "the box reported no error but $WINDOWS_ASSET_NAME is not on the draft — read the remote log above; the macOS half is staged and can be left alone (re-run with --skip-macos)"

echo ""
echo "✓ both installers staged on the draft for ${SOURCE_COMMIT:0:7}"
echo ""
echo "  Open the draft, download both assets, install and test them:"
echo "    ${DRAFT_URL:-https://github.com/taigikeyboard/taigikeyboard/releases}"
echo ""
echo "  When they pass, press \"Publish release\" on that page. That is the whole"
echo "  remaining step: it tags the commit and announces the release itself."
