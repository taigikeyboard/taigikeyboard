#!/bin/bash
#
# Distributes the compiled dictionary artifacts to every platform that ships them.
#
# Input:
#   - output/dictionary.fst     (prefix index: tl:/poj:/tps:/hanzi: keys)
#   - output/dictionary.bin     (binary mmap format)
#   - output/syllables.fst      (tl:/poj:/tps: tagged-single-FST syllable inventory)
#   - output/association.bin    (word_association binary)
# Output — one committed copy per platform:
#   - android/app/src/main/assets/
#   - ios/Resources/Dictionaries/
#   - macos/Resources/Dictionaries/
#   - windows/resources/Dictionaries/

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BASE_DIR="$(dirname "$SCRIPT_DIR")"
OUTPUT_DIR="$BASE_DIR/output"
REPOSITORY_DIR="$(cd "$BASE_DIR/.." && pwd)"

ARTIFACTS=(dictionary.fst dictionary.bin syllables.fst association.bin)

# `label|destination` per platform. Every destination must already exist: a
# missing one is a moved directory, not a platform to skip, and skipping it
# would leave that platform building against a stale dictionary.
DESTINATIONS=(
    "Android|$REPOSITORY_DIR/android/app/src/main/assets"
    "iOS|$REPOSITORY_DIR/ios/Resources/Dictionaries"
    "macOS|$REPOSITORY_DIR/macos/Resources/Dictionaries"
    "Windows|$REPOSITORY_DIR/windows/resources/Dictionaries"
)

echo "=================================================="
echo "deploy"
echo "=================================================="
echo "Source: $OUTPUT_DIR"
echo ""

# Validate every source and every destination before copying anything: a
# missing artifact or a missing platform directory then stops the run before it
# has updated anyone, rather than after some platforms and not others. A failure
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
