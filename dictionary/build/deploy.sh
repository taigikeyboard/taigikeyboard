#!/bin/bash
#
# Distributes the compiled dictionary artifacts to every platform that ships them.
#
# Input:
#   - output/dictionary.fst     (prefix index: tl:/poj:/tps:/hanzi: keys)
#   - output/dictionary.bin     (binary mmap format)
#   - output/syllables.fst      (tl:/poj:/tps: tagged-single-FST syllable inventory)
#   - output/association.bin    (word_association binary)
# Output — the one committed copy every platform packages from:
#   - dictionaries/

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BASE_DIR="$(dirname "$SCRIPT_DIR")"
OUTPUT_DIR="$BASE_DIR/output"
REPOSITORY_DIR="$(cd "$BASE_DIR/.." && pwd)"

ARTIFACTS=(dictionary.fst dictionary.bin syllables.fst association.bin)

# `label|destination`. One entry, and the loops below still iterate: the four
# platforms each reference this directory in their own build (Android through an
# assets source dir, iOS through a synchronized folder, macOS and Windows by
# copying at package time), so there is nothing left to fan out to. A missing
# destination is a moved directory, not a platform to skip.
DESTINATIONS=(
    "shared|$REPOSITORY_DIR/dictionaries"
)

echo "=================================================="
echo "deploy"
echo "=================================================="
echo "Source: $OUTPUT_DIR"
echo ""

# Validate every source and the destination before copying anything, so a
# missing artifact or a missing destination directory stops the run before it
# has half-written the shared directory every platform then packages. A failure
# during the copying itself still can, and `set -e` aborts on it.
for artifact in "${ARTIFACTS[@]}"; do
    if [ ! -f "$OUTPUT_DIR/$artifact" ] || [ ! -s "$OUTPUT_DIR/$artifact" ]; then
        echo "[ERROR] missing or empty artifact: $OUTPUT_DIR/$artifact"
        exit 1
    fi
done

for entry in "${DESTINATIONS[@]}"; do
    label="${entry%%|*}"
    destination="${entry#*|}"
    if [ ! -d "$destination" ]; then
        echo "[ERROR] $label destination directory not found: $destination"
        exit 1
    fi
done

for entry in "${DESTINATIONS[@]}"; do
    label="${entry%%|*}"
    destination="${entry#*|}"
    echo "[$label] $destination"
    for artifact in "${ARTIFACTS[@]}"; do
        cp "$OUTPUT_DIR/$artifact" "$destination/"
        printf '  - %-16s (%s)\n' "$artifact" "$(du -h "$OUTPUT_DIR/$artifact" | cut -f1)"
    done
    echo ""
done

echo "[INFO] Done!"
