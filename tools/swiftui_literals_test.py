"""Unit tests for the SwiftUI string-literal gate."""

from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import swiftui_literals


class FindingsInTests(unittest.TestCase):
    def test_flags_a_literal_title_on_views_and_modifiers(self) -> None:
        source = (
            'Text("Hello")\n'
            'Button("Save") { save() }\n'
            'list.navigationTitle("Settings")\n'
        )

        self.assertEqual(
            swiftui_literals.findings_in(source),
            [(1, 'Text("'), (2, 'Button("'), (3, '.navigationTitle("')],
        )

    def test_flags_a_literal_on_the_line_after_the_parenthesis(self) -> None:
        self.assertEqual(
            swiftui_literals.findings_in('view\n    .confirmationDialog(\n        "Delete?",\n'),
            [(2, '.confirmationDialog( "')],
        )

    def test_flags_interpolation_only_and_multiline_literals(self) -> None:
        self.assertEqual(
            swiftui_literals.findings_in(
                'Text("\\(count)")\nText("""\n  long\n  """)\nText(#"raw"#)\n'
            ),
            [(1, 'Text("'), (2, 'Text("'), (5, 'Text(#"')],
        )

    def test_flags_explicit_localizing_apis(self) -> None:
        source = (
            "let key: LocalizedStringKey = title\n"
            'let a = String(localized: "x")\n'
            'let b = NSLocalizedString("x", comment: "")\n'
        )

        self.assertEqual(
            [line for line, _ in swiftui_literals.findings_in(source)], [1, 2, 3]
        )

    def test_allows_empty_titles_verbatim_text_and_generated_strings(self) -> None:
        source = (
            'Toggle("", isOn: $isOn)\n'
            'Picker(#""#, selection: $kind)\n'
            'Text(verbatim: "•")\n'
            "Text(lang.string(.dictionaryNoResults))\n"
            "Text(title)\n"
            'Image("keyboard-preview")\n'
            'proxy.setMarkedText("", selectedRange: range)\n'
            'contextText("abc")\n'
        )

        self.assertEqual(swiftui_literals.findings_in(source), [])


