#!/bin/bash
#
# 複製辭典檔案到 Android 和 iOS 專案
#
# 輸入：
#   - output/dictionary.db      (Android: 完整 DB / iOS: 暫時保留)
#   - output/dictionary.trie    (雙平台共用)
#   - output/dictionary.bin     (iOS only: binary mmap 格式)
#   - output/association.bin    (iOS only: word_association binary)
#   - output/hanzi.trie         (iOS only: 漢字前綴搜尋)
# 輸出：
#   - android/app/src/main/assets/
#   - ios/Resources/Dictionaries/

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BASE_DIR="$(dirname "$SCRIPT_DIR")"
OUTPUT_DIR="$BASE_DIR/output"

DB_FILE="$OUTPUT_DIR/dictionary.db"
TRIE_FILE="$OUTPUT_DIR/dictionary.trie"
DICT_BIN="$OUTPUT_DIR/dictionary.bin"
ASSOC_BIN="$OUTPUT_DIR/association.bin"
HANZI_TRIE="$OUTPUT_DIR/hanzi.trie"
ANDROID_DEST="$BASE_DIR/../android/app/src/main/assets"
IOS_DEST="$BASE_DIR/../ios/Resources/Dictionaries"

echo "=================================================="
echo "06_deploy"
echo "=================================================="
echo "Source: $OUTPUT_DIR"
echo ""

# 檢查來源檔案
if [ ! -f "$DB_FILE" ]; then
    echo "[ERROR] Database not found: $DB_FILE"
    exit 1
fi

if [ ! -f "$TRIE_FILE" ]; then
    echo "[ERROR] Trie not found: $TRIE_FILE"
    exit 1
fi

for f in "$DICT_BIN" "$ASSOC_BIN" "$HANZI_TRIE"; do
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

cp "$DB_FILE" "$ANDROID_DEST/"
echo "  - dictionary.db   ($(du -h "$DB_FILE" | cut -f1))"

cp "$TRIE_FILE" "$ANDROID_DEST/"
echo "  - dictionary.trie ($(du -h "$TRIE_FILE" | cut -f1))"

echo ""

# --- iOS ---
echo "[iOS] $IOS_DEST"
if [ ! -d "$IOS_DEST" ]; then
    echo "[ERROR] iOS Dictionaries directory not found: $IOS_DEST"
    exit 1
fi

cp "$TRIE_FILE" "$IOS_DEST/"
echo "  - dictionary.trie  ($(du -h "$TRIE_FILE" | cut -f1))"

cp "$DICT_BIN" "$IOS_DEST/"
echo "  - dictionary.bin   ($(du -h "$DICT_BIN" | cut -f1))"

cp "$ASSOC_BIN" "$IOS_DEST/"
echo "  - association.bin  ($(du -h "$ASSOC_BIN" | cut -f1))"

cp "$HANZI_TRIE" "$IOS_DEST/"
echo "  - hanzi.trie       ($(du -h "$HANZI_TRIE" | cut -f1))"

echo ""
echo "[INFO] Done!"
echo "[NOTE] iOS uses binary mmap format (no dictionary.db)"
echo "[NOTE] Android still uses dictionary.db"
