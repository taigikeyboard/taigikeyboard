#!/usr/bin/env bash
# The GitHub release a desktop version is published as, shared by
# macos/scripts/publish-release.sh and windows/scripts/publish-release.sh.
# Sourced, not executed, AFTER the platform identity library (`fail`,
# `REPOSITORY_DIR`, `SHORT_VERSION`); a publish additionally needs
# `release-site.sh` sourced for the anonymous fetches. The release scripts
# source it for `release_sha256` alone. Why the release lives here rather than
# on the website repository: `docs/architecture/macos-release.md` § Publishing
# the package.
#
# One desktop version is ONE release, tagged `desktop-<version>`, holding both
# platforms' installers. The two are built on two machines at two times — the
# package on a Mac once Apple has notarized it, the installer on a Windows box —
# so whichever publishes first creates the release and the other attaches its
# asset to it. Neither waits for the other: each platform's own update manifest
# moves as soon as its own asset is proven downloadable.

RELEASE_REPOSITORY="taigikeyboard/taigikeyboard"
DESKTOP_TAG="desktop-$SHORT_VERSION"
RELEASE_PAGE_URL="https://github.com/$RELEASE_REPOSITORY/releases/tag/$DESKTOP_TAG"
# Both platforms on one page, because both are on one release. The title says
# the version; which platform an asset is for is what the asset is named.
RELEASE_TITLE="Taigi Keyboard Desktop $SHORT_VERSION"

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
    RELEASE_TEMP_DIR="$(mktemp -d)"
    trap 'rm -rf "$RELEASE_TEMP_DIR"' EXIT

    command -v gh > /dev/null || fail "the GitHub CLI (gh) is not installed"
    gh auth status > /dev/null 2>&1 || fail "gh is not authenticated — run 'gh auth login'"
    # The update manifest only accepts dotted integers — its checker rejects
    # anything with a suffix as malformed, and does so silently, so a
    # `3.6.5-beta` here would publish a release every installed copy quietly
    # refuses to read.
    [[ "$SHORT_VERSION" =~ ^[0-9]+(\.[0-9]+)*$ ]] ||
        fail "version '$SHORT_VERSION' is not dotted integers — the update manifest rejects suffixes"

    _resolve_source_commit
    _read_release_notes
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

# Create the release or attach to it, then prove the asset is downloadable.
# Runs after `desktop_release_preflight`, and sets ASSET_URL and
# PUBLISHED_SHA256 — what the caller announces to the website — as globals,
# because its own stdout is the operator's log.
publish_desktop_asset() {
    local asset_path="$1"
    local asset_name local_sha256 published_assets
    asset_name="$(basename "$asset_path")"
    local_sha256="$(release_sha256 "$asset_path")"
    ASSET_URL="https://github.com/$RELEASE_REPOSITORY/releases/download/$DESKTOP_TAG/$asset_name"

    # Before either branch: a tag can exist without a release — pushed by hand to
    # start the Windows provenance build, or left by a half-finished publish —
    # and `gh release create` silently ignores --target for a tag that is already
    # there. Checking only on the attach path would let a create publish this
    # build under a tag naming a different commit.
    _require_tag_names_commit

    if ! published_assets="$(_published_asset_names)"; then
        echo "==> Creating release $DESKTOP_TAG in $RELEASE_REPOSITORY"
        # --target is what makes the tag name this commit rather than main's
        # tip, which may already have moved past it.
        gh release create "$DESKTOP_TAG" \
            --repo "$RELEASE_REPOSITORY" \
            --target "$DESKTOP_SOURCE_COMMIT" \
            --title "$RELEASE_TITLE" \
            --notes-file "$DESKTOP_NOTES_FILE" \
            "$asset_path"
    elif grep -qxF "$asset_name" <<< "$published_assets"; then
        echo "==> $asset_name is already on $DESKTOP_TAG — verifying it"
    else
        echo "==> Release $DESKTOP_TAG exists — attaching $asset_name to it"
        # No --clobber: it deletes before it uploads, which takes the download
        # away for as long as the upload runs — or for good if it fails — while
        # every published manifest still points at it.
        gh release upload "$DESKTOP_TAG" "$asset_path" --repo "$RELEASE_REPOSITORY"
    fi

    _verify_published_asset "$asset_name" "$local_sha256"
}

