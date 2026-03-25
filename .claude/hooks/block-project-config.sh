#!/bin/bash
# Block edits to project config files (.xcodeproj, .pbxproj, .gradle)
FILE=$(echo "$CLAUDE_TOOL_INPUT" | jq -r '.file_path // empty')
if [ -n "$FILE" ] && echo "$FILE" | grep -qE '\.(xcodeproj|xcworkspace|pbxproj)/|\.(xcodeproj|xcworkspace|pbxproj)$|\.gradle$|\.gradle\.kts$'; then
  echo "BLOCK: Project config files (.xcodeproj, .pbxproj, .gradle) must be edited manually by the user." >&2
  exit 2
fi
