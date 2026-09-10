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

# -q must come first: it is what stops curl reading ~/.curlrc, which could
# otherwise switch on the netrc that --netrc-file disables here. Together they
# guarantee these requests carry no credentials, which is the entire point —
# an authenticated check cannot tell a public URL from a private one.
anonymous_curl() {
    curl -q --netrc-file /dev/null --silent --show-error --location "$@"
}

anonymous_status() {
    anonymous_curl --output /dev/null --write-out '%{http_code}' "$@" || true
}

# Download to a file, failing on an HTTP error rather than saving the error page.
# Without --fail curl exits 0 on a 404 or a 503, so a caller that hashes what it
# got would compare the digest of GitHub's error page against the asset's and
# report a corrupt upload for what is really "not served yet, try again".
anonymous_download() {
    anonymous_curl --fail --output "$2" "$1"
}

# Create or replace one file in the website repository.
commit_site_file() {
    local path="$1" message="$2" content="$3"

    python3 -c 'import json,sys; json.loads(sys.stdin.read())' <<< "$content" ||
        fail "generated $path is not valid JSON: $content"

    # Assigned on its own line: a command substitution inside a `local`
    # declaration reports `local`'s own exit status, which would hide a failure
    # here from `set -e`.
    local encoded_content
    encoded_content="$(printf '%s' "$content" | base64 | tr -d '\n')"

    local api="repos/$SITE_REPOSITORY/contents/$path"
    local -a arguments=(
        -X PUT
        -f "message=$message"
        -f "content=$encoded_content"
    )
    # Updating an existing file requires the blob it replaces; creating one must
    # not send a sha at all. Only a genuine 404 means "creating" — a rate limit
    # or a permission error read as one would turn into a confusing failure from
    # the PUT below instead of the reason it actually stopped.
    local read_result
    if read_result="$(gh api "$api" --jq .sha 2>&1)"; then
        arguments+=(-f "sha=$read_result")
    elif [[ "$read_result" != *"HTTP 404"* ]]; then
        fail "cannot read $path in $SITE_REPOSITORY: $read_result"
    fi
    gh api "$api" "${arguments[@]}" --jq '.commit.html_url'
}

# Announce one platform's finished download: commit the site data the website
# renders its manifest from, wait until the live manifest serves it, then print
# where everything ended up.
#
#   announce_desktop_platform <label> <site path> <manifest url> <json> <field=value>...
#
# The site data is written only after the download has been proven reachable —
# a manifest published before its download points every installed copy at a 404,
# and the developer's own browser, being authenticated, cannot see it happen.
announce_desktop_platform() {
    local label="$1" site_path="$2" manifest_url="$3" site_json="$4"
    shift 4

    echo "==> Publishing the release data"
    commit_site_file "$site_path" "chore: $label release -> $SHORT_VERSION" "$site_json"
    wait_for_manifest "$manifest_url" "$@"

    echo ""
    echo "✓ published $label $SHORT_VERSION"
    echo "  release   $RELEASE_PAGE_URL"
    echo "  download  $ASSET_URL"
    echo "  manifest  $manifest_url"
    echo "  website   https://taigikeyboard.tw/#download"
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
