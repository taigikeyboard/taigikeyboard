#!/usr/bin/env bash
# The ChromeOS input method's Chrome Web Store item, through the store's API v2.
#
# Usage: chrome-web-store.sh status
#        chrome-web-store.sh submit <version> <sha256>
#
#   status   print the item's published and submitted versions and the last
#            upload's state; changes nothing
#   submit   upload TaigiKeyboard-<version>-chromeos.zip from the published
#            desktop release and submit it for review, published on approval.
#            Only the package changes: the listing text, screenshots and privacy
#            fields stay as set in the Developer Dashboard. <sha256> is
#            announce-release.sh's `chromeos_sha256` — the digest of the zip it
#            checked against the staged receipt and the R2 copy — so run this
#            only after a successful announcement. `desktop-announce.yml` runs it;
#            in Actions it writes `chromeos_store_state` to $GITHUB_OUTPUT.
#
# Idempotent: a re-run finds the version already submitted or published and
# stops. Refuses a downgrade, and a submission while another version is in
# review (wait for the review, or cancel it in the Dashboard — not done here).
#
# Environment (docs/architecture/chromeos-release.md § One-time: the Chrome Web
# Store API):
#   CWS_ACCESS_TOKEN   a token with scope https://www.googleapis.com/auth/chromewebstore
#                      — in Actions minted through Workload Identity Federation;
#                      by hand, `gcloud auth print-access-token
#                      --impersonate-service-account=<CWS_SERVICE_ACCOUNT>
#                      --scopes=https://www.googleapis.com/auth/chromewebstore`
#   CWS_PUBLISHER_ID   Developer Dashboard → Publisher → Settings
#   CWS_ITEM_ID        the item's 32-letter id

set -euo pipefail

fail() { echo "error: $*" >&2; exit 1; }

REPOSITORY_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
API=https://chromewebstore.googleapis.com
USAGE="usage: chrome-web-store.sh status | submit <version> <sha256>"

for name in CWS_ACCESS_TOKEN CWS_PUBLISHER_ID CWS_ITEM_ID; do
    [[ -n "${!name:-}" ]] ||
        fail "$name is not set (docs/architecture/chromeos-release.md § One-time: the Chrome Web Store API)"
done
ITEM="publishers/$CWS_PUBLISHER_ID/items/$CWS_ITEM_ID"

# One authenticated store call; prints the response body. The URL is the last
# argument. On an HTTP error Google's body names the cause (chromeos-release.md
# § One-time, step 7), so it goes to stderr before failing.
store_call() {
    local body
    if ! body="$(curl --silent --show-error --fail-with-body \
        -H "Authorization: Bearer $CWS_ACCESS_TOKEN" "$@")"; then
        printf '%s\n' "$body" >&2
        fail "Chrome Web Store call failed: ${*: -1}"
    fi
    printf '%s\n' "$body"
}

fetch_status() {
    store_call "$API/v2/$ITEM:fetchStatus"
}

print_status() {
    jq -r '
      def revision($key):
        if .[$key] then "\($key): \(.[$key].state) (\([.[$key].distributionChannels[]?.crxVersion] | join(", ")))"
        else "\($key): none" end;
      "item \(.itemId)",
      revision("publishedItemRevisionStatus"),
      revision("submittedItemRevisionStatus"),
      "lastAsyncUploadState: \(.lastAsyncUploadState // "none")"'
}

# A revision's (first channel's) version from a fetchStatus body.
revision_version() {
    jq -r --arg key "$1" '[.[$key].distributionChannels[]?.crxVersion] | first // ""'
}

report_state() {
    echo "Chrome Web Store: $1"
    [[ -z "${GITHUB_OUTPUT:-}" ]] || echo "chromeos_store_state=$1" >> "$GITHUB_OUTPUT"
}

