#!/bin/bash
# Auto-format Swift files after edit/write
FILE=$(cat | jq -r '.tool_input.file_path // empty')
if [ -n "$FILE" ] && [ "${FILE##*.}" = "swift" ] && [ -f "$FILE" ]; then
  swiftformat "$FILE" --quiet 2>/dev/null
fi
