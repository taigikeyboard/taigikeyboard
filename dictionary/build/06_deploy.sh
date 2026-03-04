#!/bin/bash
#
# 複製辭典檔案到 Android 和 iOS 專案
#
# 輸入：
#   - output/dictionary.db (包含 word_association 表)
#   - output/dictionary.trie
# 輸出：
#   - android/app/src/main/assets/
#   - ios/Resources/Dictionaries/

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BASE_DIR="$(dirname "$SCRIPT_DIR")"
OUTPUT_DIR="$BASE_DIR/output"

DB_FILE="$OUTPUT_DIR/dictionary.db"
TRIE_FILE="$OUTPUT_DIR/dictionary.trie"
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

cp "$DB_FILE" "$IOS_DEST/"
echo "  - dictionary.db   ($(du -h "$DB_FILE" | cut -f1))"

cp "$TRIE_FILE" "$IOS_DEST/"
echo "  - dictionary.trie ($(du -h "$TRIE_FILE" | cut -f1))"

echo ""
echo "[INFO] Done!"
echo "[NOTE] word_association table is now embedded in dictionary.db"
