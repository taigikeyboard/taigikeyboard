#!/usr/bin/env bash
# The GitHub release a desktop version is staged into, shared by
# macos/scripts/publish-release.sh and windows/scripts/publish-release.sh.
# Sourced, not executed, AFTER the platform identity library (`fail`,
# `REPOSITORY_DIR`, `SHORT_VERSION`). The release scripts source it for
# `release_sha256` alone. Why the release lives here rather than on the website
# repository: `docs/architecture/macos-release.md` § Publishing the package.
#
# One desktop version is ONE release, tagged `desktop-<version>`, holding both
# platforms' installers. The two are built on two machines at two times — the
# package on a Mac once Apple has notarized it, the installer on a Windows box —
# so whichever runs first creates the release and the other attaches its asset.
#
# That release is a DRAFT until a person publishes it. A draft has no public
# asset URL and no git tag, so nothing here reaches a user: the maintainer
# downloads the artifacts from the draft, tests them, and publishes by hand.
# `scripts/announce-release.sh` is the other half — it runs after that manual
# publish and is what tells the website and every installed copy.
RELEASE_REPOSITORY="taigikeyboard/taigikeyboard"
# The platform identity libraries set SHORT_VERSION from their own project file;
# `scripts/announce-release.sh` has no platform library, and PlistBuddy does not
# exist on the Windows box, so the fallback reads `windows/Cargo.toml` — the
# other file `make version-desktop` writes, held equal to the plist by
# `release_notes.py check-versions --train desktop`.
SHORT_VERSION="${SHORT_VERSION:-$(awk '
    /^\[workspace\.package\]/ { inside = 1; next }
    inside && /^\[/ { exit }
    inside && /^version *=/ { gsub(/[" ]/, "", $3); print $3; exit }
' "$REPOSITORY_DIR/windows/Cargo.toml" | tr -d '\r')}"
DESKTOP_TAG="desktop-$SHORT_VERSION"
# Valid only once the release is published: a draft has no tag for this URL to
# name, which is why staging prints the API's own draft URL instead.
RELEASE_PAGE_URL="https://github.com/$RELEASE_REPOSITORY/releases/tag/$DESKTOP_TAG"
# Both platforms on one page, because both are on one release. The title says
# the version; which platform an asset is for is what the asset is named.
RELEASE_TITLE="Taigi Keyboard Desktop $SHORT_VERSION"

# What each platform puts on the release. One name each, read by every script
# that writes the file (`release-app.sh`), stages it (`publish-release.sh`) or
# looks for it (`announce-release.sh`), so the three cannot drift — a name the
# announcement does not recognise reads as "that platform was never staged"
# rather than as an error. A throwaway build appends its qualifier to the stem.
MACOS_ASSET="TaigiKeyboard-$SHORT_VERSION.pkg"
WINDOWS_ASSET="TaigiKeyboard-$SHORT_VERSION.exe"

# A file's SHA-256 as lowercase hex, on either host: macOS ships `shasum`, Git
# Bash on the Windows box ships `sha256sum`. It is what the Windows manifest
# publishes and what every installed copy hashes a download to
# (`taigi-windows-update::verify::file_sha256`).
release_sha256() {
    if command -v sha256sum > /dev/null; then
        sha256sum "$1" | cut -d' ' -f1
    else
        shasum -a 256 "$1" | cut -d' ' -f1
    fi
}

# Everything a publish needs to be true before it builds any evidence about the
# package, so a dirty tree fails in a second rather than after Gatekeeper
# assessment or a signtool round trip. Sets, for the rest of the publish:
#
#   RELEASE_TEMP_DIR     scratch, removed on exit — put platform temporaries
#                        here rather than setting a second EXIT trap, which
#                        would replace this one
#   DESKTOP_SOURCE_COMMIT  the commit the tag names
#   DESKTOP_NOTES_FILE     the release body, read out of that commit
desktop_release_preflight() {
    desktop_release_scratch_and_tools
    # The update manifest only accepts dotted integers — its checker rejects
    # anything with a suffix as malformed, and does so silently, so a
    # `3.6.5-beta` here would publish a release every installed copy quietly
    # refuses to read.
    [[ "$SHORT_VERSION" =~ ^[0-9]+(\.[0-9]+)*$ ]] ||
        fail "version '$SHORT_VERSION' is not dotted integers — the update manifest rejects suffixes"

    _resolve_source_commit
    _read_release_notes
}

# The scratch directory both halves put temporaries in, and the tool both need.
# One EXIT trap for the whole run: a second one anywhere would replace it.
desktop_release_scratch_and_tools() {
    RELEASE_TEMP_DIR="$(mktemp -d)"
    trap 'rm -rf "$RELEASE_TEMP_DIR"' EXIT

    command -v gh > /dev/null || fail "the GitHub CLI (gh) is not installed"
    gh auth status > /dev/null 2>&1 || fail "gh is not authenticated — run 'gh auth login'"
}

# A release created here creates a real git tag on a real commit, which the
# publish to the website repository never did — so the checkout has to be one
# that everybody else can see, and has to be exactly what was built. It need not
# be `main`'s tip: `main` may move between the two platforms' publishes, and
# requiring the tip would strand the second machine.
_resolve_source_commit() {
    DESKTOP_SOURCE_COMMIT="$(git -C "$REPOSITORY_DIR" rev-parse HEAD)" ||
        fail "$REPOSITORY_DIR is not a git checkout"
    [[ -z "$(git -C "$REPOSITORY_DIR" status --porcelain --ignore-submodules=none)" ]] ||
        fail "the working tree is dirty — a release tag must name a commit that is exactly what was built"
    git -C "$REPOSITORY_DIR" fetch --quiet origin main ||
        fail "cannot reach origin — the tag would name a commit nobody else has"
    git -C "$REPOSITORY_DIR" merge-base --is-ancestor "$DESKTOP_SOURCE_COMMIT" FETCH_HEAD ||
        fail "HEAD (${DESKTOP_SOURCE_COMMIT:0:7}) is not on origin/main — push it before publishing"
}

# The whole `changelog/desktop-v<version>.md`, both platforms' sections, because
# both platforms share this page. Read from the commit rather than from the
# working tree so the body is a function of the tag alone: a changelog that was
# never committed would otherwise publish from the first machine and be missing
# on the second. There is no fallback note — a release with no changelog is a
# release nobody can read, and its absence means the prepare step was skipped.
_read_release_notes() {
    DESKTOP_NOTES_FILE="$RELEASE_TEMP_DIR/release-notes.md"
    git -C "$REPOSITORY_DIR" show "$DESKTOP_SOURCE_COMMIT:changelog/desktop-v$SHORT_VERSION.md" \
        > "$DESKTOP_NOTES_FILE" 2> /dev/null ||
        fail "commit ${DESKTOP_SOURCE_COMMIT:0:7} has no changelog/desktop-v$SHORT_VERSION.md — prepare and commit the release notes before publishing"
    [[ -s "$DESKTOP_NOTES_FILE" ]] ||
        fail "changelog/desktop-v$SHORT_VERSION.md is empty in ${DESKTOP_SOURCE_COMMIT:0:7}"
}

# Put one platform's installer on this version's DRAFT release, creating the
# draft if this is the first platform to run, and prove the bytes GitHub now
# holds are the bytes that were built.
#
# Nothing here is public: a draft is visible only to people who can write this
# repository, and it has no `releases/download/<tag>/<name>` URL for anyone to
# find. The read-back is therefore authenticated — the anonymous proof is
# `scripts/announce-release.sh`'s job, after the draft is published.
#
# Alongside the installer goes `<installer>.sha256`, the digest of what was
# staged. It is what the announcement holds the published bytes against, so the
# thing a user downloads is checked against a digest recorded before anyone
# tested it — and on Windows, where releases are unsigned, it is also what a
# user can check a manual download with.
stage_desktop_asset() {
    local asset_path="$1"
    local asset_name local_sha256 receipt
    asset_name="$(basename "$asset_path")"
    local_sha256="$(release_sha256 "$asset_path")"
    receipt="$RELEASE_TEMP_DIR/$asset_name.sha256"
    printf '%s  %s\n' "$local_sha256" "$asset_name" > "$receipt"

    _require_release_names_commit

    # Called directly, not in a command substitution: `fail` inside one exits
    # only the subshell, so a network error would have gone on to create a
    # second release. "No release" and "a release holding nothing yet" are also
    # different states — the second must not take the create path.
    local release_exists=true
    _read_staged_asset_names || release_exists=false

    if [[ "$release_exists" == false ]]; then
        echo "==> Creating draft release $DESKTOP_TAG in $RELEASE_REPOSITORY"
        # --draft: no tag, no public download, nothing announced. --target is
        # what the tag will name when a person publishes it — a full SHA, never
        # a branch, so the tag cannot end up on whatever main has moved to by
        # then.
        gh release create "$DESKTOP_TAG" \
            --repo "$RELEASE_REPOSITORY" \
            --draft \
            --target "$DESKTOP_SOURCE_COMMIT" \
            --title "$RELEASE_TITLE" \
            --notes-file "$DESKTOP_NOTES_FILE" \
            "$asset_path" "$receipt"
    elif grep -qxF "$asset_name" <<< "$STAGED_ASSET_NAMES"; then
        echo "==> $asset_name is already on $DESKTOP_TAG — verifying it"
        # The receipt may be missing if a previous run died between the two
        # uploads; upload it only when it is not there, so a staged digest is
        # never quietly rewritten.
        grep -qxF "$asset_name.sha256" <<< "$STAGED_ASSET_NAMES" ||
            gh release upload "$DESKTOP_TAG" "$receipt" --repo "$RELEASE_REPOSITORY"
    else
        echo "==> Draft $DESKTOP_TAG exists — attaching $asset_name to it"
        # No --clobber anywhere in this flow: it deletes before it uploads, so a
        # failure part-way leaves nothing where an asset used to be.
        gh release upload "$DESKTOP_TAG" "$asset_path" "$receipt" --repo "$RELEASE_REPOSITORY"
    fi

    _verify_staged_asset "$asset_name" "$local_sha256"
}

# Sets STAGED_ASSET_NAMES to the names already on the draft, one per line, and
# returns non-zero when there is no release yet. A failed `gh release view`
# means "no such release" only when GitHub said so: a rate limit, an expired
# token or a network fault read as one would take the create path and make a
# second release, reporting the wrong problem.
_read_staged_asset_names() {
    local result
    if result="$(gh release view "$DESKTOP_TAG" --repo "$RELEASE_REPOSITORY" \
        --json assets --jq '.assets[].name' 2>&1)"; then
        STAGED_ASSET_NAMES="$result"
        return 0
    fi
    _require_absent "$result" "release $DESKTOP_TAG in $RELEASE_REPOSITORY"
    STAGED_ASSET_NAMES=""
    return 1
}

# GitHub said "not there", rather than "I could not tell you". Anything else —
# a rate limit, an expired token, a network fault — must stop the run rather
# than read as permission to create, publish or overwrite.
_require_absent() {
    local result="$1" what="$2"
    [[ "$result" == *"release not found"* || "$result" == *"HTTP 404"* || "$result" == *"Not Found"* ]] ||
        fail "cannot read $what: $result"
}

# Whatever already exists for this version has to name the commit being staged,
# so the second machine cannot hang a differently-built installer off the first
# machine's release. Two things can exist, and they are checked differently:
#
#   a git tag   — from a hand-push, or from a release published earlier. It is
#                 authoritative and immovable: `gh release create` silently
#                 ignores --target once the tag is there.
#   a draft     — no tag yet, only `targetCommitish`, which GitHub resolves when
#                 the draft is published. A branch name there would tag whatever
#                 that branch points at on publish day, so only a full SHA equal
#                 to this commit is accepted.
_require_release_names_commit() {
    _require_tag_names_commit
    _require_draft_targets_commit
}

_require_tag_names_commit() {
    local tagged
    tagged="$(desktop_tag_commit)"
    [[ -z "$tagged" || "$tagged" == "$DESKTOP_SOURCE_COMMIT" ]] ||
        fail "$DESKTOP_TAG names commit ${tagged:0:7}, but this checkout is ${DESKTOP_SOURCE_COMMIT:0:7} — check out the commit the other platform released from, or cut a new version"
}

# The commit this version's tag names, or nothing when there is no tag yet.
# Both halves ask: staging, to refuse a build that does not match a tag pushed
# by hand; the announcement, to refuse a tag that does not match what the
# release recorded.
desktop_tag_commit() {
    local reference object_sha object_type
    if ! reference="$(gh api "repos/$RELEASE_REPOSITORY/git/ref/tags/$DESKTOP_TAG" \
        --jq '.object.sha + " " + .object.type' 2>&1)"; then
        _require_absent "$reference" "the tag $DESKTOP_TAG in $RELEASE_REPOSITORY"
        return 0
    fi

    read -r object_sha object_type <<< "$reference"
    # An annotated tag points at a tag object, which points at the commit.
    if [[ "$object_type" == "tag" ]]; then
        object_sha="$(gh api "repos/$RELEASE_REPOSITORY/git/tags/$object_sha" --jq .object.sha)" ||
            fail "cannot dereference the annotated tag $DESKTOP_TAG"
    fi
    printf '%s\n' "$object_sha"
}

_require_draft_targets_commit() {
    local state target is_draft
    if ! state="$(gh release view "$DESKTOP_TAG" --repo "$RELEASE_REPOSITORY" \
        --json isDraft,targetCommitish --jq '(.isDraft|tostring) + " " + .targetCommitish' 2>&1)"; then
        # A read that merely failed must not read as "no release yet" — the
        # upload below would then add an asset to a release that may already be
        # published, which is the one thing this flow exists to prevent.
        _require_absent "$state" "release $DESKTOP_TAG in $RELEASE_REPOSITORY"
        return 0
    fi
    read -r is_draft target <<< "$state"

    if [[ "$is_draft" != true ]]; then
        fail "$DESKTOP_TAG is already published — an asset added now would be public immediately. Put it back in draft (gh release edit $DESKTOP_TAG --repo $RELEASE_REPOSITORY --draft=true) to keep testing, or cut a new version"
    fi
    [[ "$target" == "$DESKTOP_SOURCE_COMMIT" ]] ||
        fail "the draft $DESKTOP_TAG will tag '$target', but this checkout is ${DESKTOP_SOURCE_COMMIT:0:7} — it was created from a different commit (or from a branch, which would tag whatever that branch points at on publish day); delete the draft and stage again, or cut a new version"
}

# Authenticated, because a draft has no anonymous URL to read — but still a
# whole download, because "GitHub holds these bytes" and "the upload landed
# whole" are different facts. The maintainer is about to test what this proves.
_verify_staged_asset() {
    local asset_name="$1" local_sha256="$2"
    local download_dir="$RELEASE_TEMP_DIR/staged"
    local staged_sha256

    echo "==> Reading the staged asset back"
    rm -rf "$download_dir"
    mkdir -p "$download_dir"
    gh release download "$DESKTOP_TAG" \
        --repo "$RELEASE_REPOSITORY" \
        --pattern "$asset_name" \
        --dir "$download_dir" ||
        fail "cannot read $asset_name back from the draft $DESKTOP_TAG"
    staged_sha256="$(release_sha256 "$download_dir/$asset_name")"
    [[ "$staged_sha256" == "$local_sha256" ]] ||
        fail "$DESKTOP_TAG holds $asset_name as $staged_sha256, but this build is $local_sha256 — either the upload did not land whole, or that name was staged from a different build; a staged asset is never replaced, so delete the draft and stage again"
    echo "  sha256 $staged_sha256"
}

# What is left to do, printed by each platform script when its staging is done.
# The two manual steps between here and a user seeing anything are the point of
# the draft: the maintainer installs what was staged, and only then publishes.
desktop_draft_summary() {
    local platform="$1"
    # The draft's own URL, from the API: a draft has no tag, so it is not the
    # `releases/tag/<tag>` address the announcement uses.
    local draft_url
    draft_url="$(gh release view "$DESKTOP_TAG" --repo "$RELEASE_REPOSITORY" --json url --jq .url)"

    echo ""
    echo "✓ staged $platform $SHORT_VERSION on the draft $DESKTOP_TAG"
    echo "  draft     $draft_url"
    echo "  commit    ${DESKTOP_SOURCE_COMMIT:0:7} (what publishing will tag)"
    echo ""
    echo "  Nothing is public yet. To test what was staged:"
    echo "    gh release download $DESKTOP_TAG --repo $RELEASE_REPOSITORY --dir ~/Downloads"
    echo ""
    echo "  When both platforms are staged and both have been tested, publish and announce:"
    echo "    gh release edit $DESKTOP_TAG --repo $RELEASE_REPOSITORY --draft=false"
    echo "    make desktop-announce"
}
