"""Unit tests for release-note validation and source synchronization."""

from __future__ import annotations

import plistlib
import stat
import tempfile
import unittest
import unittest.mock
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
        self.plist_path = self.repo_root / release_notes.MACOS_INFO_PLIST_FILE
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


GRADLE_FIXTURE = """android {
    defaultConfig {
        // versionCode = Unix epoch minutes; versionName stays SemVer.
        versionCode = (System.currentTimeMillis() / 60_000L).toInt()
        versionName = "3.6.6"
    }
}
"""

PBXPROJ_FIXTURE = """// !$*UTF8*$!
\t\t8A01 /* Debug */ = {
\t\t\tbuildSettings = {
\t\t\t\tCURRENT_PROJECT_VERSION = 1;
\t\t\t\tMARKETING_VERSION = 3.6.6;
\t\t\t\tPRODUCT_BUNDLE_IDENTIFIER = com.siansiansu.TaigiKeyboard;
\t\t\t};
\t\t};
\t\t8A02 /* Release */ = {
\t\t\tbuildSettings = {
\t\t\t\tCURRENT_PROJECT_VERSION = 1;
\t\t\t\tMARKETING_VERSION = 3.6.6;
\t\t\t\tPRODUCT_BUNDLE_IDENTIFIER = com.siansiansu.TaigiKeyboard;
\t\t\t};
\t\t};
\t\t8A03 /* Debug */ = {
\t\t\tbuildSettings = {
\t\t\t\tCURRENT_PROJECT_VERSION = 1;
\t\t\t\tMARKETING_VERSION = 3.6.6;
\t\t\t\tPRODUCT_BUNDLE_IDENTIFIER = com.siansiansu.TaigiKeyboard.TaigiKeyboardExtension;
\t\t\t};
\t\t};
\t\t8A04 /* Release */ = {
\t\t\tbuildSettings = {
\t\t\t\tCURRENT_PROJECT_VERSION = 1;
\t\t\t\tMARKETING_VERSION = 3.6.6;
\t\t\t\tPRODUCT_BUNDLE_IDENTIFIER = com.siansiansu.TaigiKeyboard.TaigiKeyboardExtension;
\t\t\t};
\t\t};
\t\t8A05 /* Debug */ = {
\t\t\tbuildSettings = {
\t\t\t\tCURRENT_PROJECT_VERSION = 1;
\t\t\t\tMARKETING_VERSION = 1.0;
\t\t\t\tPRODUCT_BUNDLE_IDENTIFIER = com.siansiansu.TaigiKeyboardTests;
\t\t\t};
\t\t};
"""

PLIST_FIXTURE = """<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
\t<key>CFBundleName</key>
\t<string>TaigiKeyboard</string>
\t<!-- The marketing version is kept in lockstep with iOS and Android; the build
\t     version derives as MAJOR*10000 + MINOR*100 + PATCH. -->
\t<key>CFBundleShortVersionString</key>
\t<string>3.6.6</string>
\t<key>CFBundleVersion</key>
\t<string>30606</string>
</dict>
</plist>
"""


CARGO_FIXTURE = """[workspace]
resolver = "2"
members = ["crates/taigi-windows-core"]

[workspace.package]
version = "3.6.6"
edition = "2021"

[workspace.dependencies]
# A dependency pinned to a version that must NOT move with the release.
windows = { version = "0.62" }
egui = "=0.31.1"
"""


PROJECT_FILES = (
    release_notes.ANDROID_GRADLE_FILE,
    release_notes.IOS_PROJECT_FILE,
    release_notes.MACOS_INFO_PLIST_FILE,
    release_notes.WINDOWS_CARGO_FILE,
)


class ProjectVersionWriterTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp_dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp_dir.cleanup)
        self.repo_root = Path(self.temp_dir.name)
        self.write_tree()

    def write_tree(
        self,
        gradle: str = GRADLE_FIXTURE,
        pbxproj: str = PBXPROJ_FIXTURE,
        plist: str = PLIST_FIXTURE,
        cargo: str = CARGO_FIXTURE,
    ) -> None:
        for relative_path, content in zip(PROJECT_FILES, (gradle, pbxproj, plist, cargo)):
            path = self.repo_root / relative_path
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(content, encoding="utf-8", newline="")

    def read(self, relative_path: str) -> str:
        return (self.repo_root / relative_path).read_text(encoding="utf-8")

    def snapshot(self) -> dict[str, bytes]:
        """Every project file's raw bytes — what a rejected run must not change."""
        return {
            relative_path: (self.repo_root / relative_path).read_bytes()
            for relative_path in PROJECT_FILES
            if (self.repo_root / relative_path).exists()
        }

    def assert_rejected(self, message: str, version: str, **kwargs: object) -> None:
        """The run raises and leaves all three files exactly as they were."""
        before = self.snapshot()

        with self.assertRaisesRegex(release_notes.ReleaseNotesError, message):
            release_notes.set_project_versions(self.repo_root, version, **kwargs)

        self.assertEqual(self.snapshot(), before)

    def patch_write_failing_on(self, failing_call: int) -> unittest.mock._patch:
        """Make the nth `_write_atomically` call raise; every other call behaves."""
        real_write = release_notes._write_atomically
        calls: list[Path] = []

        def failing_write(path: Path, content: str) -> None:
            calls.append(path)
            if len(calls) == failing_call:
                raise OSError("disk full")
            real_write(path, content)

        return unittest.mock.patch.object(
            release_notes, "_write_atomically", failing_write
        )

    def test_writes_one_version_across_all_three_platforms(self) -> None:
        changes = release_notes.set_project_versions(self.repo_root, "3.6.7")

        release_notes.check_project_versions(self.repo_root, "3.6.7")
        self.assertEqual(
            changes,
            (
                "Android: versionName 3.6.6 -> 3.6.7",
                "iOS: MARKETING_VERSION 3.6.6 -> 3.6.7, "
                "CURRENT_PROJECT_VERSION 1 -> 1",
                "macOS: CFBundleShortVersionString 3.6.6 -> 3.6.7, "
                "CFBundleVersion 30606 -> 30607",
                "Windows: workspace version 3.6.6 -> 3.6.7",
            ),
        )

    def test_refuses_a_cargo_manifest_without_a_workspace_version(self) -> None:
        self.write_tree(cargo=CARGO_FIXTURE.replace('version = "3.6.6"\n', ""))

        with self.assertRaisesRegex(release_notes.ReleaseNotesError, "found 0"):
            release_notes.set_project_versions(self.repo_root, "3.6.7")

    def test_refuses_a_cargo_manifest_with_two_workspace_package_tables(self) -> None:
        # A duplicate key inside one table is TOML-invalid (Cargo refuses it);
        # the duplicate that parses is a second `[workspace.package]` table.
        self.write_tree(
            cargo=CARGO_FIXTURE + '\n[workspace.package]\nversion = "3.6.6"\n'
        )

        with self.assertRaisesRegex(release_notes.ReleaseNotesError, "found 2"):
            release_notes.set_project_versions(self.repo_root, "3.6.7")

    def test_moves_only_the_windows_workspace_version(self) -> None:
        release_notes.set_project_versions(self.repo_root, "3.6.7")

        cargo_source = self.read(release_notes.WINDOWS_CARGO_FILE)
        self.assertEqual(
            cargo_source,
            CARGO_FIXTURE.replace('version = "3.6.6"', 'version = "3.6.7"'),
        )
        # The pinned dependency versions are not the release train's.
        self.assertIn('windows = { version = "0.62" }', cargo_source)
        self.assertIn('egui = "=0.31.1"', cargo_source)

    def test_leaves_the_test_target_and_every_other_byte_alone(self) -> None:
        release_notes.set_project_versions(self.repo_root, "3.6.7")

        # Only the four shipping blocks move, and their build number is pinned to
        # 1 — App Store Connect numbers a version's uploads itself. The test
        # target keeps its own MARKETING_VERSION = 1.0.
        self.assertEqual(
            self.read(release_notes.IOS_PROJECT_FILE),
            PBXPROJ_FIXTURE.replace(
                "MARKETING_VERSION = 3.6.6;", "MARKETING_VERSION = 3.6.7;"
            ),
        )
        self.assertEqual(
            self.read(release_notes.ANDROID_GRADLE_FILE),
            GRADLE_FIXTURE.replace("3.6.6", "3.6.7"),
        )

    def test_keeps_the_plists_hand_written_comments(self) -> None:
        release_notes.set_project_versions(self.repo_root, "3.6.7")

        plist_source = self.read(release_notes.MACOS_INFO_PLIST_FILE)
        self.assertIn("MAJOR*10000 + MINOR*100 + PATCH", plist_source)
        self.assertEqual(
            plist_source,
            PLIST_FIXTURE.replace("3.6.6", "3.6.7").replace("30606", "30607"),
        )

    def test_pins_the_ios_build_number_to_one(self) -> None:
        # Whatever the tree carried, every run writes 1: App Store Connect
        # numbers the uploads of a marketing version itself.
        self.write_tree(
            pbxproj=PBXPROJ_FIXTURE.replace(
                "CURRENT_PROJECT_VERSION = 1;\n\t\t\t\tMARKETING_VERSION = 3.6.6;",
                "CURRENT_PROJECT_VERSION = 9;\n\t\t\t\tMARKETING_VERSION = 3.6.6;",
            )
        )

        changes = release_notes.set_project_versions(self.repo_root, "3.6.7")

        self.assertEqual(
            self.read(release_notes.IOS_PROJECT_FILE).count(
                "CURRENT_PROJECT_VERSION = 1;"
            ),
            5,
            "four shipping blocks pinned to 1, plus the untouched test target",
        )
        self.assertIn("CURRENT_PROJECT_VERSION 9 -> 1", changes[1])

    def test_rerunning_the_current_version_changes_nothing(self) -> None:
        before = self.snapshot()

        release_notes.set_project_versions(self.repo_root, "3.6.6")

        self.assertEqual(self.snapshot(), before)

    def test_rejects_a_version_that_goes_backwards(self) -> None:
        self.assert_rejected("lower than", "3.6.5")

    def test_allows_a_downgrade_when_asked_for_one(self) -> None:
        release_notes.set_project_versions(
            self.repo_root, "3.6.5", allow_downgrade=True
        )

        release_notes.check_project_versions(self.repo_root, "3.6.5")

    def test_rejects_a_version_that_is_not_three_components(self) -> None:
        self.assert_rejected("MAJOR.MINOR.PATCH", "3.6")

    def test_rejects_shipping_targets_that_already_disagree(self) -> None:
        self.write_tree(
            pbxproj=PBXPROJ_FIXTURE.replace(
                "MARKETING_VERSION = 3.6.6;", "MARKETING_VERSION = 3.6.5;", 1
            )
        )

        self.assert_rejected("different MARKETING_VERSION", "3.6.7")

    def test_rejects_an_unexpected_number_of_shipping_blocks(self) -> None:
        self.write_tree(
            pbxproj=PBXPROJ_FIXTURE.replace(
                "PRODUCT_BUNDLE_IDENTIFIER = com.siansiansu.TaigiKeyboard;",
                "PRODUCT_BUNDLE_IDENTIFIER = com.siansiansu.TaigiKeyboardTests;",
                1,
            )
        )

        self.assert_rejected("expected 2 iOS build settings blocks", "3.6.7")

    def test_rejects_a_duplicate_android_version_name(self) -> None:
        self.write_tree(gradle=GRADLE_FIXTURE + '        versionName = "3.6.6"\n')

        self.assert_rejected("exactly one versionName", "3.6.7")

    def test_rejects_a_binary_plist_it_cannot_edit_as_text(self) -> None:
        (self.repo_root / release_notes.MACOS_INFO_PLIST_FILE).write_bytes(
            plistlib.dumps(
                {"CFBundleShortVersionString": "3.6.6", "CFBundleVersion": "30606"},
                fmt=plistlib.FMT_BINARY,
            )
        )

        self.assert_rejected("cannot read", "3.6.7")

    def test_rejects_an_xml_plist_that_is_not_a_plist(self) -> None:
        self.write_tree(plist="{ not a plist }\n")

        self.assert_rejected("is not XML text", "3.6.7")

    def test_reports_a_missing_project_file_without_touching_the_others(self) -> None:
        (self.repo_root / release_notes.MACOS_INFO_PLIST_FILE).unlink()

        self.assert_rejected("missing macos/", "3.6.7")

    def test_restores_the_tree_when_a_later_write_fails(self) -> None:
        before = self.snapshot()

        with self.patch_write_failing_on(2):
            with self.assertRaisesRegex(
                release_notes.ReleaseNotesError, "the tree was restored"
            ):
                release_notes.set_project_versions(self.repo_root, "3.6.7")

        self.assertEqual(self.snapshot(), before)

    def test_names_the_files_a_failed_rollback_left_behind(self) -> None:
        real_write = release_notes._write_atomically
        calls: list[Path] = []

        def write_once_then_fail(path: Path, content: str) -> None:
            calls.append(path)
            if len(calls) == 1:
                real_write(path, content)
                return
            raise OSError("disk full")

        with unittest.mock.patch.object(
            release_notes, "_write_atomically", write_once_then_fail
        ):
            with self.assertRaisesRegex(
                release_notes.ReleaseNotesError,
                "restoring them failed too — android/app/build.gradle.kts",
            ):
                release_notes.set_project_versions(self.repo_root, "3.6.7")

        self.assertIn(
            'versionName = "3.6.7"', self.read(release_notes.ANDROID_GRADLE_FILE)
        )

    def test_keeps_each_files_permission_bits(self) -> None:
        modes = {}
        for relative_path in PROJECT_FILES:
            path = self.repo_root / relative_path
            path.chmod(0o644)
            modes[relative_path] = stat.S_IMODE(path.stat().st_mode)

        release_notes.set_project_versions(self.repo_root, "3.6.7")

        for relative_path, mode in modes.items():
            self.assertEqual(
                stat.S_IMODE((self.repo_root / relative_path).stat().st_mode), mode
            )


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
