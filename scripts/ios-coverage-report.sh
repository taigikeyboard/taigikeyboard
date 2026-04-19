#!/usr/bin/env bash
# ios-coverage-report.sh
#
# Parse a user-generated xcresult bundle and print per-file line-coverage for
# the 10 G9 shared-core candidates (see docs/architecture/g9-coverage-matrix.md).
# This script is READ-ONLY: it never invokes xcodebuild. The user runs the
# test suite manually per `feedback_manual_build_test`; this script just
# extracts numbers from the bundle they produce.
#
# Usage:
#   xcodebuild test \
#     -scheme TaigiKeyboard \
#     -destination "platform=iOS Simulator,name=iPhone 15" \
#     -enableCodeCoverage YES \
#     -resultBundlePath /tmp/taigi-g9.xcresult
#
#   scripts/ios-coverage-report.sh /tmp/taigi-g9.xcresult
#
# Exits non-zero if any candidate falls below the 70% gate.

set -euo pipefail

XCRESULT="${1:-}"
if [[ -z "$XCRESULT" || ! -d "$XCRESULT" ]]; then
  echo "usage: $0 <path-to-xcresult-bundle>" >&2
  exit 64
fi

if ! command -v xcrun >/dev/null 2>&1; then
  echo "error: xcrun not found — this script only runs on macOS with Xcode" >&2
  exit 69
fi

# Shell `<<heredoc` overrides stdin, so we can't combine `xcrun | python3 <<PY`
# with `json.load(sys.stdin)` — the heredoc would win and the JSON never arrives.
# Instead, stage xccov output to a temp file and pass its path through argv.
TMP_JSON="$(mktemp -t g9-coverage.XXXXXX)"
trap 'rm -f "$TMP_JSON"' EXIT
xcrun xccov view --report --json "$XCRESULT" > "$TMP_JSON"

python3 - "$TMP_JSON" <<'PY'
import json
import sys

# Paths are not used — xccov reports by basename under each target.
G9_CANDIDATES = {
    "PhoneticsConverter.swift",
    "SyllableParser.swift",
    "TPSToTL.swift",
    "TLToTPS.swift",
    "InputNormalizer.swift",
    "CandidateProcessor.swift",
    "NextWordScorer.swift",
    "AutocompleteContextBooster.swift",
    "CaseTransformer.swift",
    "CustomDictionaryDerivation.swift",
}
GATE = 0.70

with open(sys.argv[1]) as f:
    report = json.load(f)

# xccov report shape: { targets: [ { files: [ { name, lineCoverage, ... } ] } ] }
rows = []
seen = set()
for target in report.get("targets", []):
    for file_entry in target.get("files", []):
        name = file_entry.get("name", "")
        if name in G9_CANDIDATES and name not in seen:
            rows.append((name, file_entry.get("lineCoverage", 0.0)))
            seen.add(name)

missing = sorted(G9_CANDIDATES - seen)

rows.sort()
failing = [(n, c) for n, c in rows if c < GATE]

width = max((len(n) for n, _ in rows), default=20)
print(f"{'file'.ljust(width)}  coverage   status")
print(f"{'-' * width}  --------   ------")
for name, cov in rows:
    status = "OK" if cov >= GATE else "BELOW"
    print(f"{name.ljust(width)}  {cov * 100:6.2f}%   {status}")

if missing:
    print()
    print("Candidates not found in xcresult (check that target compiled & linked):")
    for name in missing:
        print(f"  - {name}")

print()
if failing or missing:
    print(f"GATE FAIL — {len(failing)} candidate(s) below {int(GATE * 100)}%, {len(missing)} missing")
    sys.exit(1)
else:
    print(f"GATE PASS — all {len(rows)} candidates ≥ {int(GATE * 100)}%")
PY
