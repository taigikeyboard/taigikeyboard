"""Unit tests for the INVARIANT label gate."""

from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock

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

    def _init_repo(self, repo: Path) -> Path:
        self._git(repo, "init", "-q")
        (repo / ".gitignore").write_text("engine/target/\n", encoding="utf-8")
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
        return deleted

    def test_plain_git_skips_deleted_and_untracked_and_never_invokes_jj(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            deleted = self._init_repo(repo)
            deleted.unlink()
            (repo / "ios" / "TaigiKeyboardTests").mkdir(parents=True)
            (repo / "ios" / "TaigiKeyboardTests" / "UntrackedTests.swift").write_text(
                "// INVARIANT_untracked_rule\n", encoding="utf-8"
            )

            real_run = subprocess.run
            calls = []

            def spy_run(cmd, *args, **kwargs):
                calls.append(cmd[0])
                return real_run(cmd, *args, **kwargs)

            with mock.patch.object(invariant_labels.subprocess, "run", side_effect=spy_run):
                mentions = invariant_labels.test_mentions(repo)

            self.assertNotIn("jj", calls)
            self.assertEqual(mentions, {"committed_rule"})

    def test_jj_repo_uses_jj_file_list_and_falls_back_when_jj_missing(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            deleted = self._init_repo(repo)
            (repo / ".jj").mkdir()
            deleted.unlink()
            (repo / "ios" / "TaigiKeyboardTests").mkdir(parents=True)
            (repo / "ios" / "TaigiKeyboardTests" / "NewTests.swift").write_text(
                "// INVARIANT_new_jj_rule\n", encoding="utf-8"
            )

            real_run = subprocess.run

            def fake_jj(cmd, *args, **kwargs):
                if cmd[0] == "jj":
                    self.assertEqual(
                        cmd,
                        [
                            "jj",
                            "--no-pager",
                            "--color=never",
                            "--quiet",
                            "-R",
                            str(repo),
                            "file",
                            "list",
                            "-T",
                            r'path ++ "\0"',
                            "--",
                            *(f"root-glob:{d}/**" for d in invariant_labels.TEST_ROOTS),
                        ],
                    )
                    return subprocess.CompletedProcess(
                        cmd,
                        0,
                        stdout="engine/phonetics/tests/a.rs\0android/app/src/test/java/DeletedTest.kt\0ios/TaigiKeyboardTests/NewTests.swift\0",
                        stderr="",
                    )
                return real_run(cmd, *args, **kwargs)

            with mock.patch.object(invariant_labels.subprocess, "run", side_effect=fake_jj):
                self.assertEqual(
                    invariant_labels.test_mentions(repo),
                    {"committed_rule", "new_jj_rule"},
                )

            def missing_jj(cmd, *args, **kwargs):
                if cmd[0] == "jj":
                    raise FileNotFoundError("jj")
                return real_run(cmd, *args, **kwargs)

            with mock.patch.object(invariant_labels.subprocess, "run", side_effect=missing_jj):
                self.assertEqual(invariant_labels.test_mentions(repo), {"committed_rule"})

    @unittest.skipUnless(shutil.which("jj"), "jj not installed")
    def test_real_jj_reads_new_tests_and_respects_auto_track(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            repo = root / "repo"
            cfg = root / "jj.toml"
            cfg.write_text('[user]\nname = "Test"\nemail = "test@example.com"\n', encoding="utf-8")
            with mock.patch.dict(os.environ, {"JJ_CONFIG": str(cfg)}):
                subprocess.run(
                    ["jj", "git", "init", "--colocate", str(repo)],
                    check=True,
                    capture_output=True,
                    text=True,
                )
                (repo / ".gitignore").write_text("engine/target/\n", encoding="utf-8")
                (repo / "engine" / "phonetics" / "tests").mkdir(parents=True)
                (repo / "android" / "app" / "src" / "test" / "java").mkdir(parents=True)
                (repo / "engine" / "phonetics" / "tests" / "a.rs").write_text(
                    "// INVARIANT_committed_rule\n", encoding="utf-8"
                )
                deleted = repo / "android" / "app" / "src" / "test" / "java" / "DeletedTest.kt"
                deleted.write_text("// INVARIANT_deleted_rule\n", encoding="utf-8")
                subprocess.run(
                    ["jj", "-R", str(repo), "commit", "-m", "init"],
                    check=True,
                    capture_output=True,
                    text=True,
                )

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

                cfg.write_text(
                    '[user]\nname = "Test"\nemail = "test@example.com"\n[snapshot]\nauto-track = "none()"\n',
                    encoding="utf-8",
                )
                (repo / "ios" / "TaigiKeyboardTests" / "UntrackedTests.swift").write_text(
                    "// INVARIANT_untracked_rule\n", encoding="utf-8"
                )
                self.assertNotIn("untracked_rule", invariant_labels.test_mentions(repo))


if __name__ == "__main__":
    unittest.main()
