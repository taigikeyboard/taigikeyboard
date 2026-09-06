#!/usr/bin/env bash
# Shared by macos/scripts/publish-release.sh and windows/scripts/publish-release.sh:
# anonymous download probes plus the website-repository file commit. Sourced, not
# executed. The caller must define `fail` and `PUBLISH_REPOSITORY` before calling.

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

    local api="repos/$PUBLISH_REPOSITORY/contents/$path"
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
        fail "cannot read $path in $PUBLISH_REPOSITORY: $read_result"
    fi
    gh api "$api" "${arguments[@]}" --jq '.commit.html_url'
}