submit() {
    local version="$1" sha256="$2"
    [[ "$version" =~ ^[0-9]+(\.[0-9]+)+$ ]] || fail "version '$version' is not dotted integers"
    [[ "$sha256" =~ ^[0-9a-f]{64}$ ]] || fail "sha256 '$sha256' is not 64 lowercase hex digits"

    # The release's tag, asset name and hash helper, so this cannot drift from
    # what staging attached; the anonymous download, so the bytes are the ones
    # every user could fetch.
    SHORT_VERSION="$version"
    # shellcheck source=lib/desktop-release.sh
    source "$REPOSITORY_DIR/tools/release/lib/desktop-release.sh"
    # shellcheck source=lib/release-site.sh
    source "$REPOSITORY_DIR/tools/release/lib/release-site.sh"

    local status published submitted submitted_state
    status="$(fetch_status)"
    printf '%s\n' "$status" | print_status
    published="$(revision_version publishedItemRevisionStatus <<< "$status")"
    submitted="$(revision_version submittedItemRevisionStatus <<< "$status")"
    submitted_state="$(jq -r '.submittedItemRevisionStatus.state // ""' <<< "$status")"

    case "$submitted_state" in
        PENDING_REVIEW | STAGED | PUBLISHED_TO_TESTERS)
            if [[ "$submitted" == "$version" ]]; then
                report_state "$submitted_state (already submitted)"
                return
            fi
            # STAGED passed review and waits for a manual publish.
            fail "$submitted is $submitted_state — let its review finish (STAGED: publish it), or cancel it in the Developer Dashboard, then re-run"
            ;;
        REJECTED | CANCELLED)
            # A finished submission: a new upload replaces it.
            submitted=""
            ;;
        "" | PUBLISHED) ;;
        *) fail "the submitted revision is in state '$submitted_state', which this script does not know" ;;
    esac
    if [[ "$published" == "$version" && ( -z "$submitted" || "$submitted" == "$version" ) ]]; then
        report_state "PUBLISHED (already published)"
        return
    fi
    local newest
    for newest in "$published" "$submitted"; do
        [[ -z "$newest" ]] && continue
        printf '%s\n' "$newest" "$version" | sort -VC ||
            fail "the store already has $newest, newer than $version — not downgrading it"
    done

    local zip="$RELEASE_TEMP_DIR/$CHROMEOS_ASSET"
    local asset_url="https://github.com/$RELEASE_REPOSITORY/releases/download/$DESKTOP_TAG/$CHROMEOS_ASSET"
    echo "==> downloading $asset_url"
    anonymous_download_retrying "$asset_url" "$zip" || fail "$asset_url is not anonymously reachable"
    [[ "$(release_sha256 "$zip")" == "$sha256" ]] ||
        fail "$CHROMEOS_ASSET does not hash to $sha256 — the announcement checked a different file"

    echo "==> uploading $CHROMEOS_ASSET"
    local upload upload_state crx_version
    upload="$(store_call -X POST -H "Content-Type: application/zip" --data-binary "@$zip" \
        "$API/upload/v2/$ITEM:upload")"
    upload_state="$(jq -r '.uploadState // ""' <<< "$upload")"
    crx_version="$(jq -r '.crxVersion // ""' <<< "$upload")"
    # The reference names the pending state both ways.
    for _ in $(seq 1 30); do
        [[ "$upload_state" == IN_PROGRESS || "$upload_state" == UPLOAD_IN_PROGRESS ]] || break
        echo "  upload in progress — checking again in 10s"
        sleep 10
        upload_state="$(fetch_status | jq -r '.lastAsyncUploadState // ""')"
    done
    [[ "$upload_state" == SUCCEEDED ]] || fail "upload ended $upload_state: $upload"
    [[ -z "$crx_version" || "$crx_version" == "$version" ]] ||
        fail "the store read version $crx_version from $CHROMEOS_ASSET, not $version"

    echo "==> submitting $version for review"
    local published_state
    published_state="$(store_call -X POST -H "Content-Type: application/json" \
        --data '{"publishType":"DEFAULT_PUBLISH"}' "$API/v2/$ITEM:publish" | jq -r '.state // ""')"
    case "$published_state" in
        PENDING_REVIEW | STAGED | PUBLISHED | PUBLISHED_TO_TESTERS) report_state "$published_state" ;;
        *) fail "publish returned state '$published_state'" ;;
    esac
}

case "${1:-}" in
    status)
        [[ $# -eq 1 ]] || fail "$USAGE"
        fetch_status | print_status
        ;;
    submit)
        [[ $# -eq 3 ]] || fail "$USAGE"
        RELEASE_TEMP_DIR="$(mktemp -d)"
        trap 'rm -rf "$RELEASE_TEMP_DIR"' EXIT
        submit "$2" "$3"
        ;;
    *)
        echo "$USAGE" >&2
        exit 2
        ;;
esac
