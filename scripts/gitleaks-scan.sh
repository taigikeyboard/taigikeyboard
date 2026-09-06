#!/usr/bin/env bash
# The one place that decides what a credential scan covers and how its result is
# read. `make scan-secrets`, `make scan-secrets-full` and
# .github/workflows/secrets.yml all go through here, so the local gate and the CI
# gate cannot drift apart — they did once already, when only one of the two
# carried --exit-code 2.
#
#   gitleaks-scan.sh                  everything .gitleaks-scanned does not cover
#   gitleaks-scan.sh <base>..<head>   one range, for a pull request or a push
#   gitleaks-scan.sh --full           the whole history, to re-baseline
#   gitleaks-scan.sh --staged         what is staged right now, for the commit hook
#
# Exit 0 clean, 1 anything else. Under GitHub Actions the messages are emitted as
# workflow annotations.
set -uo pipefail

readonly BASELINE_FILE=.gitleaks-scanned

# Passing --log-opts at all replaces gitleaks' own defaults, so what it would have
# used has to be restated. --diff-filter=tuxdb drops deletions, type changes and
# unmerged entries, whose content is already covered by the commit that introduced
# it. --diff-merges=first-parent is added on top: `git log -p` prints no patch for
# a merge commit at all, so a credential added while resolving a conflict — one
# that exists in neither parent — is otherwise invisible. Not --first-parent,
# which would also stop the walk from entering the merged branch.
readonly LOG_OPTS_COMMON="--diff-filter=tuxdb --diff-merges=first-parent"
# gitleaks' other two defaults. They belong to a whole-repository scan and must
# stay off a range scan, where --all would widen A..B back to everything.
readonly LOG_OPTS_ALL_REFS="--full-history --all"

cd "$(git rev-parse --show-toplevel)" || exit 1

fail() {
  if [ -n "${GITHUB_ACTIONS:-}" ]; then printf '::error::%s\n' "$@" >&2; else printf '%s\n' "$@" >&2; fi
  exit 1
}

note() {
  if [ -n "${GITHUB_ACTIONS:-}" ]; then printf '::warning::%s\n' "$1" >&2; else printf '%s\n' "$1" >&2; fi
}

baseline() { sed -n "s/^$1=//p" "$BASELINE_FILE"; }

# The recorded scan vouches for the history behind the watermark under one
# gitleaks version's rules. Running a different one and still skipping that
# history would present the old rules' verdict as the new rules' verdict.
require_recorded_version() {
  local recorded actual
  recorded=$(baseline gitleaks)
  actual=$(gitleaks version 2>/dev/null)
  [ "$recorded" = "$actual" ] && return 0
  fail "$BASELINE_FILE records gitleaks $recorded, but $actual is installed." \
       "The recorded scan cannot vouch for another version's rules." \
       "Re-baseline with \`make scan-secrets-full\`, or install gitleaks $recorded."
}

# gitleaks reports a finding and its own errors both as 1 by default. --exit-code 2
# separates them, so a broken config cannot read as a pass and a finding cannot
# read as a crash.
run() {
  local status=0
  gitleaks git --no-banner --redact -v --exit-code 2 "$@" || status=$?
  case "$status" in
    0) echo "No findings." ;;
    2) fail "gitleaks found a credential. Rotate it, then remove it from the branch." ;;
    *) fail "gitleaks failed to run (exit $status) — nothing was scanned." ;;
  esac
}

scan_log() { run --log-opts="$LOG_OPTS_COMMON $*"; }

command -v gitleaks >/dev/null 2>&1 || fail "gitleaks is not installed: brew install gitleaks"
[ -f "$BASELINE_FILE" ] || fail "$BASELINE_FILE is missing — it records what has already been scanned."

case "${1:---incremental}" in
  --full)
    echo "Scanning the whole history"
    scan_log "$LOG_OPTS_ALL_REFS"
    cat <<MSG

✓ full history clean. Re-baseline $BASELINE_FILE with:
    commit=$(git rev-parse HEAD)
    date=$(date +%F)
    gitleaks=$(gitleaks version)
MSG
    ;;
  --staged)
    # No baseline check: this reads the index, not history, so nothing the
    # watermark vouches for is being skipped.
    run --staged --log-level warn
    ;;
  --incremental)
    require_recorded_version
    watermark=$(baseline commit)
    git cat-file -e "$watermark^{commit}" 2>/dev/null \
      || fail "$BASELINE_FILE names $watermark, which is not in this repository." \
              "History was rewritten, or the file is stale — re-baseline with \`make scan-secrets-full\`."
    echo "Scanning everything not reachable from $watermark (scanned clean $(baseline date))"
    scan_log "$LOG_OPTS_ALL_REFS" "--not $watermark"
    ;;
  *..*)
    require_recorded_version
    echo "Scanning $1"
    scan_log "$1"
    ;;
  *)
    fail "usage: $0 [--full | --incremental | --staged | <base>..<head>]"
    ;;
esac
