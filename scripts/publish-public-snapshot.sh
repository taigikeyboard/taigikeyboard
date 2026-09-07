#!/usr/bin/env bash
# Publishes a filtered snapshot of this private repository to the public one.
#
# Only PUBLIC_PATHS ship. Documentation, AI working notes, the dictionary
# sources and build pipeline, and the mobile and macOS applications stay
# private. The public repository exists so that SignPath can sign a Windows
# installer built by a workflow inside it — code signing requires the signed
# artifact to be produced by the public repository's own CI.
#
# The snapshot carries no history from this repository: each publish replaces
# the public tree wholesale and lands as one commit.
#
# Markdown is stripped from the copied trees — the notes in them are written for
# development in the private repository — and replaced by the overlay in
# `scripts/public-overlay/`.
#
#   scripts/publish-public-snapshot.sh            # stage and diff, do not push
#   scripts/publish-public-snapshot.sh --publish  # stage, commit and push
set -euo pipefail

PRIVATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PUBLIC_REMOTE="git@github.com:taigikeyboard/taigikeyboard.git"
WORK_DIR="${PUBLIC_SNAPSHOT_WORK_DIR:-$HOME/Workspace/taigikeyboard-public}"
OVERLAY_DIR="$PRIVATE_DIR/scripts/public-overlay"

# Everything the Windows installer build reads. `windows/Cargo.toml:34-35` takes
# `engine/dispatch` and `engine/protos` as path dependencies, which pulls in the
# whole engine workspace, and `windows/scripts/lib/identity.sh:46-47` stages the
# dictionary and font payloads out of `ios/Resources/`. The dictionary sources
# and the pipeline that compiles them stay private; only the compiled artifacts
# ship.
PUBLIC_PATHS=(
    .gitattributes
    .gitignore
    .github/workflows/windows-build.yml
    LICENSE
    NOTICE
    SECURITY.md
    THIRD_PARTY_LICENSES.md
    engine
    ios/Resources/Dictionaries
    ios/Resources/Fonts
    windows
)

# Paths that must never reach the public tree even if PUBLIC_PATHS grows to
# cover them by accident. Checked against the staged tree, so a mistake fails
# the publish rather than leaking.
FORBIDDEN_PATTERNS=(
    '(^|/)\.claude/'
    '(^|/)AGENTS\.md$'
    '(^|/)CLAUDE\.md$'
    '^changelog/'
    '^dictionary/'
    '^docs/'
    '^knowledge/'
)

fail() {
    echo "publish-public-snapshot: $*" >&2
    exit 1
}

should_publish=false
case "${1:-}" in
    --publish) should_publish=true ;;
    "") ;;
    *) fail "unknown argument '$1' — expected --publish or nothing" ;;
esac

[[ -z "$(git -C "$PRIVATE_DIR" status --porcelain)" ]] ||
    fail "working tree is dirty; the snapshot must describe a committed state"

private_head="$(git -C "$PRIVATE_DIR" rev-parse --short HEAD)"
private_branch="$(git -C "$PRIVATE_DIR" rev-parse --abbrev-ref HEAD)"

if [[ -d "$WORK_DIR/.git" ]]; then
    git -C "$WORK_DIR" fetch --prune origin
    git -C "$WORK_DIR" checkout main 2>/dev/null || git -C "$WORK_DIR" checkout -b main
    git -C "$WORK_DIR" reset --hard origin/main 2>/dev/null || true
else
    git clone "$PUBLIC_REMOTE" "$WORK_DIR"
    git -C "$WORK_DIR" checkout -B main
fi

# Replace the tree wholesale so that a path dropped from PUBLIC_PATHS also
# disappears from the public repository.
find "$WORK_DIR" -mindepth 1 -maxdepth 1 ! -name .git -exec rm -rf {} +

git -C "$PRIVATE_DIR" archive --format=tar HEAD -- "${PUBLIC_PATHS[@]}" |
    tar -x -C "$WORK_DIR"

# Documentation-shaped files inside the kept trees are developer notes; the
# public repository gets the overlay README instead. `engine/README.md` is not
# documentation to the compiler — `engine/phonetics/src/lib.rs:2` pulls it in
# with `include_str!`, so dropping it fails the build.
find "$WORK_DIR" -path "$WORK_DIR/.git" -prune -o -name '*.md' -print |
    grep -vE "/(NOTICE|SECURITY|THIRD_PARTY_LICENSES)\.md$|^$WORK_DIR/engine/README\.md$" |
    xargs -r rm -f

# `NOTICE` and `THIRD_PARTY_LICENSES.md` in the overlay are edited copies of the
# private repository's own. They do not track edits to the originals — resync
# them by hand when a font or a dependency changes.
cp -R "$OVERLAY_DIR/." "$WORK_DIR/"

git -C "$WORK_DIR" add --all

staged="$(git -C "$WORK_DIR" diff --cached --name-only)"
for pattern in "${FORBIDDEN_PATTERNS[@]}"; do
    matched="$(printf '%s\n' "$staged" | grep -E "$pattern" || true)"
    [[ -z "$matched" ]] ||
        fail "staged tree matches forbidden pattern '$pattern':"$'\n'"$matched"
done

echo "--- staged snapshot ---"
git -C "$WORK_DIR" diff --cached --stat | tail -20
echo "files: $(printf '%s\n' "$staged" | grep -c .)"

if ! $should_publish; then
    echo "dry run — nothing pushed. Re-run with --publish to commit and push."
    exit 0
fi

if git -C "$WORK_DIR" diff --cached --quiet; then
    echo "no change against the published snapshot"
    exit 0
fi

git -C "$WORK_DIR" commit -m "Snapshot of ${private_branch} at ${private_head}

Published from the private development repository. It carries the Windows input method and
the engine behind it, and is the build source for the Windows installer.

Claude-Session: https://claude.ai/code/session_0156o3uCkgVBgbEY4UrdwinN"
git -C "$WORK_DIR" push origin main
echo "published: $(git -C "$WORK_DIR" rev-parse --short HEAD)"
