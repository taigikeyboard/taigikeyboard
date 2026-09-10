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

echo "==> Staging desktop ${SOURCE_COMMIT:0:7} on the draft release"

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
remote_script=$(cat <<REMOTE
set -euo pipefail
cd "\$(cygpath '$WINDOWS_REPO_DIR')"
export GIT_SSH_COMMAND="ssh -o BatchMode=yes -o ConnectTimeout=15"
git fetch --quiet origin main
git checkout --quiet --detach $SOURCE_COMMIT
git status --porcelain --ignore-submodules=none | head -5
make windows-release RELEASE_FLAGS=--skip-sign
REMOTE
)

ssh "$WINDOWS_SSH_HOST" "& '$WINDOWS_BASH' -lc \"\$(cat)\"" <<< "$remote_script" ||
    fail "staging on $WINDOWS_SSH_HOST failed — the log above is the box's; re-run this script with --skip-macos once it is fixed"

echo ""
echo "✓ both installers staged on the draft for ${SOURCE_COMMIT:0:7}"
echo "  test them, then publish:"
echo "    gh release download desktop-<version> --repo taigikeyboard/taigikeyboard --dir ~/Downloads"
echo "    gh release edit desktop-<version> --repo taigikeyboard/taigikeyboard --draft=false"
