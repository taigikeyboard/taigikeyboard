#!/usr/bin/env bash
# The Windows-only crates' real gate (roadmap W13 amendment, W17): clippy +
# tests on the box, for the MSVC target that ships. The macOS gnu check only
# type-checks them — `windows-reactor-setup` refuses the gnu target,
# `windows-reactor` has no host build at all, and the platform crate's key
# translation is the Win32 keyboard API itself (so its tests cannot run on
# the macOS host either).
#
# Gates the PUSHED commit (commit-first ordering): the box keeps its own gate
# clone, fetches HEAD's SHA, checks it out detached and runs cargo there. An
# unreachable box or a failing command is a gate FAILURE — never a skip.
#
#   TAIGI_WINDOWS_BOX       ssh host (default: win)
#   TAIGI_WINDOWS_BOX_REPO  the gate clone on the box
#                           (default: C:/Workspace/taigikeyboard-gate)

set -euo pipefail

fail() {
    echo "check-box: $*" >&2
    exit 1
}

BOX="${TAIGI_WINDOWS_BOX:-win}"
BOX_REPO="${TAIGI_WINDOWS_BOX_REPO:-C:/Workspace/taigikeyboard-gate}"
REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

sha="$(git -C "$REPO_DIR" rev-parse HEAD)"
remote_url="$(git -C "$REPO_DIR" config --get remote.origin.url)"
[[ -n "$remote_url" ]] || fail "no remote.origin.url"
case "$remote_url$BOX_REPO" in *"'"*) fail "a single quote in the remote URL or box path would break the remote command" ;; esac
if ! git -C "$REPO_DIR" diff --quiet || ! git -C "$REPO_DIR" diff --cached --quiet; then
    echo "check-box: warning — the working tree is dirty; the box checks $sha, not this tree" >&2
fi

# One PowerShell line (the box's ssh shell); `;` separates, each step fails
# the whole run. Whether the SHA is pushed is decided ON THE BOX from a fresh
# fetch of every branch (Codex: local remote-tracking refs can be stale both
# ways), not from this clone's view of the remote.
remote="\$ErrorActionPreference='Stop'"
remote+="; if (-not (Test-Path '$BOX_REPO')) { git clone -q '$remote_url' '$BOX_REPO' }"
remote+="; git -C '$BOX_REPO' fetch -q --prune origin '+refs/heads/*:refs/remotes/origin/*'; if (\$LASTEXITCODE -ne 0) { exit 1 }"
remote+="; if (-not (git -C '$BOX_REPO' branch -r --contains $sha)) { Write-Error 'HEAD $sha is on no remote branch: push first, the box gates the pushed commit'; exit 1 }"
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
remote+="; cargo build -p taigi-windows-settings; exit \$LASTEXITCODE"

echo "==> check-box: $BOX ($BOX_REPO) @ $sha"
ssh -o BatchMode=yes -o ConnectTimeout=15 "$BOX" "$remote" ||
    fail "the Windows box gate failed (or the box is unreachable) — see the output above"
echo "check-box OK: $sha"
