#!/bin/bash
# Auto-format Kotlin files after edit/write
command -v ktlint >/dev/null 2>&1 || exit 0
FILE=$(cat | jq -r '.tool_input.file_path // empty')
if [ -n "$FILE" ] && [ "${FILE##*.}" = "kt" ] && [ -f "$FILE" ]; then
  ktlint --format "$FILE" 2>/dev/null || true
fi
