#!/usr/bin/env bash
# The ChromeOS input method's Chrome Web Store item, through the store's API v2.
#
# Usage: chrome-web-store.sh status
#
#   status   print the item's published and submitted versions and the last
#            upload's state; changes nothing
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

API=https://chromewebstore.googleapis.com/v2

for name in CWS_ACCESS_TOKEN CWS_PUBLISHER_ID CWS_ITEM_ID; do
    [[ -n "${!name:-}" ]] ||
        fail "$name is not set (docs/architecture/chromeos-release.md § One-time: the Chrome Web Store API)"
done
ITEM="publishers/$CWS_PUBLISHER_ID/items/$CWS_ITEM_ID"

# One authenticated store call; prints the response body. On an HTTP error
# Google's body names the cause (chromeos-release.md § One-time, step 7), so it
# goes to stderr before failing.
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
    store_call "$API/$ITEM:fetchStatus"
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

case "${1:-}" in
    status)
        [[ $# -eq 1 ]] || fail "usage: chrome-web-store.sh status"
        fetch_status | print_status
        ;;
    *)
        echo "usage: chrome-web-store.sh status" >&2
        exit 2
        ;;
esac
