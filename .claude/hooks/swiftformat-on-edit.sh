#!/bin/bash
# Auto-format Swift files after edit/write
FILE=$(echo "$CLAUDE_TOOL_INPUT" | jq -r '.file_path // empty')
if [ -n "$FILE" ] && [ "${FILE##*.}" = "swift" ] && [ -f "$FILE" ]; then
  swiftformat "$FILE" --quiet 2>/dev/null
fi
