"""Unit tests for the doc-paths gate."""

from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import doc_paths

TOP_DIRS = {"engine", "docs", "android", "taigi-converter"}
TRACKED = {
    "engine/composing/src/lib.rs",
    "docs/engine/tone.md",
    "docs/a b.md",
    "android/app/src/main/java/x/engine/PhoneticsBridge.kt",
    "android/app/src/main/assets/ime/text/a.json",
    "android/app/src/main/assets/ime/text/b.json",
    "android/app/src/main/java/x/engine/Shared.kt",
    "desktop/src/engine/Shared.kt",
}
SUBMODULES = {"taigi-converter"}


def dead(doc: str, text: str, ignored: frozenset[str] = frozenset()) -> list[str]:
    refs = doc_paths.references(doc, text, TOP_DIRS)
    return doc_paths.dead_paths([(doc, refs)], TRACKED, set(ignored), SUBMODULES)


class ReferencesTests(unittest.TestCase):
    def test_strips_line_symbol_and_anchor_suffixes(self) -> None:
        text = (
            "`engine/composing/src/lib.rs:12` `engine/composing/src/lib.rs:L12`"
            " `engine/composing/src/lib.rs::fetch` `engine/composing/src/lib.rs#top`"
        )
        self.assertEqual(dead("AGENTS.md", text), [])

    def test_patterns_and_placeholders_are_not_paths(self) -> None:
        text = (
            "`engine/*/src` `engine/<crate>/src` `engine/x/vMAJOR.MINOR.PATCH.md`"
            " `engine/x/YYYY.M.P` `engine/a, engine/b` `engine/x.rs (old)` `foo/bar.rs`"
        )
        self.assertEqual(dead("AGENTS.md", text), [])

    def test_fenced_blocks_are_skipped_and_line_numbers_kept(self) -> None:
        text = "```\n`engine/gone.rs`\n```\n`engine/gone.rs`\n"
        self.assertEqual(dead("AGENTS.md", text), ["AGENTS.md:4: `engine/gone.rs` does not exist"])

    def test_indented_and_nested_fences_are_skipped(self) -> None:
        text = "  ```\n  [a](docs/gone.md)\n  ```\n````md\n```\n`engine/gone.rs`\n```\n````\n"
        self.assertEqual(dead("AGENTS.md", text), [])

    def test_a_url_link_text_names_another_repository(self) -> None:
        self.assertEqual(dead("AGENTS.md", "[`engine/gone.rs`](https://github.com/o/r/blob/main/engine/gone.rs)"), [])


class LinkTests(unittest.TestCase):
    def test_link_forms_resolve_from_the_doc(self) -> None:
        text = (
            '[a](tone.md#marks) [b](<tone.md>) [c](tone.md "Tone") [d](mailto:x@y) [e](#top)'
            " [f](../a%20b.md) [g](/docs/engine/tone.md) [h](../) [i](/)"
        )
        self.assertEqual(dead("docs/engine/x.md", text), [])

    def test_reference_definitions_are_checked(self) -> None:
        self.assertEqual(
            dead("docs/engine/x.md", "[tone]: ./gone.md"),
            ["docs/engine/x.md:1: `docs/engine/gone.md` does not exist"],
        )

    def test_a_link_out_of_the_repository_reports(self) -> None:
        self.assertEqual(dead("AGENTS.md", "[x](../outside.md)"), ["AGENTS.md:1: `../outside.md` does not exist"])

    def test_links_never_resolve_by_tail(self) -> None:
        self.assertEqual(dead("AGENTS.md", "[t](tone.md)"), ["AGENTS.md:1: `tone.md` does not exist"])