class SwiftFilesTests(unittest.TestCase):
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

    def _init_swift_repo(self, repo: Path) -> None:
        self._git(repo, "init", "-q")
        (repo / ".gitignore").write_text("ios/Ignored/\n*.generated.swift\n", encoding="utf-8")
        (repo / "ios" / "Sources").mkdir(parents=True)
        (repo / "macos" / "Sources").mkdir(parents=True)
        (repo / "ios" / "Sources" / "Kept.swift").write_text("let a = 1\n", encoding="utf-8")
        (repo / "ios" / "Sources" / "Deleted.swift").write_text("let b = 2\n", encoding="utf-8")
        (repo / "macos" / "Sources" / "Mac.swift").write_text("let c = 3\n", encoding="utf-8")
        self._git(
            repo,
            "add",
            ".gitignore",
            "ios/Sources/Kept.swift",
            "ios/Sources/Deleted.swift",
            "macos/Sources/Mac.swift",
        )
        self._git(repo, "commit", "-q", "-m", "init")

    def test_plain_git_skips_deleted_and_untracked_and_never_invokes_jj(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            self._init_swift_repo(repo)
            (repo / "ios" / "Sources" / "Deleted.swift").unlink()
            (repo / "ios" / "Untracked.swift").write_text("let x = 0\n", encoding="utf-8")

            real_run = subprocess.run
            calls = []

            def spy_run(cmd, *args, **kwargs):
                calls.append(cmd[0])
                return real_run(cmd, *args, **kwargs)

            with mock.patch.object(swiftui_literals.subprocess, "run", side_effect=spy_run):
                files = swiftui_literals.swift_files(repo)

            self.assertNotIn("jj", calls)
            self.assertEqual(
                files,
                ["ios/Sources/Kept.swift", "macos/Sources/Mac.swift"],
            )

    def test_jj_repo_uses_jj_file_list_and_falls_back_when_jj_missing(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            self._init_swift_repo(repo)
            (repo / ".jj").mkdir()
            (repo / "ios" / "Sources" / "Deleted.swift").unlink()
            (repo / "ios" / "NewFeature").mkdir(parents=True)
            (repo / "ios" / "NewFeature" / "Added.swift").write_text("let d = 4\n", encoding="utf-8")

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
                            "root-glob:ios/**/*.swift",
                            "root-glob:macos/**/*.swift",
                        ],
                    )
                    return subprocess.CompletedProcess(
                        cmd,
                        0,
                        stdout="ios/Sources/Kept.swift\0ios/Sources/Deleted.swift\0ios/NewFeature/Added.swift\0macos/Sources/Mac.swift\0",
                        stderr="",
                    )
                return real_run(cmd, *args, **kwargs)

            with mock.patch.object(swiftui_literals.subprocess, "run", side_effect=fake_jj):
                self.assertEqual(
                    swiftui_literals.swift_files(repo),
                    [
                        "ios/NewFeature/Added.swift",
                        "ios/Sources/Kept.swift",
                        "macos/Sources/Mac.swift",
                    ],
                )

            def missing_jj(cmd, *args, **kwargs):
                if cmd[0] == "jj":
                    raise FileNotFoundError("jj")
                return real_run(cmd, *args, **kwargs)

            with mock.patch.object(swiftui_literals.subprocess, "run", side_effect=missing_jj):
                self.assertEqual(
                    swiftui_literals.swift_files(repo),
                    ["ios/Sources/Kept.swift", "macos/Sources/Mac.swift"],
                )

    @unittest.skipUnless(shutil.which("jj"), "jj not installed")
    def test_real_jj_tracks_new_swift_files_and_respects_auto_track(self) -> None:
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
                (repo / ".gitignore").write_text("ios/Ignored/\n*.generated.swift\n", encoding="utf-8")
                (repo / "ios" / "Sources").mkdir(parents=True)
                (repo / "macos" / "Sources").mkdir(parents=True)
                (repo / "ios" / "Sources" / "Kept.swift").write_text("let a = 1\n", encoding="utf-8")
                (repo / "ios" / "Sources" / "Deleted.swift").write_text("let b = 2\n", encoding="utf-8")
                (repo / "macos" / "Sources" / "Mac.swift").write_text("let c = 3\n", encoding="utf-8")
                subprocess.run(
                    ["jj", "-R", str(repo), "commit", "-m", "init"],
                    check=True,
                    capture_output=True,
                    text=True,
                )

                (repo / "ios" / "Sources" / "Deleted.swift").unlink()
                (repo / "ios" / "Top.swift").write_text("let top = 0\n", encoding="utf-8")
                (repo / "ios" / "NewFeature").mkdir(parents=True)
                (repo / "ios" / "NewFeature" / "Added.swift").write_text("let d = 4\n", encoding="utf-8")
                (repo / "macos" / "NewMac.swift").write_text("let e = 5\n", encoding="utf-8")
                (repo / "ios" / "Ignored").mkdir(parents=True)
                (repo / "ios" / "Ignored" / "Skip.swift").write_text("let f = 6\n", encoding="utf-8")
                (repo / "ios" / "Sources" / "Auto.generated.swift").write_text("let g = 7\n", encoding="utf-8")

                self.assertEqual(
                    swiftui_literals.swift_files(repo),
                    [
                        "ios/NewFeature/Added.swift",
                        "ios/Sources/Kept.swift",
                        "ios/Top.swift",
                        "macos/NewMac.swift",
                        "macos/Sources/Mac.swift",
                    ],
                )

                cfg.write_text(
                    '[user]\nname = "Test"\nemail = "test@example.com"\n[snapshot]\nauto-track = "none()"\n',
                    encoding="utf-8",
                )
                (repo / "ios" / "Untracked.swift").write_text("let u = 8\n", encoding="utf-8")
                self.assertNotIn("ios/Untracked.swift", swiftui_literals.swift_files(repo))


if __name__ == "__main__":
    unittest.main()
