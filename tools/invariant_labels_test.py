"""Unit tests for the INVARIANT label gate."""

from __future__ import annotations

import subprocess
import tempfile
import unittest
from pathlib import Path

import invariant_labels
from invariant_labels import Label


class LabelsInDocTests(unittest.TestCase):
    def test_reads_plain_and_family_labels_once_each(self) -> None:
        text = (
            "- `INVARIANT_tl_to_poj_roundtrip`\n"
            "### `INVARIANT_NASAL_MARKER_CASE`\n"
            "`INVARIANT_keyboard_popup_hide_*` and `INVARIANT_display_dedup_`\n"
            "again `INVARIANT_tl_to_poj_roundtrip`\n"
        )

        labels = invariant_labels.labels_in("doc.md", text)

        self.assertEqual(
            labels,
            [
                Label("tl_to_poj_roundtrip", False, "doc.md", 1),
                Label("nasal_marker_case", False, "doc.md", 2),
                Label("keyboard_popup_hide", True, "doc.md", 3),
                Label("display_dedup", True, "doc.md", 3),
            ],
        )

    def test_ignores_labels_outside_backticks(self) -> None:
        self.assertEqual(
            invariant_labels.labels_in("doc.md", "retired INVARIANT_old_rule\n"), []
        )


class TestCodeTests(unittest.TestCase):
    def test_whole_file_under_a_test_directory(self) -> None:
        for rel in (
            "engine/composing/tests/a.rs",
            "ios/TaigiKeyboardTests/A.swift",
            "macos/Tests/Core/A.swift",
            "android/app/src/test/java/A.kt",
        ):
            with self.subTest(rel=rel):
                self.assertEqual(invariant_labels.test_code(rel, "body"), "body")

    def test_rust_source_counts_from_its_test_module(self) -> None:
        text = "// INVARIANT_prod_comment\n#[cfg(test)]\nmod tests { // INVARIANT_pinned\n}"

        code = invariant_labels.test_code("engine/phonetics/src/tl.rs", text)

        self.assertNotIn("prod_comment", code)
        self.assertIn("INVARIANT_pinned", code)

    def test_production_source_is_not_test_code(self) -> None:
        self.assertEqual(
            invariant_labels.test_code(
                "android/app/src/main/java/A.kt", "// Pins INVARIANT_x"
            ),
            "",
        )


class CoverageTests(unittest.TestCase):
    NAMES = frozenset(
        {
            "tl_to_poj_roundtrip_is_lossless",
            "keyboard_popup_hide_clears_anchor",
            "case_rule",
        }
    )

    def test_exact_name_or_case_suffix_covers_a_label(self) -> None:
        self.assertTrue(
            invariant_labels.is_covered(Label("case_rule", False, "d", 1), self.NAMES)
        )
        self.assertTrue(
            invariant_labels.is_covered(
                Label("tl_to_poj_roundtrip", False, "d", 1), self.NAMES
            )
        )

    def test_a_longer_word_does_not_cover_a_label(self) -> None:
        self.assertFalse(
            invariant_labels.is_covered(Label("case_ru", False, "d", 1), self.NAMES)
        )

    def test_family_matches_any_prefix(self) -> None:
        self.assertTrue(
            invariant_labels.is_covered(
                Label("keyboard_popup_hide", True, "d", 1), self.NAMES
            )
        )
        self.assertFalse(
            invariant_labels.is_covered(
                Label("display_dedup", True, "d", 1), self.NAMES
            )
        )


class PendingBaselineTests(unittest.TestCase):
    def test_reads_entries_and_drops_comments_and_blanks(self) -> None:
        text = "# header\n\nfoo_rule  # why\nkeyboard_*  # family\n"

        self.assertEqual(
            invariant_labels.read_pending(text), {"foo_rule", "keyboard_*"}
        )

    def test_a_missing_label_outside_the_baseline_is_new(self) -> None:
        missing = [Label("foo_rule", False, "d", 1), Label("bar_rule", False, "d", 2)]

        new, stale = invariant_labels.check(missing, {"foo_rule"})

        self.assertEqual(new, [Label("bar_rule", False, "d", 2)])
        self.assertEqual(stale, [])

    def test_a_baseline_entry_whose_label_got_a_test_is_stale(self) -> None:
        missing = [Label("keyboard", True, "d", 1)]

        new, stale = invariant_labels.check(missing, {"keyboard_*", "foo_rule"})

        self.assertEqual(new, [])
        self.assertEqual(stale, ["foo_rule"])


class TestMentionsWorkingTreeTests(unittest.TestCase):
    def _git(self, repo: Path, *args: str) -> None:
        subprocess.run(
            [
                "git",
                "-C",
                str(repo),
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.com",
                "-c",
                "commit.gpgsign=false",
                *args,
            ],
            check=True,
            capture_output=True,
            text=True,
        )

    def test_reads_new_untracked_tests_and_skips_deleted_or_ignored_files(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            self._git(repo, "init", "-q")
            (repo / ".gitignore").write_text("engine/target/\n", encoding="utf-8")
            (repo / ".jj").mkdir()
            (repo / ".jj" / ".gitignore").write_text("/*\n", encoding="utf-8")
            (repo / "engine" / "phonetics" / "tests").mkdir(parents=True)
            (repo / "android" / "app" / "src" / "test" / "java").mkdir(parents=True)

            (repo / "engine" / "phonetics" / "tests" / "a.rs").write_text(
                "// INVARIANT_committed_rule\n", encoding="utf-8"
            )
            deleted = repo / "android" / "app" / "src" / "test" / "java" / "DeletedTest.kt"
            deleted.write_text("// INVARIANT_deleted_rule\n", encoding="utf-8")
            self._git(
                repo,
                "add",
                ".gitignore",
                "engine/phonetics/tests/a.rs",
                "android/app/src/test/java/DeletedTest.kt",
            )
            self._git(repo, "commit", "-q", "-m", "init")

            # Simulate jj @ edits: remove tracked file, add new untracked test files, add ignored file.
            deleted.unlink()
            (repo / "ios" / "TaigiKeyboardTests").mkdir(parents=True)
            (repo / "ios" / "TaigiKeyboardTests" / "NewTests.swift").write_text(
                "// INVARIANT_new_jj_rule\n", encoding="utf-8"
            )
            (repo / "macos" / "Tests" / "NewSuite").mkdir(parents=True)
            (repo / "macos" / "Tests" / "NewSuite" / "CaseTests.swift").write_text(
                "// INVARIANT_new_macos_rule\n", encoding="utf-8"
            )
            (repo / "engine" / "target" / "debug" / "tests").mkdir(parents=True)
            (repo / "engine" / "target" / "debug" / "tests" / "ignored.rs").write_text(
                "// INVARIANT_ignored_rule\n", encoding="utf-8"
            )

            self.assertEqual(
                invariant_labels.test_mentions(repo),
                {"committed_rule", "new_jj_rule", "new_macos_rule"},
            )


if __name__ == "__main__":
    unittest.main()
