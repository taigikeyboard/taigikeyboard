#!/usr/bin/env bash
# The Windows-only crates' real gate (roadmap W13 amendment, W17): clippy +
# tests on the box, for the MSVC target that ships. The macOS gnu check only
# type-checks them — `windows-reactor-setup` refuses the gnu target,
# `windows-reactor` has no host build at all, and the platform crate's key
# translation is the Win32 keyboard API itself (so its tests cannot run on
# the macOS host either).
#
# Gates the PUSHED commit (commit-first ordering): in Git, HEAD (warning if the
# working tree is dirty); in colocated jj, @ when non-empty/bookmarked/merge or
# clean @- (failing on unresolved conflicts). The box keeps its own gate clone,
# fetches the SHA, checks it out detached and runs cargo there. An unreachable
# box or a failing command is a gate FAILURE — never a skip.
#
#   TAIGI_WINDOWS_BOX       ssh host (default: win)
#   TAIGI_WINDOWS_BOX_REPO  the gate clone on the box
#                           (default: C:/Workspace/taigikeyboard-gate)

set -euo pipefail

fail() {
    echo "check-box: $*" >&2
    exit 1
}

resolve_gate_sha() {
    local repo_dir="$1"
    if [[ -d "$repo_dir/.jj" ]] && command -v jj >/dev/null 2>&1; then
        local jj_state jj_commit jj_empty jj_parents jj_bookmarks jj_conflict
        jj_state="$(jj --no-pager --color=never -R "$repo_dir" log -r '@' --no-graph \
            -T 'commit_id ++ " " ++ empty ++ " " ++ parents.len() ++ " " ++ local_bookmarks.len() ++ " " ++ conflict ++ "\n"')" ||
            fail "cannot inspect jj working copy (@)"
        read -r jj_commit jj_empty jj_parents jj_bookmarks jj_conflict <<< "$jj_state"
        [[ -n "$jj_commit" ]] || fail "cannot resolve jj working copy (@)"
        if [[ "$jj_empty" == "true" && "$jj_parents" == "1" && "$jj_bookmarks" == "0" ]]; then
            local parent_state parent_commit parent_conflict
            parent_state="$(jj --no-pager --color=never -R "$repo_dir" --ignore-working-copy log -r '@-' --no-graph \
                -T 'commit_id ++ " " ++ conflict ++ "\n"')" ||
                fail "cannot resolve jj parent commit (@-)"
            read -r parent_commit parent_conflict <<< "$parent_state"
            [[ -n "$parent_commit" ]] || fail "cannot resolve jj parent commit (@-)"
            [[ "$parent_conflict" != "true" ]] ||
                fail "commit ${parent_commit:0:7} has unresolved jj conflicts"
            printf '%s\n' "$parent_commit"
        else
            [[ "$jj_conflict" != "true" ]] ||
                fail "commit ${jj_commit:0:7} has unresolved jj conflicts"
            printf '%s\n' "$jj_commit"
        fi
        return 0
    fi

    local sha
    sha="$(git -C "$repo_dir" rev-parse HEAD)" || fail "cannot resolve HEAD"
    if ! git -C "$repo_dir" diff --quiet || ! git -C "$repo_dir" diff --cached --quiet; then
        echo "check-box: warning — the working tree is dirty; the box checks $sha, not this tree" >&2
    fi
    printf '%s\n' "$sha"
}

BOX="${TAIGI_WINDOWS_BOX:-win}"
BOX_REPO="${TAIGI_WINDOWS_BOX_REPO:-C:/Workspace/taigikeyboard-gate}"
REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

case "${1:-}" in
    "") ;;
    --print-sha)
        [[ $# -le 2 ]] || { echo "usage: check-box.sh [--print-sha [repo_dir]]" >&2; exit 2; }
        resolve_gate_sha "${2:-$REPO_DIR}"
        exit 0
        ;;
    *)
        echo "usage: check-box.sh [--print-sha [repo_dir]]" >&2
        exit 2
        ;;
esac

sha="$(resolve_gate_sha "$REPO_DIR")"
remote_url="$(git -C "$REPO_DIR" config --get remote.origin.url)"
[[ -n "$remote_url" ]] || fail "no remote.origin.url"
case "$remote_url$BOX_REPO" in *"'"*) fail "a single quote in the remote URL or box path would break the remote command" ;; esac

# One PowerShell line (the box's ssh shell); `;` separates, each step fails
# the whole run. Whether the SHA is pushed is decided ON THE BOX from a fresh
# fetch of every branch (Codex: local remote-tracking refs can be stale both
# ways), not from this clone's view of the remote.
remote="\$ErrorActionPreference='Stop'"
remote+="; if (-not (Test-Path '$BOX_REPO')) { git clone -q '$remote_url' '$BOX_REPO' }"
remote+="; git -C '$BOX_REPO' fetch -q --prune origin '+refs/heads/*:refs/remotes/origin/*'; if (\$LASTEXITCODE -ne 0) { exit 1 }"
remote+="; if (-not (git -C '$BOX_REPO' branch -r --contains $sha)) { Write-Error 'commit $sha is on no remote branch: push first, the box gates the pushed commit'; exit 1 }"
remote+="; git -C '$BOX_REPO' checkout -q --detach $sha; if (\$LASTEXITCODE -ne 0) { exit 1 }"
remote+="; Set-Location '$BOX_REPO/windows'"
remote+="; cargo clippy -p taigi-windows-settings -p taigi-windows-platform --all-targets -- -D warnings; if (\$LASTEXITCODE -ne 0) { exit 1 }"
remote+="; cargo test -p taigi-windows-settings -p taigi-windows-platform; if (\$LASTEXITCODE -ne 0) { exit 1 }"
# The settings exe only links here: its UI is WinUI 3, and the setup crate
# refuses every target but MSVC. This replaces the gnu `check-exe` that
# went with eframe at the W17-C cutover. Dev profile: the gate proves the
# MSVC link, and a fresh checkout would rebuild the whole windows-rs graph in
# release (5.5 min measured 2026-09-06 vs ~10 s dev); release-app.sh builds
# the shipped profile.
remote+="; cargo build -p taigi-windows-settings; if (\$LASTEXITCODE -ne 0) { exit 1 }"
# The 32-bit text service the installer ships beside the 64-bit one: the
# macOS gnu check is 64-bit only, so a pointer-width mistake (an `as isize`
# where the i686 `windows` crate takes an `i32`) surfaces only here. A type
# check is enough to catch that; release-app.sh proves the i686 link.
remote+="; cargo check -p taigi-windows-tsf --target i686-pc-windows-msvc; if (\$LASTEXITCODE -ne 0) { exit 1 }"
# The Arm64 text service the installer ships (docs/architecture/
# windows-arm64-roadmap.md). The macOS `check-msvc` leaves the TSF crate out
# (windows/Makefile MSVC_CHECKABLE), and on Windows its graph pulls in the
# engine's bundled SQLite (rusqlite via dispatch → userdata), which needs the
# MSVC ARM64 C compiler: the box has it (windows-release.md machine setup).
# release-app.sh proves the Arm64 link.
remote+="; cargo check -p taigi-windows-tsf --target aarch64-pc-windows-msvc; exit \$LASTEXITCODE"

echo "==> check-box: $BOX ($BOX_REPO) @ $sha"
ssh -o BatchMode=yes -o ConnectTimeout=15 "$BOX" "$remote" ||
    fail "the Windows box gate failed (or the box is unreachable) — see the output above"
echo "check-box OK: $sha"
