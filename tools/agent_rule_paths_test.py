"""Unit tests for the `.claude/rules` ↔ contributing README path gate."""

from __future__ import annotations

import unittest

import agent_rule_paths

README = """# Contributing guides

| Guide | Covers |
|---|---|
| [`i18n.md`](i18n.md) | Shared `i18n/*.json` keys |

## Which guide for which path

Read a guide before your first edit under its paths.

| Guide | Paths |
|---|---|
| [`i18n.md`](i18n.md) | `i18n/**` |
| [`linux-guidelines.md`](linux-guidelines.md) | `linux/**`, `desktop/**` |

## Later section

| [`web-guidelines.md`](web-guidelines.md) | `web/**` |
"""


class TableGlobsTests(unittest.TestCase):
    def test_reads_only_the_table_under_its_heading(self) -> None:
        self.assertEqual(
            agent_rule_paths.table_globs(README),
            {"i18n.md": {"i18n/**"}, "linux-guidelines.md": {"linux/**", "desktop/**"}},
        )

    def test_a_missing_heading_is_an_error(self) -> None:
        with self.assertRaisesRegex(ValueError, "has no `## Which guide for which path`"):
            agent_rule_paths.table_globs("# Contributing guides\n")

    def test_a_deeper_heading_is_not_the_section_but_ends_it(self) -> None:
        with self.assertRaisesRegex(ValueError, "has no `## Which guide for which path`"):
            agent_rule_paths.table_globs(README.replace("## Which", "### Which"))

        nested = README.replace("## Later section", "### Later section")
        self.assertNotIn("web-guidelines.md", agent_rule_paths.table_globs(nested))

    def test_a_row_that_is_not_two_cells_is_an_error(self) -> None:
        piped = README.replace("`i18n/**` |", r"`i18n/{a\|b}` |")

        with self.assertRaisesRegex(ValueError, "is not two cells"):
            agent_rule_paths.table_globs(piped)

    def test_a_guide_listed_twice_is_an_error(self) -> None:
        twice = README.replace("linux-guidelines.md", "i18n.md")

        with self.assertRaisesRegex(ValueError, "lists i18n.md twice"):
            agent_rule_paths.table_globs(twice)


class RuleGlobsTests(unittest.TestCase):
    def test_reads_an_inline_list(self) -> None:
        rule = '---\npaths: ["linux/**", "desktop/**"]\n---\n\n# Linux\n'

        self.assertEqual(agent_rule_paths.rule_globs(rule), {"linux/**", "desktop/**"})

    def test_reads_a_block_list(self) -> None:
        rule = '---\npaths:\n  - "engine/**"\n  - "dictionary/**"\n---\n\n# Phonetics\n'

        self.assertEqual(agent_rule_paths.rule_globs(rule), {"engine/**", "dictionary/**"})

    def test_a_comment_is_not_a_glob(self) -> None:
        rule = '---\npaths:\n  - "engine/**"  # was "dictionary/**"\n---\n'

        self.assertEqual(agent_rule_paths.rule_globs(rule), {"engine/**"})

    def test_ignores_other_keys_and_the_body(self) -> None:
        rule = '---\npaths: ["web/**"]\ndescription: "not a glob"\n---\n\nSee "docs/**".\n'

        self.assertEqual(agent_rule_paths.rule_globs(rule), {"web/**"})

    def test_no_frontmatter_or_unquoted_globs_read_as_empty(self) -> None:
        self.assertEqual(agent_rule_paths.rule_globs("# Always-on rule\n"), set())
        self.assertEqual(agent_rule_paths.rule_globs("---\npaths: [web/**]\n---\n"), set())


class DifferencesTests(unittest.TestCase):
    GUIDES = {"i18n.md", "web-guidelines.md"}

    def test_equal_lists_pass(self) -> None:
        both = {"i18n.md": {"i18n/**"}}

        self.assertEqual(agent_rule_paths.differences(both, both, self.GUIDES), [])

    def test_names_the_globs_each_side_lacks(self) -> None:
        failures = agent_rule_paths.differences(
            {"i18n.md": {"i18n/**", "tools/i18n/**"}}, {"i18n.md": {"i18n/**", "ios/**"}}, self.GUIDES
        )

        self.assertEqual(
            failures,
            [
                "i18n.md: only in docs/contributing/README.md: tools/i18n/**;"
                " only in .claude/rules/i18n.md: ios/**"
            ],
        )

    def test_a_guide_on_one_side_only_fails(self) -> None:
        failures = agent_rule_paths.differences(
            {"i18n.md": {"i18n/**"}}, {"web-guidelines.md": {"web/**"}}, self.GUIDES
        )

        self.assertEqual(
            failures,
            [
                "i18n.md: the docs/contributing/README.md row has no .claude/rules/i18n.md",
                "web-guidelines.md: .claude/rules/web-guidelines.md has no row in"
                " docs/contributing/README.md",
            ],
        )

    def test_a_rule_without_globs_or_without_a_guide_fails(self) -> None:
        failures = agent_rule_paths.differences(
            {"i18n.md": {"i18n/**"}, "gone.md": {"x/**"}},
            {"i18n.md": set(), "gone.md": {"x/**"}},
            self.GUIDES,
        )

        self.assertEqual(
            failures,
            [
                "gone.md: docs/contributing/gone.md does not exist",
                'i18n.md: .claude/rules/i18n.md has no "quoted" glob under `paths:`',
            ],
        )


if __name__ == "__main__":
    unittest.main()
