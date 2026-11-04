#!/bin/bash
# Run `make lint` before `gh pr create`; a failure blocks the PR.
# The `if` filter in settings.json is best-effort: Claude Code runs the hook
# whenever it cannot parse a Bash command (for loops, for example), so the
# command is re-checked here. Claude Code passes the PreToolUse payload on stdin.
CMD=$(jq -r '.tool_input.command // empty') || { echo 'BLOCK: jq could not read the hook input.' >&2; exit 2; }
echo "$CMD" | grep -qE '(^|[^[:alnum:]_-])gh[[:space:]]+pr[[:space:]]+create([^[:alnum:]_-]|$)' || exit 0
make -C "${CLAUDE_PROJECT_DIR}" lint 1>&2 || {
  echo 'BLOCK: make lint failed — fix lint before opening the PR (CI gates the same checks).' >&2
  exit 2
}
