#!/bin/bash
#
# 複製辭典檔案到 Android 和 iOS 專案
#
# 輸入：
#   - output/dictionary.fst     (雙平台共用，fst 前綴索引含 tl:/poj:/tps:/hanzi: keys)
#   - output/dictionary.bin     (雙平台共用: binary mmap 格式)
#   - output/syllables.fst      (雙平台共用: tl:/poj:/tps: tagged-single-FST 音節庫)
#   - output/association.bin    (雙平台共用: word_association binary)
# 輸出：
#   - android/app/src/main/assets/
#   - ios/Resources/Dictionaries/

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BASE_DIR="$(dirname "$SCRIPT_DIR")"
OUTPUT_DIR="$BASE_DIR/output"

FST_FILE="$OUTPUT_DIR/dictionary.fst"
DICT_BIN="$OUTPUT_DIR/dictionary.bin"
SYLLABLES_FST="$OUTPUT_DIR/syllables.fst"
ASSOC_BIN="$OUTPUT_DIR/association.bin"
ANDROID_DEST="$BASE_DIR/../android/app/src/main/assets"
IOS_DEST="$BASE_DIR/../ios/Resources/Dictionaries"

echo "=================================================="
echo "deploy"
echo "=================================================="
echo "Source: $OUTPUT_DIR"
echo ""

# 檢查來源檔案
if [ ! -f "$FST_FILE" ]; then
    echo "[ERROR] FST index not found: $FST_FILE"
    exit 1
fi

for f in "$DICT_BIN" "$SYLLABLES_FST" "$ASSOC_BIN"; do
    if [ ! -f "$f" ]; then
        echo "[ERROR] Binary file not found: $f"
        exit 1
    fi
done

# --- Android ---
echo "[Android] $ANDROID_DEST"
if [ ! -d "$ANDROID_DEST" ]; then
    echo "[ERROR] Android assets directory not found: $ANDROID_DEST"
    exit 1
fi

cp "$FST_FILE" "$ANDROID_DEST/"
echo "  - dictionary.fst  ($(du -h "$FST_FILE" | cut -f1))"

cp "$DICT_BIN" "$ANDROID_DEST/"
echo "  - dictionary.bin  ($(du -h "$DICT_BIN" | cut -f1))"

cp "$SYLLABLES_FST" "$ANDROID_DEST/"
echo "  - syllables.fst   ($(du -h "$SYLLABLES_FST" | cut -f1))"

cp "$ASSOC_BIN" "$ANDROID_DEST/"
echo "  - association.bin ($(du -h "$ASSOC_BIN" | cut -f1))"

echo ""

# --- iOS ---
echo "[iOS] $IOS_DEST"
if [ ! -d "$IOS_DEST" ]; then
    echo "[ERROR] iOS Dictionaries directory not found: $IOS_DEST"
    exit 1
fi

cp "$FST_FILE" "$IOS_DEST/"
echo "  - dictionary.fst  ($(du -h "$FST_FILE" | cut -f1))"

cp "$DICT_BIN" "$IOS_DEST/"
echo "  - dictionary.bin  ($(du -h "$DICT_BIN" | cut -f1))"

cp "$SYLLABLES_FST" "$IOS_DEST/"
echo "  - syllables.fst   ($(du -h "$SYLLABLES_FST" | cut -f1))"

cp "$ASSOC_BIN" "$IOS_DEST/"
echo "  - association.bin ($(du -h "$ASSOC_BIN" | cut -f1))"

echo ""
echo "[INFO] Done!"
