"""Unit tests for release-note validation and source synchronization."""

from __future__ import annotations

import plistlib
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

    def test_rejects_macos_mention_on_both_mobile_platforms(self) -> None:
        for platform, entry in (
            ("ios", "New: macOS candidate window shows selection keys."),
            ("android", "New: the Mac app can now check for updates."),
        ):
            with self.subTest(platform=platform):
                notes = release_notes.PlatformNotes(
                    platform=platform, entries=(entry,)
                )

                with self.assertRaisesRegex(
                    release_notes.ReleaseNotesError, "must not mention"
                ):
                    release_notes.validate_notes(notes)

    def test_accepts_ordinary_words_that_begin_with_mac(self) -> None:
        notes = release_notes.PlatformNotes(
            platform="ios",
            entries=(
                "Fixed: a macro no longer breaks tone marks on any machine.",
            ),
        )

        release_notes.validate_notes(notes)

    def test_rejects_plurals_and_compounds_of_forbidden_terms(self) -> None:
        for platform, entry in (
            ("ios", "Changed: Androids now share the same candidate order."),
            ("android", "New: MacBooks can run the same dictionary."),
            ("android", "New: macOSX support."),
            ("ios", "New: Macs share the user dictionary."),
        ):
            with self.subTest(entry=entry):
                notes = release_notes.PlatformNotes(
                    platform=platform, entries=(entry,)
                )

                with self.assertRaisesRegex(
                    release_notes.ReleaseNotesError, "must not mention"
                ):
                    release_notes.validate_notes(notes)


class MacOSVersionGateTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp_dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp_dir.cleanup)
        self.repo_root = Path(self.temp_dir.name)
        self.plist_path = self.repo_root / "macos/App/Info.plist"
        self.plist_path.parent.mkdir(parents=True)

    def write_plist(self, short_version: str, build_version: str) -> None:
        self.plist_path.write_bytes(
            plistlib.dumps(
                {
                    "CFBundleShortVersionString": short_version,
                    "CFBundleVersion": build_version,
                },
            ),
        )

    def test_derives_build_version_from_marketing_version(self) -> None:
        self.assertEqual(release_notes.macos_build_version("3.6.5"), "30605")
        self.assertEqual(release_notes.macos_build_version("3.10.12"), "31012")

    def test_rejects_components_the_derivation_cannot_encode(self) -> None:
        # 3.1.100 and 3.2.0 would both derive to 30605-style collisions.
        for version in ("3.1.100", "3.100.0"):
            with self.subTest(version=version):
                with self.assertRaisesRegex(
                    release_notes.ReleaseNotesError, "would collide"
                ):
                    release_notes.macos_build_version(version)

    def test_rejects_a_version_that_is_not_three_components(self) -> None:
        with self.assertRaisesRegex(
            release_notes.ReleaseNotesError, "MAJOR.MINOR.PATCH"
        ):
            release_notes.macos_build_version("3.6")

    def test_accepts_matching_versions(self) -> None:
        self.write_plist("3.6.5", "30605")

        release_notes.check_macos_version(self.repo_root, "3.6.5")

    def test_rejects_stale_marketing_version(self) -> None:
        self.write_plist("3.6.4", "30604")

        with self.assertRaisesRegex(
            release_notes.ReleaseNotesError, "CFBundleShortVersionString is 3.6.4"
        ):
            release_notes.check_macos_version(self.repo_root, "3.6.5")

    def test_rejects_build_version_that_does_not_derive(self) -> None:
        self.write_plist("3.6.5", "30604")

        with self.assertRaisesRegex(
            release_notes.ReleaseNotesError, "CFBundleVersion is 30604"
        ):
            release_notes.check_macos_version(self.repo_root, "3.6.5")

    def test_reports_missing_plist(self) -> None:
        with self.assertRaisesRegex(
            release_notes.ReleaseNotesError, "missing macOS Info.plist"
        ):
            release_notes.check_macos_version(self.repo_root, "3.6.5")

    def test_reports_a_plist_that_is_not_a_dictionary(self) -> None:
        self.plist_path.write_bytes(plistlib.dumps(["3.6.5"]))

        with self.assertRaisesRegex(
            release_notes.ReleaseNotesError, "does not contain a dictionary"
        ):
            release_notes.check_macos_version(self.repo_root, "3.6.5")

    def test_reports_an_unreadable_plist(self) -> None:
        self.plist_path.write_bytes(b"not a plist at all")

        with self.assertRaisesRegex(
            release_notes.ReleaseNotesError, "is not a readable plist"
        ):
            release_notes.check_macos_version(self.repo_root, "3.6.5")

    def test_reports_missing_keys(self) -> None:
        self.plist_path.write_bytes(plistlib.dumps({"CFBundleName": "TaigiKeyboard"}))

        with self.assertRaisesRegex(
            release_notes.ReleaseNotesError, "CFBundleShortVersionString is missing"
        ):
            release_notes.check_macos_version(self.repo_root, "3.6.5")

    def test_accepts_a_binary_plist(self) -> None:
        self.plist_path.write_bytes(
            plistlib.dumps(
                {
                    "CFBundleShortVersionString": "3.6.5",
                    "CFBundleVersion": "30605",
                },
                fmt=plistlib.FMT_BINARY,
            ),
        )

        release_notes.check_macos_version(self.repo_root, "3.6.5")


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
