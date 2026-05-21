#!/bin/bash
# Block edits to Xcode project config (.xcodeproj, .xcworkspace, .pbxproj).
# 2026-05-21: .gradle / .gradle.kts removed from block-list per project CLAUDE.md
# Core Principle #1 (Android Gradle editable by Claude since 2026-05-09).
FILE=$(echo "$CLAUDE_TOOL_INPUT" | jq -r '.file_path // empty')
if [ -n "$FILE" ] && echo "$FILE" | grep -qE '\.(xcodeproj|xcworkspace|pbxproj)/|\.(xcodeproj|xcworkspace|pbxproj)$'; then
  echo "BLOCK: Xcode project files (.xcodeproj, .xcworkspace, .pbxproj) must be edited manually by the user." >&2
  exit 2
fi
