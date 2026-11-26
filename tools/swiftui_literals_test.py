"""Unit tests for the SwiftUI string-literal gate."""

from __future__ import annotations

import subprocess
import tempfile
import unittest
from pathlib import Path

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

    def test_tracks_new_files_and_skips_deleted_ignored_and_jj_files(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            self._git(repo, "init", "-q")
            (repo / ".gitignore").write_text("ios/Ignored/\n*.generated.swift\n", encoding="utf-8")
            (repo / ".jj").mkdir()
            (repo / ".jj" / ".gitignore").write_text("/*\n", encoding="utf-8")
            (repo / ".jj" / "Ignored.swift").write_text("// jj\n", encoding="utf-8")
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

            # Simulate jj @ edits without staging in Git's index:
            # delete a tracked file, add files in a brand-new subdir, and add ignored files.
            (repo / "ios" / "Sources" / "Deleted.swift").unlink()
            (repo / "ios" / "NewFeature").mkdir(parents=True)
            (repo / "ios" / "NewFeature" / "Added.swift").write_text("let d = 4\n", encoding="utf-8")
            (repo / "macos" / "NewMac.swift").write_text("let e = 5\n", encoding="utf-8")
            (repo / "ios" / "Ignored").mkdir(parents=True)
            (repo / "ios" / "Ignored" / "Skip.swift").write_text("let f = 6\n", encoding="utf-8")
            (repo / "ios" / "Sources" / "Auto.generated.swift").write_text(
                "let g = 7\n", encoding="utf-8"
            )

            self.assertEqual(
                swiftui_literals.swift_files(repo),
                [
                    "ios/NewFeature/Added.swift",
                    "ios/Sources/Kept.swift",
                    "macos/NewMac.swift",
                    "macos/Sources/Mac.swift",
                ],
            )


if __name__ == "__main__":
    unittest.main()
