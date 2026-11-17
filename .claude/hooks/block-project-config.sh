#!/bin/bash
# Block edits to Xcode project config (.xcodeproj, .xcworkspace, .pbxproj).
# Claude Code passes the PreToolUse payload as JSON on stdin.
FILE=$(jq -r '.tool_input.file_path // .tool_input.notebook_path // empty') || { echo 'BLOCK: jq could not read the hook input.' >&2; exit 2; }
if [ -n "$FILE" ] && echo "$FILE" | grep -qE '\.(xcodeproj|xcworkspace|pbxproj)/|\.(xcodeproj|xcworkspace|pbxproj)$'; then
  echo "BLOCK: Xcode project files (.xcodeproj, .xcworkspace, .pbxproj) must be edited manually by the user." >&2
  exit 2
fi
