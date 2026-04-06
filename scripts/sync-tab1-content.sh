#!/bin/bash
# Sync tab1 content JSON files from canonical source to both platform directories.
# Usage: ./scripts/sync-tab1-content.sh

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

IOS_DIR="$ROOT_DIR/ios/Sources/TaigiKeyboard/App/Tabs/Tab1"
ANDROID_DIR="$ROOT_DIR/android/app/src/main/assets"

for FILE in tab1-features.json tab1-faq.json; do
    cp "$ROOT_DIR/content/$FILE" "$IOS_DIR/$FILE"
    cp "$ROOT_DIR/content/$FILE" "$ANDROID_DIR/$FILE"
    echo "Synced $FILE to iOS and Android."
done
