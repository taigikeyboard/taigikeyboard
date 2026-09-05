#!/bin/bash
#
# Dictionary build entry point — every run is a full rebuild + deploy.
# Pipeline:
#   1. merge_csv              - merge source CSVs (drop stats -> .build_stats.json)
#   2. create_dictionary_bin  - dictionary.bin (binary mmap) + shared build_ts
#   3. create_fst             - dictionary.fst prefix index (tl:/poj:/tps:/hanzi:)
#   4. create_syllables_fst   - syllables.fst inventory (tl:/poj:/tps: tagged single FST)
#   5. create_association_bin - association.bin (binary mmap), reusing build_ts
#   6. verify_poj_integrity   - fail-fast POJ invariant gate
#   7. version_snapshot       - drop summary + dictionary.csv diff vs previous release tag
#   8. verify_known_keys      - fst fixture check (known keys such as poj:chi2)
#   9. deploy                 - copy into the Android/iOS projects (incl. syllables.fst)
#
# RELEASE_VERSION=vX.Y.Z (or $1) sets step 7's diff base to the newest tag strictly below it;
# unset compares against the newest release tag. Previous contents are read from
# `git show <tag>:dictionary/output/dictionary.csv`; no snapshot file is written.
# output/ is cleared manually; there is no clean / deploy-only subcommand.

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BUILD_DIR="$SCRIPT_DIR/build"
OUTPUT_DIR="$SCRIPT_DIR/output"

# Build scripts import from `build.common` + `common.*`; run them as modules
# from dictionary/ so the package imports resolve without sys.path hacks.
export PYTHONPATH="$SCRIPT_DIR"
cd "$SCRIPT_DIR"

GREEN='\033[0;32m'
NC='\033[0m'

# Release version label: env RELEASE_VERSION or first positional arg.
RELEASE_VERSION="${RELEASE_VERSION:-${1:-}}"

step() {
    echo ""
    echo -e "${GREEN}▶ $1${NC}"
    echo ""
}

step "Step 1/9: Merging dictionaries..."
python3 -m build.merge_csv

step "Step 2/9: Creating dictionary.bin..."
python3 -m build.create_dictionary_bin --verify

step "Step 3/9: Creating fst prefix index..."
python3 -m build.create_fst

step "Step 4/9: Creating syllables.fst (tl/poj/tps tagged-single-FST)..."
python3 -m build.create_syllables_fst

step "Step 5/9: Creating association.bin..."
python3 -m build.create_association_bin --verify

step "Step 6/9: Verifying POJ integrity (fatal gate)..."
python3 -m build.verify_poj_integrity

step "Step 7/9: Drop summary + vs-previous-release-tag diff..."
if [ -n "$RELEASE_VERSION" ]; then
    python3 -m build.version_snapshot --version "$RELEASE_VERSION"
else
    python3 -m build.version_snapshot
fi

step "Step 8/9: Verifying known fst keys..."
python3 -m tools.verify_known_keys

step "Step 9/9: Deploying to Android/iOS..."
bash "$BUILD_DIR/deploy.sh"

echo ""
echo -e "${GREEN}✓ Build and deploy complete!${NC}"
echo "  Output: $OUTPUT_DIR"