# The names already on the release, or non-zero when there is no release yet.
# A failed `gh release view` means "no such release" only when GitHub said so:
# a rate limit, an expired token or a network fault read as one would take the
# create path and fail there, reporting the wrong problem.
_published_asset_names() {
    local result
    if result="$(gh release view "$DESKTOP_TAG" --repo "$RELEASE_REPOSITORY" \
        --json assets --jq '.assets[].name' 2>&1)"; then
        printf '%s\n' "$result"
        return 0
    fi
    [[ "$result" == *"release not found"* || "$result" == *"HTTP 404"* ]] ||
        fail "cannot read release $DESKTOP_TAG in $RELEASE_REPOSITORY: $result"
    return 1
}

# The tag is what both platforms' assets hang off, so the second machine must be
# at the same commit as the first — a package built elsewhere that happens to
# carry the same version would otherwise ship under a tag that does not describe
# it. The tag is never moved: a mismatch is a stop, not a fixup.
_require_tag_names_commit() {
    local reference object_sha object_type
    reference="$(gh api "repos/$RELEASE_REPOSITORY/git/ref/tags/$DESKTOP_TAG" \
        --jq '.object.sha + " " + .object.type' 2> /dev/null)" ||
        return 0 # No tag yet; the create below makes it name this commit.

    read -r object_sha object_type <<< "$reference"
    # An annotated tag points at a tag object, which points at the commit.
    if [[ "$object_type" == "tag" ]]; then
        object_sha="$(gh api "repos/$RELEASE_REPOSITORY/git/tags/$object_sha" --jq .object.sha)" ||
            fail "cannot dereference the annotated tag $DESKTOP_TAG"
    fi
    [[ "$object_sha" == "$DESKTOP_SOURCE_COMMIT" ]] ||
        fail "$DESKTOP_TAG names commit ${object_sha:0:7}, but this checkout is ${DESKTOP_SOURCE_COMMIT:0:7} — check out the commit the other platform released from, or cut a new version"
}

# Anonymously, because that is how every user and every installed copy reaches
# it — and whole, because the digest published next has to be the one this URL
# actually serves. Those are different facts the moment an upload truncates or
# the wrong build was handed to the script, and this is the last step that can
# catch it before somebody's update window downloads it. GitHub can take a
# moment to make a fresh asset reachable, so an unreachable one is retried; only
# bytes that arrived whole and hash differently are a hard stop.
_verify_published_asset() {
    local asset_name="$1" local_sha256="$2"
    local downloaded="$RELEASE_TEMP_DIR/published-$asset_name"
    local page_status attempt

    echo "==> Reading the release back without credentials"
    for attempt in 1 2 3 4 5; do
        page_status="$(anonymous_status "$RELEASE_PAGE_URL")"
        if [[ "$page_status" == "200" ]] && anonymous_download "$ASSET_URL" "$downloaded"; then
            PUBLISHED_SHA256="$(release_sha256 "$downloaded")"
            [[ "$PUBLISHED_SHA256" == "$local_sha256" ]] ||
                fail "$ASSET_URL serves $PUBLISHED_SHA256, but $asset_name here is $local_sha256 — either the upload did not land whole, or that name was published from a different build and a published asset is never replaced; cut a new version"
            echo "  page 200, sha256 $PUBLISHED_SHA256"
            return
        fi
        [[ $attempt -eq 5 ]] &&
            fail "$DESKTOP_TAG is not anonymously reachable (page $page_status, asset $ASSET_URL) — is $RELEASE_REPOSITORY public?"
        echo "  page $page_status, asset unreadable — retrying in 5s"
        sleep 5
    done
}
