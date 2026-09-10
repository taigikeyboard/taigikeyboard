#!/usr/bin/env bash
# The website side of a release, shared by macos/scripts/publish-release.sh and
# windows/scripts/publish-release.sh: anonymous fetches, the website repository
# file commit, and the announcement built on both. Sourced, not executed; the
# caller must define `fail`.
#
# The website is not where the releases live — those are in this repository
# (`scripts/lib/desktop-release.sh`). The two were one repository until
# 2026-09-09, which is why each name says which it is: a single name for both
# is how a change of release host silently starts writing `_data/` into the app
# repository.
SITE_REPOSITORY="taigikeyboard/taigikeyboard.github.io"
# What a release writes over there, and what the site renders from it. The
# manifest URLs are compiled into every shipped copy, so they never move.
MACOS_SITE_PATH="_data/macos_release.json"
WINDOWS_SITE_PATH="_data/windows_release.json"
MACOS_MANIFEST_URL="https://taigikeyboard.tw/appcast/macos.json"
WINDOWS_MANIFEST_URL="https://taigikeyboard.tw/appcast/windows.json"

# -q must come first: it is what stops curl reading ~/.curlrc, which could
# otherwise switch on the netrc that --netrc-file disables here. Together they
# guarantee these requests carry no credentials, which is the entire point —
# an authenticated check cannot tell a public URL from a private one.
anonymous_curl() {
    curl -q --netrc-file /dev/null --silent --show-error --location "$@"
}

# Download to a file, failing on an HTTP error rather than saving the error page.
# Without --fail curl exits 0 on a 404 or a 503, so a caller that hashes what it
# got would compare the digest of GitHub's error page against the asset's and
# report a corrupt upload for what is really "not served yet, try again".
anonymous_download() {
    anonymous_curl --fail --output "$2" "$1"
}

# Create or replace one or more files in the website repository, in ONE commit.
#
#   commit_site_files <message> <path> <json content> [<path> <json content>...]
#
# One commit, because each Pages run deploys the tree of its own commit: two
# commits seconds apart race, and the run for the earlier one finishing last
# serves the earlier tree. That is not a hypothetical — on 2026-08-28 it served
# a manifest one release behind for a day. A desktop release announces two
# platforms, so the same hazard is one commit away unless they go together.
#
# Built through the git data API rather than the contents API, which can only
# write one file per commit.
commit_site_files() {
    local message="$1"
    shift

    # The tree API takes a file's content inline and writes the blob itself, so
    # a file is one entry rather than an upload plus a reference.
    local -a tree_entries=()
    local path content
    while [[ $# -gt 0 ]]; do
        path="$1"
        content="$2"
        shift 2

        tree_entries+=("$(python3 -c '
import json, sys
path, content = sys.argv[1], sys.argv[2]
json.loads(content)  # a manifest source that is not JSON is a bug, not a file to commit
print(json.dumps({"path": path, "mode": "100644", "type": "blob", "content": content}))
' "$path" "$content" 2> /dev/null)") || fail "generated $path is not valid JSON: $content"
    done

    local branch base_commit base_tree tree_sha commit_sha
    branch="$(gh api "repos/$SITE_REPOSITORY" --jq .default_branch)" ||
        fail "cannot read $SITE_REPOSITORY — is gh authenticated for it?"
    base_commit="$(gh api "repos/$SITE_REPOSITORY/git/ref/heads/$branch" --jq .object.sha)"
    base_tree="$(gh api "repos/$SITE_REPOSITORY/git/commits/$base_commit" --jq .tree.sha)"

    # A tree built on the current one: every path not named here keeps the blob
    # it already has.
    tree_sha="$(printf '{"base_tree":"%s","tree":[%s]}' \
        "$base_tree" "$(
            IFS=,
            echo "${tree_entries[*]}"
        )" |
        gh api "repos/$SITE_REPOSITORY/git/trees" --input - --jq .sha)" ||
        fail "cannot build the tree for $SITE_REPOSITORY"
    commit_sha="$(printf '{"message":%s,"tree":"%s","parents":["%s"]}' \
        "$(printf '%s' "$message" | python3 -c 'import json,sys; print(json.dumps(sys.stdin.read()))')" \
        "$tree_sha" "$base_commit" |
        gh api "repos/$SITE_REPOSITORY/git/commits" --input - --jq .sha)" ||
        fail "cannot commit to $SITE_REPOSITORY"
    # Not forced: a website commit landing between the read and here means
    # somebody else is mid-change, and overwriting it is never the right answer.
    gh api "repos/$SITE_REPOSITORY/git/refs/heads/$branch" -X PATCH -f "sha=$commit_sha" --jq .object.sha ||
        fail "cannot move $SITE_REPOSITORY's $branch — did the site change while this ran? Re-run to rebuild on top of it"
}

# Wait until the live manifest serves exactly these fields.
#
#   wait_for_manifest <url> <field>=<expected>...
#
# GitHub Pages has to build and the CDN has to expire what it holds. Polling the
# real URL is the only thing that proves the release is actually announced;
# everything before it only proves the site data was committed. It also proves
# the site rendered the manifest from what was committed, which is the one step
# of the announcement the publish scripts do not perform themselves.
#
# Every published field is checked, not just the version. `packageURL` — and on
# Windows `packageSHA256` — are what let an installed copy fetch and admit the
# installer itself, and a manifest missing one still reads as a perfectly valid
# update: the user is sent to a browser instead. A render that dropped one would
# satisfy a version-only poll and quietly cost every install the in-app
# download, with no later signal that it happened.
wait_for_manifest() {
    local manifest_url="$1"
    shift
    # One list, split here, so a caller cannot pair a field with the wrong value.
    local -a fields=() values=()
    local pair
    for pair in "$@"; do
        fields+=("${pair%%=*}")
        values+=("${pair#*=}")
    done
    local expected="${values[*]}"

    echo "==> Waiting for $manifest_url to serve $expected"
    local attempt live
    for attempt in $(seq 1 30); do
        live="$(anonymous_curl --header 'Cache-Control: no-cache' "$manifest_url" 2>/dev/null |
            python3 -c 'import json,sys
try:
    manifest = json.load(sys.stdin)
    print(" ".join((manifest.get(field) or "-") for field in sys.argv[1:]))
except Exception:
    print("-")' "${fields[@]}" || true)"
        [[ "$live" == "$expected" ]] && return
        [[ $attempt -eq 30 ]] &&
            fail "manifest still serving '$live' after 5 minutes, wanted '$expected' — check the Pages deployment"
        sleep 10
    done
}
