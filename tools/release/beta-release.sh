#!/usr/bin/env bash
# Offer an unpublished desktop draft to internal testers on Discord.
#
# Usage: beta-release.sh fetch <dir> <release id> <source sha>
#        beta-release.sh publish <dir> <source sha> <run id>
#        beta-release.sh announce <dir> <source sha> <run id>
#
# Run through `.github/actions/desktop-beta`: by `desktop-beta.yml`, which
# `stage-desktop.sh` dispatches once a draft is fully staged, and by the
# monthly train's `sign-and-beta` job. Both wait for the `release`
# environment's reviewer (roadmap D5). Three steps, each with its own secret:
#
#   fetch    GH_TOKEN = the workflow token (contents: write — GitHub lists a
#            draft only to push access). Downloads the installers of THE draft
#            the staging run made, by release id: a later staging run deletes
#            and re-creates the draft under the same tag and commit, and an
#            older beta run must not read the new, half-built one.
#   publish  The three R2_* credentials. Mirrors each installer to R2
#            `beta/<version>-<sha7>-<run id>/` (a fresh prefix per run: the
#            download host caches for 30 days, and a re-stage at the same
#            commit builds different bytes), lists the links in the job
#            summary, then deletes every older `beta/` prefix.
#   announce DISCORD_BETA_WEBHOOK only. Posts each installer's R2 link to the
#            internal Discord channel the webhook belongs to. The beta is not
#            on the website (USER 2026-11-14: internal Discord testing only).
#
# Nothing here touches the published releases: not `desktop/` in the bucket,
# not the website, not the appcasts.

set -euo pipefail

fail() {
    echo "error: $*" >&2
    exit 1
}

REPOSITORY_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

usage() {
    echo "usage: beta-release.sh fetch <dir> <release id> <source sha>" >&2
    echo "       beta-release.sh publish <dir> <source sha> <run id>" >&2
    echo "       beta-release.sh announce <dir> <source sha> <run id>" >&2
    exit 2
}

mode="${1:-}"
case "$mode:$#" in
    fetch:4 | publish:4 | announce:4) ;;
    *) usage ;;
esac

# SHORT_VERSION comes from the environment (the workflow input), never from the
# checkout: the run checks out main's tip, which may already carry a later bump.
[[ "${SHORT_VERSION:-}" =~ ^[0-9]+(\.[0-9]+)*$ ]] ||
    fail "SHORT_VERSION '${SHORT_VERSION:-}' is not dotted integers"

# shellcheck source=lib/release-site.sh
source "$REPOSITORY_DIR/tools/release/lib/release-site.sh"
# shellcheck source=lib/desktop-release.sh
source "$REPOSITORY_DIR/tools/release/lib/desktop-release.sh"

# This run's folder under beta/: publish writes it, announce links into it.
beta_run_name() { echo "$SHORT_VERSION-${1:0:7}-$2"; }

# The only names publish writes and prune deletes under beta/.
BETA_NAME_RE='^[0-9.]+-[0-9a-f]{7}-[0-9]+$'

# One "<name> <id>" line per asset of the release being fetched.
asset_listing=""
asset_id() { awk -v name="$1" '$1 == name { print $2 }' <<< "$asset_listing"; }
download_asset() {
    gh api -H "Accept: application/octet-stream" \
        "repos/$RELEASE_REPOSITORY/releases/assets/$1" > "$2" || fail "cannot download $2"
}

fetch() {
    local release_id="$1" source_sha="$2" state
    [[ "$release_id" =~ ^[0-9]+$ ]] || fail "release id '$release_id' is not a number"

    state="$(gh api "repos/$RELEASE_REPOSITORY/releases/$release_id" \
        --jq '"\(.draft) \(.target_commitish) \(.tag_name)"' 2>&1)" ||
        fail "cannot read release $release_id (a 404 means a later staging run replaced it, and its own beta run offers that one): $state"
    read -r is_draft target tag <<< "$state"
    [[ "$is_draft" == true ]] || fail "release $release_id is already published — it needs no beta"
    [[ "$target" == "$source_sha" ]] || fail "release $release_id targets $target, not $source_sha"
    [[ "$tag" == "$DESKTOP_TAG" ]] || fail "release $release_id is $tag, not $DESKTOP_TAG"

    # By asset id: `gh release download` resolves a draft by tag, which is not
    # unique among drafts.
    asset_listing="$(gh api "repos/$RELEASE_REPOSITORY/releases/$release_id/assets" --paginate \
        --jq '.[] | "\(.name) \(.id)"')" || fail "cannot list the assets of release $release_id"

    mkdir -p "$assets_dir"
    local asset id receipt_id found=0
    for asset in "${DESKTOP_ASSETS[@]}"; do
        id="$(asset_id "$asset")"
        [[ -n "$id" ]] || continue
        receipt_id="$(asset_id "$asset.sha256")"
        [[ -n "$receipt_id" ]] || fail "$asset is on release $release_id without its .sha256"
        echo "==> downloading $asset"
        download_asset "$id" "$assets_dir/$asset"
        download_asset "$receipt_id" "$assets_dir/$asset.sha256"
        found=$((found + 1))
    done
    [[ $found -gt 0 ]] || fail "release $release_id carries none of ${DESKTOP_ASSETS[*]}"
}