class DeadPathsTests(unittest.TestCase):
    def test_resolves_from_root_doc_directory_and_as_a_unique_tail(self) -> None:
        text = "`engine/composing` `engine/tone.md` `engine/PhoneticsBridge.kt`"
        self.assertEqual(dead("docs/README.md", text), [])

    def test_a_unique_directory_tail_resolves(self) -> None:
        refs = doc_paths.references("AGENTS.md", "`main/assets/ime/text/`", TOP_DIRS | {"main"})
        self.assertEqual(doc_paths.dead_paths([("AGENTS.md", refs)], TRACKED, set(), SUBMODULES), [])

    def test_an_ambiguous_tail_reports(self) -> None:
        self.assertEqual(dead("AGENTS.md", "`engine/Shared.kt`"), ["AGENTS.md:1: `engine/Shared.kt` does not exist"])

    def test_reports_a_moved_file(self) -> None:
        self.assertEqual(
            dead("AGENTS.md", "see `engine/composing/src/old.rs`"),
            ["AGENTS.md:1: `engine/composing/src/old.rs` does not exist"],
        )

    def test_ignored_and_submodule_paths_pass(self) -> None:
        text = "`engine/target/debug` `taigi-converter/src/index.js` [c](taigi-converter/README.md)"
        self.assertEqual(dead("AGENTS.md", text, frozenset({"engine/target/debug"})), [])


class IgnoredPathsTests(unittest.TestCase):
    def test_a_directory_only_rule_matches_an_absent_directory(self) -> None:
        # CI never builds the ignored output, so the directory is absent there.
        with tempfile.TemporaryDirectory() as repo:
            subprocess.run(["git", "init", "-q", repo], check=True)
            Path(repo, ".gitignore").write_text("build/out/\n", encoding="utf-8")
            self.assertEqual(
                doc_paths.ignored_paths({"build/out", "build/kept"}, Path(repo)), {"build/out"}
            )

    def test_no_paths_runs_no_git(self) -> None:
        self.assertEqual(doc_paths.ignored_paths(set(), Path("/nonexistent")), set())


class LivingDocTests(unittest.TestCase):
    def test_living_docs(self) -> None:
        for path in (
            "AGENTS.md",
            "CLAUDE.md",
            "docs/roadmap.md",
            "docs/architecture/system-overview.md",
            ".claude/skills/audit/SKILL.md",
            "engine/README.md",
            "emoji/AGENTS.md",
        ):
            self.assertTrue(doc_paths.is_living_doc(path), path)

    def test_skips_snapshots_references_design_records_and_vendored_code(self) -> None:
        for path in (
            "docs/reports/2026-11-13-audit-all.md",
            "docs/references/keywords.md",
            "docs/architecture/linux-roadmap.md",
            "docs/architecture/maintainability-roadmap-2026-10.md",
            "ios/Vendor/ISEmojiView/README.md",
            "engine/notes.md",
            "docs/engine/migration-inventory.csv",
        ):
            self.assertFalse(doc_paths.is_living_doc(path), path)


