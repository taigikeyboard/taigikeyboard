#!/usr/bin/env python3
"""`.claude/rules/*.md` and `docs/contributing/README.md` name the same paths.

`docs/contributing/README.md` § Which guide for which path is the list every
coding agent reads. Claude Code is pointed at a guide by the `paths:`
frontmatter of that guide's pointer in `.claude/rules/`, so the two lists must
name the same guides with the same globs — otherwise the tools disagree about
which guide a file needs.

Usage: python3 tools/agent_rule_paths.py   (exit 1 lists every difference)
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
README = "docs/contributing/README.md"
RULES = ".claude/rules"
SECTION = "Which guide for which path"
HEADING = f"## {SECTION}"

CODE_SPAN = re.compile(r"`([^`]+)`")
FRONTMATTER = re.compile(r"\A---\n(.*?)\n---\n", re.S)
# The `paths:` value: the rest of its line plus the indented lines after it.
PATHS_VALUE = re.compile(r"^paths:(.*(?:\n[ \t]+.*)*)", re.M)


def table_globs(readme: str) -> dict[str, set[str]]:
    """Guide file name -> globs, from the table under `HEADING` only."""
    parts = re.split(rf"^{re.escape(HEADING)}\n", readme, maxsplit=1, flags=re.M)
    if len(parts) != 2:
        raise ValueError(f"{README} has no `{HEADING}` section")
    section = re.split(r"^#+ ", parts[1], maxsplit=1, flags=re.M)[0]
    table: dict[str, set[str]] = {}
    for line in section.splitlines():
        if not line.startswith("|"):
            continue
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        if len(cells) != 2:  # a `|` inside a glob would hide the row
            raise ValueError(f"{README}: a row under `{HEADING}` is not two cells: {line}")
        guide = CODE_SPAN.search(cells[0])
        if guide is None:  # header and separator rows
            continue
        if guide.group(1) in table:
            raise ValueError(f"{README} lists {guide.group(1)} twice")
        table[guide.group(1)] = set(CODE_SPAN.findall(cells[1]))
    return table


def rule_globs(rule: str) -> set[str]:
    """The double-quoted globs of a rule file's `paths:` frontmatter.

    Reads the two forms the rule files use — an inline `["a", "b"]` list and an
    indented `- "a"` block list — not YAML in general; any other form reads as
    no globs, which `differences` reports.
    """
    frontmatter = FRONTMATTER.match(rule)
    paths = PATHS_VALUE.search(frontmatter.group(1)) if frontmatter else None
    if paths is None:
        return set()
    without_comments = re.sub(r"\s#.*$", "", paths.group(1), flags=re.M)
    return set(re.findall(r'"([^"]+)"', without_comments))


def differences(
    table: dict[str, set[str]], rules: dict[str, set[str]], guides: set[str]
) -> list[str]:
    failures = []
    for name in sorted(table.keys() | rules.keys()):
        if name not in guides:
            failures.append(f"{name}: docs/contributing/{name} does not exist")
        if name not in table:
            failures.append(f"{name}: {RULES}/{name} has no row in {README}")
        elif name not in rules:
            failures.append(f"{name}: the {README} row has no {RULES}/{name}")
        elif not rules[name]:
            failures.append(f'{name}: {RULES}/{name} has no "quoted" glob under `paths:`')
        elif table[name] != rules[name]:
            only_readme = ", ".join(sorted(table[name] - rules[name])) or "nothing"
            only_rule = ", ".join(sorted(rules[name] - table[name])) or "nothing"
            failures.append(
                f"{name}: only in {README}: {only_readme}; only in {RULES}/{name}: {only_rule}"
            )
    return failures


def main() -> int:
    try:
        table = table_globs((ROOT / README).read_text(encoding="utf-8"))
    except ValueError as error:
        print(f"agent-rule-paths: {error}", file=sys.stderr)
        return 1
    rules = {
        path.name: rule_globs(path.read_text(encoding="utf-8"))
        for path in (ROOT / RULES).glob("*.md")
    }
    guides = {path.name for path in (ROOT / README).parent.glob("*.md")}
    failures = differences(table, rules, guides)
    if not failures:
        print(f"✓ {len(table)} guides: {README} and {RULES}/ name the same paths")
        return 0
    print("\n".join(failures), file=sys.stderr)
    print(
        f"agent-rule-paths: give the guide's row in {README} § {SECTION}"
        f" and the `paths:` frontmatter of its pointer in {RULES}/ the same globs —"
        " the README row is what agents other than Claude Code read.",
        file=sys.stderr,
    )
    return 1


if __name__ == "__main__":
    sys.exit(main())
