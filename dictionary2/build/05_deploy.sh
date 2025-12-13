#!/bin/bash
#
# 複製辭典檔案到 Android 專案
#
# 輸入：output/dictionary.db, output/dictionary.trie
# 輸出：android/app/src/main/assets/

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BASE_DIR="$(dirname "$SCRIPT_DIR")"
OUTPUT_DIR="$BASE_DIR/output"

DB_FILE="$OUTPUT_DIR/dictionary.db"
TRIE_FILE="$OUTPUT_DIR/dictionary.trie"
ANDROID_DEST="$BASE_DIR/../android/app/src/main/assets"

echo "=================================================="
echo "04_deploy"
echo "=================================================="
echo "Source: $OUTPUT_DIR"
echo "Dest:   $ANDROID_DEST"
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

# 檢查目標目錄
if [ ! -d "$ANDROID_DEST" ]; then
    echo "[ERROR] Android assets directory not found: $ANDROID_DEST"
    exit 1
fi

echo "[INFO] Copying dictionary files to Android..."

cp "$DB_FILE" "$ANDROID_DEST/"
echo "  - dictionary.db   ($(du -h "$DB_FILE" | cut -f1))"

cp "$TRIE_FILE" "$ANDROID_DEST/"
echo "  - dictionary.trie ($(du -h "$TRIE_FILE" | cut -f1))"

echo ""
echo "[INFO] Done!"