class WorkingTreePathsTests(unittest.TestCase):
    def _git(self, repo: Path, *args: str) -> str:
        return subprocess.run(
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
        ).stdout.strip()

    def _init_repo_with_submodule(self, repo: Path) -> None:
        self._git(repo, "init", "-q")
        (repo / ".gitignore").write_text("ignored/\n", encoding="utf-8")
        (repo / "docs").mkdir()
        (repo / "docs" / "kept.md").write_text("kept\n", encoding="utf-8")
        (repo / "docs" / "deleted.md").write_text("gone\n", encoding="utf-8")
        self._git(repo, "add", ".gitignore", "docs/kept.md", "docs/deleted.md")
        self._git(repo, "commit", "-q", "-m", "init")
        head_sha = self._git(repo, "rev-parse", "HEAD")
        self._git(
            repo,
            "update-index",
            "--add",
            "--cacheinfo",
            f"160000,{head_sha},taigi-converter",
        )

    def test_plain_git_tracks_submodules_skips_deleted_and_untracked_without_jj(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            self._init_repo_with_submodule(repo)
            (repo / "docs" / "deleted.md").unlink()
            (repo / "engine").mkdir()
            (repo / "engine" / "untracked.rs").write_text("fn f() {}\n", encoding="utf-8")
            (repo / "docs" / "untracked.md").write_text("untracked\n", encoding="utf-8")
            (repo / "ignored").mkdir()
            (repo / "ignored" / "skip.md").write_text("ignored\n", encoding="utf-8")

            real_run = subprocess.run
            calls = []

            def spy_run(cmd, *args, **kwargs):
                calls.append(cmd[0])
                return real_run(cmd, *args, **kwargs)

            with mock.patch.object(doc_paths.subprocess, "run", side_effect=spy_run):
                tracked, submodules = doc_paths.working_tree_paths(repo)

            self.assertNotIn("jj", calls)
            self.assertEqual(submodules, {"taigi-converter"})
            self.assertEqual(tracked, {".gitignore", "docs/kept.md", "taigi-converter"})

    def test_jj_repo_uses_jj_file_list_and_falls_back_when_jj_missing(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            self._init_repo_with_submodule(repo)
            (repo / ".jj").mkdir()
            (repo / "docs" / "deleted.md").unlink()
            (repo / "Makefile").write_text("all:\n", encoding="utf-8")

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
                            r'file_type ++ "\t" ++ path ++ "\0"',
                        ],
                    )
                    return subprocess.CompletedProcess(
                        cmd,
                        0,
                        stdout="file\tMakefile\0file\tdocs/deleted.md\0git-submodule\ttaigi-converter\0",
                        stderr="",
                    )
                return real_run(cmd, *args, **kwargs)

            with mock.patch.object(doc_paths.subprocess, "run", side_effect=fake_jj):
                tracked, submodules = doc_paths.working_tree_paths(repo)

            self.assertEqual(submodules, {"taigi-converter"})
            self.assertEqual(tracked, {"Makefile", "taigi-converter"})

            def missing_jj(cmd, *args, **kwargs):
                if cmd[0] == "jj":
                    raise FileNotFoundError("jj")
                return real_run(cmd, *args, **kwargs)

            with mock.patch.object(doc_paths.subprocess, "run", side_effect=missing_jj):
                tracked, submodules = doc_paths.working_tree_paths(repo)

            self.assertEqual(submodules, {"taigi-converter"})
            self.assertEqual(tracked, {".gitignore", "docs/kept.md", "taigi-converter"})

    @unittest.skipUnless(shutil.which("jj"), "jj not installed")
    def test_real_jj_tracks_new_files_and_respects_auto_track(self) -> None:
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
                (repo / ".gitignore").write_text("ignored/\n", encoding="utf-8")
                (repo / "docs").mkdir()
                (repo / "docs" / "kept.md").write_text("kept\n", encoding="utf-8")
                (repo / "docs" / "deleted.md").write_text("gone\n", encoding="utf-8")
                subprocess.run(
                    ["jj", "-R", str(repo), "commit", "-m", "init"],
                    check=True,
                    capture_output=True,
                    text=True,
                )

                (repo / "docs" / "deleted.md").unlink()
                (repo / "engine").mkdir()
                (repo / "engine" / "new_file.rs").write_text("fn f() {}\n", encoding="utf-8")
                (repo / "docs" / "new_doc.md").write_text(
                    "See `engine/new_file.rs` and [kept](kept.md).\n", encoding="utf-8"
                )
                (repo / "ignored").mkdir()
                (repo / "ignored" / "skip.md").write_text("ignored\n", encoding="utf-8")

                tracked, submodules = doc_paths.working_tree_paths(repo)
                self.assertEqual(submodules, set())
                self.assertEqual(
                    tracked,
                    {".gitignore", "docs/kept.md", "docs/new_doc.md", "engine/new_file.rs"},
                )

                cfg.write_text(
                    '[user]\nname = "Test"\nemail = "test@example.com"\n[snapshot]\nauto-track = "none()"\n',
                    encoding="utf-8",
                )
                (repo / "docs" / "untracked.md").write_text("untracked\n", encoding="utf-8")
                tracked_after, _ = doc_paths.working_tree_paths(repo)
                self.assertNotIn("docs/untracked.md", tracked_after)


if __name__ == "__main__":
    unittest.main()