publish() {
    local source_sha="$1" run_id="$2"
    [[ "$source_sha" =~ ^[0-9a-f]{40}$ ]] || fail "source sha '$source_sha' is not 40 hex characters"
    local run_name; run_name="$(beta_run_name "$source_sha" "$run_id")"
    [[ "$run_name" =~ $BETA_NAME_RE ]] || fail "'$run_name' is not a beta name"
    require_r2
    RELEASE_TEMP_DIR="$(mktemp -d)"
    trap 'rm -rf "$RELEASE_TEMP_DIR"' EXIT

    # The job summary keeps the links even when the Discord post fails: the
    # post is the only other place they appear.
    local summary="${GITHUB_STEP_SUMMARY:-/dev/null}"
    echo "### Desktop beta $SHORT_VERSION (${source_sha:0:7})" >> "$summary"
    local prefix="beta/$run_name" row asset label sha256 mirrored=0
    for row in "${DESKTOP_ASSET_ROWS[@]}"; do
        IFS='|' read -r asset label <<< "$row"
        [[ -f "$assets_dir/$asset" ]] || continue
        echo "==> $asset"
        sha256="$(release_sha256 "$assets_dir/$asset")"
        mirror_asset "$assets_dir/$asset" "$assets_dir/$asset.sha256" "$asset" "$sha256" "$prefix"
        echo "- $label: $DOWNLOAD_HOST_URL/$prefix/$asset" >> "$summary"
        mirrored=$((mirrored + 1))
    done
    [[ $mirrored -gt 0 ]] || fail "$assets_dir holds none of ${DESKTOP_ASSETS[*]}"

    prune_older_betas "$run_name"

    echo ""
    echo "✓ beta $run_name mirrored to $DOWNLOAD_HOST_URL/$prefix/"
}

# Delete every beta/<name>/ but the one just written. The path is the literal
# beta/ plus a name matching BETA_NAME_RE, never a variable that could be
# empty: this bucket also holds the published installers.
prune_older_betas() {
    local keep="$1" name listing
    # `aws s3 ls` exits 1 on an empty prefix with nothing on stderr, so the exit
    # status alone cannot tell "nothing to prune" from a failed listing.
    listing="$(r2 s3 ls "s3://$R2_BUCKET/beta/" 2> "$RELEASE_TEMP_DIR/ls.err")" ||
        [[ ! -s "$RELEASE_TEMP_DIR/ls.err" ]] ||
        fail "cannot list beta/ in R2: $(cat "$RELEASE_TEMP_DIR/ls.err")"
    while read -r name; do
        [[ -n "$name" && "$name" != "$keep" ]] || continue
        [[ "$name" =~ $BETA_NAME_RE ]] || {
            echo "  leaving beta/$name alone — not a name this script writes"
            continue
        }
        echo "  removing beta/$name/"
        r2 s3 rm --only-show-errors --recursive "s3://$R2_BUCKET/beta/$name/" ||
            fail "cannot remove beta/$name/ from R2"
    done < <(awk '$1 == "PRE" { sub("/$", "", $2); print $2 }' <<< "$listing")
}

announce() {
    local source_sha="$1" run_id="$2" message
    [[ -n "${DISCORD_BETA_WEBHOOK:-}" ]] || fail "DISCORD_BETA_WEBHOOK is not set"
    [[ "$source_sha" =~ ^[0-9a-f]{40}$ ]] || fail "source sha '$source_sha' is not 40 hex characters"
    local prefix_url="$DOWNLOAD_HOST_URL/beta/$(beta_run_name "$source_sha" "$run_id")"
    # One "- <label>: <url>" bullet per installer this run mirrored.
    local downloads=""
    local row asset label
    for row in "${DESKTOP_ASSET_ROWS[@]}"; do
        IFS='|' read -r asset label <<< "$row"
        [[ -f "$assets_dir/$asset" ]] || continue
        downloads+="- $label: $prefix_url/$asset"$'\n'
    done
    [[ -n "$downloads" ]] || fail "$assets_dir holds none of ${DESKTOP_ASSETS[*]}"

    message="$(python3 -c '
import datetime, json, sys, zoneinfo
version, sha7, downloads = sys.argv[1:4]
# The train releases on the 5th: this month'"'"'s if it is still ahead (Taiwan
# time), otherwise next month'"'"'s.
today = datetime.datetime.now(zoneinfo.ZoneInfo("Asia/Taipei")).date()
release = today.replace(day=5)
if today.day >= 5:
    release = (today.replace(day=1) + datetime.timedelta(days=32)).replace(day=5)
print(json.dumps({
    "content": (
        f"**Desktop beta {version} ({sha7}) is ready for testing**\n"
        "If no major issues turn up (crashes, unable to type), "
        f"this build is released to all users on {release:%B} {release.day}.\n\n"
        f"{downloads}"
    ).rstrip(),
    "allowed_mentions": {"parse": []},
    # SUPPRESS_EMBEDS: no link preview cards (maintainer preference).
    "flags": 4,
}))
' "$SHORT_VERSION" "${source_sha:0:7}" "$downloads")"
    curl -q --silent --show-error --fail -H "Content-Type: application/json" \
        --data "$message" "$DISCORD_BETA_WEBHOOK" > /dev/null ||
        fail "the Discord webhook refused the post"
    echo "✓ posted beta ${source_sha:0:7} to Discord"
}

case "$mode" in
    fetch) assets_dir="$2" && fetch "$3" "$4" ;;
    publish) assets_dir="$2" && publish "$3" "$4" ;;
    announce) assets_dir="$2" && announce "$3" "$4" ;;
esac
