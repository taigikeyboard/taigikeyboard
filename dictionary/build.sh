#!/bin/bash
#
# Dictionary build entry point — every run is a full rebuild + deploy.
# Pipeline (steps 1-8 in one process, build/run.py):
#   1. merge_csv              - merge source CSVs (drop stats -> .build_stats.json)
#   2. create_dictionary_bin  - dictionary.bin (binary mmap)
#   3. create_fst             - dictionary.fst prefix index (tl:/poj:/tps:/hanzi:)
#   4. create_syllables_fst   - syllables.fst inventory (tl:/poj:/tps: tagged single FST)
#   5. create_association_bin - association.bin (binary mmap)
#   6. verify_poj_integrity   - fail-fast POJ invariant gate
#   7. version_snapshot       - drop summary + word-keys.tsv diff vs previous release tag
#   8. verify_known_keys      - fst fixture check (known keys such as poj:chi2)
#   9. deploy                 - copy into assets/dictionaries/, which every platform packages (incl. syllables.fst)
#
# RELEASE_VERSION=vX.Y.Z (or $1) sets step 7's diff base to the newest tag strictly below it;
# unset compares against the newest release tag. Step 7 writes the committed
# `dictionary/word-keys.tsv` and reads the previous release's copy from
# `git show <tag>:dictionary/word-keys.tsv` — the 3.1 MB key set, not the 35 MB CSV,
# so output/ never has to be committed for the diff to work.
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

# Steps 1-8 run in one Python process (build/run.py) so the CSV is parsed and
# the Node converter started once; each step module still runs on its own.
if [ -n "$RELEASE_VERSION" ]; then
    python3 -m build.run --version "$RELEASE_VERSION"
else
    python3 -m build.run
fi

step "Step 9/9: Deploying to assets/dictionaries/..."
bash "$BUILD_DIR/deploy.sh"

echo ""
echo -e "${GREEN}✓ Build and deploy complete!${NC}"
echo "  Output: $OUTPUT_DIR"
