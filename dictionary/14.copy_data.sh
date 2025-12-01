#!/bin/bash

set -e

DB_FILE="./dictionary.db"
ANDROID_DEST="../android/app/src/main/assets/dictionary.db"
IOS_DEST="../ios/Resources/Dictionaries/dictionary.db"

if [ ! -f "$DB_FILE" ]; then
    echo "[ERROR] Database file not found: $DB_FILE"
    exit 1
fi

echo "[INFO] Copying dictionary.db..."

cp "$DB_FILE" "$ANDROID_DEST"
echo "[INFO] Copied to Android: $ANDROID_DEST"

cp "$DB_FILE" "$IOS_DEST"
echo "[INFO] Copied to iOS: $IOS_DEST"

echo "[INFO] Done!"
