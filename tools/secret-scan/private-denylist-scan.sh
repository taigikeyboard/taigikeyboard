#!/usr/bin/env bash
# Blocks personal identifiers that are not credentials — a former address, an
# account or project ID, a mail label — which gitleaks has no rule for and which
# this public repository cannot list without publishing them.
#
# The list lives outside the repository, in the maintainer's private dotfiles:
# $TAIGI_PRIVATE_DENYLIST, default ~/.config/taigikeyboard-maintainer/private-denylist.txt
# (not ~/.config/taigikeyboard/, which is the Linux input method's settings directory).
# One fixed string per line, matched case-insensitively; `#` comments and blank
# lines are ignored. No list, nothing to check — a contributor's clone exits 0.
#
#   private-denylist-scan.sh --staged   lines the staged diff adds (pre-commit hook)
#   private-denylist-scan.sh --tree     every tracked text file (`make scan-private`);
#                                       binaries are skipped (git grep -I)
#   private-denylist-scan.sh --stdin <name>   the text on standard input; <name> is
#                                       what the report calls it (the commit-msg hook
#                                       pipes the commit message, .claude/hooks/
#                                       lint-before-pr.sh the `gh pr create` command)
#
# Prints offending file names only, never the matched value. Exit 0 clean, 1 hit.
#
# Scope: a local gate. It does not run in CI (the list is private), and a clone
# without `make hooks` or a `git commit --no-verify` skips it. Of the text that
# never enters the tree it sees commit messages and the title and inline body of
# a pull request Claude Code opens — not a `--body-file`, a pull request opened
# or edited any other way, or an issue. History is not rewritten, so older
# commits keep whatever they already published.
set -uo pipefail

LIST="${TAIGI_PRIVATE_DENYLIST:-$HOME/.config/taigikeyboard-maintainer/private-denylist.txt}"
[ -f "$LIST" ] || exit 0

cd "$(git rev-parse --show-toplevel)" || exit 1

PATTERNS=$(mktemp)
trap 'rm -f "$PATTERNS"' EXIT
# Drop CR, comments and blank lines — an empty pattern would match every line.
tr -d '\r' <"$LIST" | grep -v -e '^[[:space:]]*#' -e '^[[:space:]]*$' >"$PATTERNS"
[ -s "$PATTERNS" ] || exit 0

# Added lines of one file's staged diff, without the `+` marker. Everything
# before the first hunk header is the diff header (`+++ b/<path>` included).
# --text reads a binary file as text, so a string embedded in it is still seen.
added_lines() {
  git diff --cached --no-color --text -U0 -- "$1" |
    awk '/^@@/ { in_hunk = 1; next } in_hunk && /^\+/ { print substr($0, 2) }'
}

hits=()
case "${1:-}" in
  --staged)
    while IFS= read -r -d '' file; do
      if added_lines "$file" | grep -qiF -f "$PATTERNS"; then
        hits+=("$file")
      fi
    done < <(git diff --cached --name-only -z --diff-filter=ACMR)
    ;;
  --tree)
    while IFS= read -r -d '' file; do
      hits+=("$file")
    done < <(git grep -z -l -I -i -F -f "$PATTERNS")
    ;;
  --stdin)
    if grep -qiF -f "$PATTERNS"; then
      hits+=("${2:-standard input}")
    fi
    ;;
  *)
    echo "usage: $0 --staged | --tree | --stdin <name>" >&2
    exit 2
    ;;
esac

[ "${#hits[@]}" -eq 0 ] && exit 0

echo "private-denylist: a personal identifier listed in $LIST is in:" >&2
printf '  %s\n' "${hits[@]}" >&2
echo "Describe it generically instead (e.g. \"a former work address\"); never allowlist it in a tracked file." >&2
exit 1
