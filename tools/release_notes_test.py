"""Unit tests for release-note validation and source synchronization."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

import release_notes


class ReleaseNotesValidationTests(unittest.TestCase):
    def test_accepts_concise_platform_specific_notes(self) -> None:
        notes = release_notes.PlatformNotes(
            platform="ios",
            entries=("Fixed: candidate selection is now more reliable.",),
        )

        release_notes.validate_notes(notes)

        self.assertEqual(
            notes.store_text,
            "• Fixed: candidate selection is now more reliable.",
        )

    def test_rejects_other_platform_name(self) -> None:
        notes = release_notes.PlatformNotes(
            platform="ios",
            entries=("Changed: Android and iOS now behave consistently.",),
        )

        with self.assertRaisesRegex(
            release_notes.ReleaseNotesError, "must not mention"
        ):
            release_notes.validate_notes(notes)

    def test_counts_rendered_bullets_against_google_limit(self) -> None:
        notes = release_notes.PlatformNotes(
            platform="android",
            entries=(f"Fixed: {'x' * 491}.",),
        )

        with self.assertRaisesRegex(release_notes.ReleaseNotesError, "maximum is 500"):
            release_notes.validate_notes(notes)


class VersionHistorySyncTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp_dir = tempfile.TemporaryDirectory()
        self.repo_root = Path(self.temp_dir.name)
        ios_path = self.repo_root / "ios/Sources/TaigiKeyboard/App/Tabs/Home"
        android_path = (
            self.repo_root
            / "android/app/src/main/java/com/siansiansu/taigikeyboard/content"
        )
        notes_path = self.repo_root / "changelog/store/v3.7.0"
        ios_path.mkdir(parents=True)
        android_path.mkdir(parents=True)
        notes_path.mkdir(parents=True)
        (notes_path / "ios.txt").write_text(
            "New: candidate navigation is easier to use.\n",
            encoding="utf-8",
        )
        (notes_path / "android.txt").write_text(
            "Fixed: candidate navigation responds consistently.\n",
            encoding="utf-8",
        )
        (ios_path / "VersionHistory.swift").write_text(
            "enum VersionHistory {\n"
            "    static let entries: [(version: String, date: String, changes: [String])] = [\n"
            '        ("3.6.4", "2026/08/07", [\n'
            '            "Old entry.",\n'
            "        ]),\n"
            "    ]\n"
            "}\n",
            encoding="utf-8",
        )
        (android_path / "VersionHistory.kt").write_text(
            "object VersionHistory {\n"
            "    val entries =\n"
            "        listOf(\n"
            "            VersionEntry(\n"
            '                "3.6.4",\n'
            '                "2026/08/07",\n'
            "                listOf(\n"
            '                    "Old entry.",\n'
            "                ),\n"
            "            ),\n"
            "        )\n"
            "}\n",
            encoding="utf-8",
        )

    def tearDown(self) -> None:
        self.temp_dir.cleanup()

    def test_sync_inserts_canonical_notes_as_newest_entry_and_is_idempotent(
        self,
    ) -> None:
        release_notes.sync_version_history(self.repo_root, "3.7.0", "2026/08/08")
        first_swift = (
            self.repo_root
            / "ios/Sources/TaigiKeyboard/App/Tabs/Home/VersionHistory.swift"
        ).read_text(encoding="utf-8")
        first_kotlin = (
            self.repo_root
            / "android/app/src/main/java/com/siansiansu/taigikeyboard/content/VersionHistory.kt"
        ).read_text(encoding="utf-8")

        release_notes.sync_version_history(self.repo_root, "3.7.0", "2026/08/08")

        self.assertEqual(
            first_swift,
            (
                self.repo_root
                / "ios/Sources/TaigiKeyboard/App/Tabs/Home/VersionHistory.swift"
            ).read_text(encoding="utf-8"),
        )
        self.assertEqual(
            first_kotlin,
            (
                self.repo_root
                / "android/app/src/main/java/com/siansiansu/taigikeyboard/content/VersionHistory.kt"
            ).read_text(encoding="utf-8"),
        )
        self.assertIn('("3.7.0", "2026/08/08"', first_swift)
        self.assertIn('"New: candidate navigation is easier to use."', first_swift)
        self.assertIn(
            '"Fixed: candidate navigation responds consistently."', first_kotlin
        )
        release_notes.check_version_history(self.repo_root, "3.7.0")

    def test_sync_escapes_kotlin_string_interpolation(self) -> None:
        notes_path = self.repo_root / "changelog/store/v3.7.0/android.txt"
        notes_path.write_text(
            "Updated: Environment variables such as $HOME are displayed safely.\n",
            encoding="utf-8",
        )

        release_notes.sync_version_history(self.repo_root, "3.7.0", "2026/08/08")

        kotlin_source = (
            self.repo_root
            / "android/app/src/main/java/com/siansiansu/taigikeyboard/content/VersionHistory.kt"
        ).read_text(encoding="utf-8")
        self.assertIn(r"such as \$HOME are displayed safely", kotlin_source)
        release_notes.check_version_history(self.repo_root, "3.7.0")

    def test_malformed_kotlin_source_does_not_partially_update_swift(self) -> None:
        swift_path = (
            self.repo_root
            / "ios/Sources/TaigiKeyboard/App/Tabs/Home/VersionHistory.swift"
        )
        kotlin_path = (
            self.repo_root
            / "android/app/src/main/java/com/siansiansu/taigikeyboard/content/VersionHistory.kt"
        )
        original_swift = swift_path.read_text(encoding="utf-8")
        kotlin_path.write_text("object VersionHistory {}\n", encoding="utf-8")

        with self.assertRaisesRegex(
            release_notes.ReleaseNotesError, "list marker not found"
        ):
            release_notes.sync_version_history(self.repo_root, "3.7.0", "2026/08/08")

        self.assertEqual(original_swift, swift_path.read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()
