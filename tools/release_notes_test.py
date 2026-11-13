"""Unit tests for release-note validation and release-train version handling."""

from __future__ import annotations

import plistlib
import stat
import subprocess
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

    def test_admits_eight_entries_and_rejects_a_ninth(self) -> None:
        entries = tuple(f"Fixed: bug {index}." for index in range(1, 9))

        release_notes.validate_notes(
            release_notes.PlatformNotes(platform="ios", entries=entries)
        )

        with self.assertRaisesRegex(release_notes.ReleaseNotesError, "expected 1-8"):
            release_notes.validate_notes(
                release_notes.PlatformNotes(
                    platform="ios", entries=entries + ("Fixed: bug 9.",)
                )
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


class CanonicalNotesCheckTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp_dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp_dir.cleanup)
        self.repo_root = Path(self.temp_dir.name)
        self.notes_dir = self.repo_root / "changelog/store/v3.7.0"
        self.notes_dir.mkdir(parents=True)
        (self.notes_dir / "ios.txt").write_text(
            "New: candidate navigation is easier to use.\n", encoding="utf-8"
        )
        (self.notes_dir / "android.txt").write_text(
            "Fixed: candidate navigation responds consistently.\n", encoding="utf-8"
        )

    def test_passes_on_both_canonical_files_alone(self) -> None:
        release_notes.check_notes(self.repo_root, "3.7.0")

    def test_fails_when_one_platforms_file_is_missing(self) -> None:
        (self.notes_dir / "android.txt").unlink()
        with self.assertRaisesRegex(
            release_notes.ReleaseNotesError, "missing canonical"
        ):
            release_notes.check_notes(self.repo_root, "3.7.0")

    def test_fails_when_one_platforms_file_is_invalid(self) -> None:
        (self.notes_dir / "ios.txt").write_text(
            "candidate navigation\n", encoding="utf-8"
        )
        with self.assertRaisesRegex(release_notes.ReleaseNotesError, "must start with"):
            release_notes.check_notes(self.repo_root, "3.7.0")


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

    def test_derives_a_calver_build_version_above_every_semver_one(self) -> None:
        # trace: 2026*10_000 + 12*100 + 0 = 20_261_200 > 30_900 (3.9.0), so
        # Installer upgrades 3.9.0 to the first CalVer release.
        self.assertEqual(release_notes.macos_build_version("2026.12.0"), "20261200")
        self.assertGreater(
            int(release_notes.macos_build_version("2026.1.0")),
            int(release_notes.macos_build_version("3.9.0")),
        )

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
members = ["crates/taigi-desktop-core"]

[workspace.package]
version = "3.6.6"
edition = "2021"

[workspace.dependencies]
# A dependency pinned to a version that must NOT move with the release.
windows = { version = "0.62" }
egui = "=0.31.1"
"""


DESKTOP_SHARED_CARGO_FIXTURE = """[workspace]
resolver = "2"
members = ["crates/taigi-desktop-core", "crates/taigi-desktop-storage"]

[workspace.package]
version = "3.6.6"
edition = "2021"

[workspace.dependencies]
rusqlite = { version = "0.40", features = ["bundled"] }
"""


LINUX_CARGO_FIXTURE = """[workspace]
resolver = "2"
members = ["crates/taigi-linux-core"]

[workspace.package]
version = "3.6.6"
edition = "2024"

[workspace.dependencies]
zbus = { version = "5", default-features = false }
"""


MACOS_CARGO_FIXTURE = """[workspace]
resolver = "2"
members = ["crates/taigi-macos-ffi"]

[workspace.package]
version = "3.6.6"
edition = "2021"

[workspace.dependencies]
prost = "0.14"
"""


CHROMEOS_MANIFEST_FIXTURE = """{
  "manifest_version": 3,
  "name": "Taigi",
  "version": "3.6.6",
  "minimum_chrome_version": "116"
}
"""


# The version each train's writer tests move to: the desktop train is CalVer.
NEXT_VERSION = {"mobile": "3.6.7", "desktop": "2026.12.0"}
# A version below NEXT_VERSION on the same train.
LOWER_VERSION = {"mobile": "3.6.5", "desktop": "2026.11.3"}


PROJECT_FILES = (
    release_notes.ANDROID_GRADLE_FILE,
    release_notes.IOS_PROJECT_FILE,
    release_notes.MACOS_INFO_PLIST_FILE,
    release_notes.WINDOWS_CARGO_FILE,
    release_notes.DESKTOP_SHARED_CARGO_FILE,
    release_notes.LINUX_CARGO_FILE,
    release_notes.MACOS_CARGO_FILE,
    release_notes.CHROMEOS_MANIFEST_FILE,
)


class DesktopLockfileRefreshTests(unittest.TestCase):
    def test_refreshes_every_lockfile_that_pins_the_desktop_crates(self) -> None:
        repo_root = Path("/repo")
        with unittest.mock.patch.object(release_notes.subprocess, "run") as run:
            run.return_value.returncode = 0
            release_notes.refresh_desktop_lockfiles(repo_root)

        refreshed = [call.args[0][-1] for call in run.call_args_list]
        self.assertEqual(
            refreshed,
            [
                f"/repo/{cargo_file}"
                for cargo_file in (
                    "windows/Cargo.toml",
                    "desktop/Cargo.toml",
                    "linux/Cargo.toml",
                    "macos/Cargo.toml",
                    "web/Cargo.toml",
                )
            ],
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
        desktop_cargo: str = DESKTOP_SHARED_CARGO_FIXTURE,
        linux_cargo: str = LINUX_CARGO_FIXTURE,
        macos_cargo: str = MACOS_CARGO_FIXTURE,
        chromeos_manifest: str = CHROMEOS_MANIFEST_FIXTURE,
    ) -> None:
        for relative_path, content in zip(
            PROJECT_FILES,
            (gradle, pbxproj, plist, cargo, desktop_cargo, linux_cargo, macos_cargo, chromeos_manifest),
            strict=True,
        ):
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

    def assert_rejected(
        self, message: str, version: str, train: str, **kwargs: object
    ) -> None:
        """The run raises and leaves all four files exactly as they were."""
        before = self.snapshot()

        with self.assertRaisesRegex(release_notes.ReleaseNotesError, message):
            release_notes.set_project_versions(
                self.repo_root, version, train, **kwargs
            )

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

    def test_writes_the_mobile_trains_version_to_both_mobile_platforms(self) -> None:
        changes = release_notes.set_project_versions(self.repo_root, "3.6.7", "mobile")

        release_notes.check_project_versions(self.repo_root, "3.6.7", "mobile")
        self.assertEqual(
            changes,
            (
                "Android: versionName 3.6.6 -> 3.6.7",
                "iOS: MARKETING_VERSION 3.6.6 -> 3.6.7, "
                "CURRENT_PROJECT_VERSION 1 -> 1",
            ),
        )

    def test_writes_the_desktop_trains_version_to_every_desktop_platform(self) -> None:
        # The fixture tree carries a SemVer desktop version, as the tree did
        # before the first CalVer train: moving to CalVer is an upgrade.
        changes = release_notes.set_project_versions(self.repo_root, "2026.12.0", "desktop")

        release_notes.check_project_versions(self.repo_root, "2026.12.0", "desktop")
        self.assertEqual(
            changes,
            (
                "macOS: CFBundleShortVersionString 3.6.6 -> 2026.12.0, "
                "CFBundleVersion 30606 -> 20261200",
                "windows/Cargo.toml: workspace version 3.6.6 -> 2026.12.0",
                "desktop/Cargo.toml: workspace version 3.6.6 -> 2026.12.0",
                "linux/Cargo.toml: workspace version 3.6.6 -> 2026.12.0",
                "macos/Cargo.toml: workspace version 3.6.6 -> 2026.12.0",
                "web/chromeos/manifest.json: version 3.6.6 -> 2026.12.0",
            ),
        )
        # Only the version moved: the other keys, including the two that end
        # in "version", keep their values and the file keeps its layout.
        self.assertEqual(
            self.read(release_notes.CHROMEOS_MANIFEST_FILE),
            CHROMEOS_MANIFEST_FIXTURE.replace('"version": "3.6.6"', '"version": "2026.12.0"'),
        )

    def test_check_refuses_a_chromeos_manifest_on_another_version(self) -> None:
        # chromeos-build.yml names the draft from the manifest: one left behind
        # would attach to last release's draft, or to none.
        self.write_tree(
            chromeos_manifest=CHROMEOS_MANIFEST_FIXTURE.replace('"3.6.6"', '"3.6.5"')
        )

        with self.assertRaisesRegex(
            release_notes.ReleaseNotesError, "web/chromeos/manifest.json version is 3.6.5"
        ):
            release_notes.check_project_versions(self.repo_root, "3.6.6", "desktop")

    def test_refuses_to_move_a_chromeos_manifest_backwards(self) -> None:
        # The store refuses a version that is not higher than the last it took.
        self.write_tree(
            chromeos_manifest=CHROMEOS_MANIFEST_FIXTURE.replace('"3.6.6"', '"2026.12.0"')
        )

        self.assert_rejected("2026.12.0", "2026.11.0", "desktop")

    def test_check_refuses_a_chromeos_manifest_that_is_not_json(self) -> None:
        self.write_tree(chromeos_manifest=CHROMEOS_MANIFEST_FIXTURE + "}")

        with self.assertRaisesRegex(release_notes.ReleaseNotesError, "not valid JSON"):
            release_notes.check_project_versions(self.repo_root, "3.6.6", "desktop")

    def test_check_refuses_a_linux_workspace_on_another_version(self) -> None:
        # The `.deb` is named from linux/Cargo.toml: a Linux workspace left
        # behind would stage last release's package name under this tag.
        self.write_tree(
            linux_cargo=LINUX_CARGO_FIXTURE.replace('version = "3.6.6"', 'version = "3.6.5"')
        )

        with self.assertRaisesRegex(
            release_notes.ReleaseNotesError, "linux/Cargo.toml workspace version is 3.6.5"
        ):
            release_notes.check_project_versions(self.repo_root, "3.6.6", "desktop")

    def test_check_refuses_a_macos_workspace_on_another_version(self) -> None:
        # The archive the macOS app links is a desktop crate like any other: a
        # bump that skipped it would ship a crate version its installer is not.
        self.write_tree(
            macos_cargo=MACOS_CARGO_FIXTURE.replace('version = "3.6.6"', 'version = "3.6.5"')
        )

        with self.assertRaisesRegex(
            release_notes.ReleaseNotesError, "macos/Cargo.toml workspace version is 3.6.5"
        ):
            release_notes.check_project_versions(self.repo_root, "3.6.6", "desktop")

    def test_a_train_bump_leaves_the_other_trains_files_alone(self) -> None:
        # The two trains are numbered independently: a mobile release must not
        # move macOS or Windows, and a desktop release must not move iOS or
        # Android — that is the whole point of having two.
        desktop_before = {
            path: self.read(path)
            for path in release_notes.TRAIN_FILES["desktop"]
        }
        release_notes.set_project_versions(self.repo_root, "3.6.7", "mobile")
        self.assertEqual(
            {path: self.read(path) for path in release_notes.TRAIN_FILES["desktop"]},
            desktop_before,
        )
        release_notes.check_project_versions(self.repo_root, "3.6.6", "desktop")

        mobile_before = {
            path: self.read(path) for path in release_notes.TRAIN_FILES["mobile"]
        }
        release_notes.set_project_versions(self.repo_root, "2026.12.0", "desktop")
        self.assertEqual(
            {path: self.read(path) for path in release_notes.TRAIN_FILES["mobile"]},
            mobile_before,
        )
        release_notes.check_project_versions(self.repo_root, "3.6.7", "mobile")

    def test_a_train_never_reads_the_other_trains_files(self) -> None:
        # A missing or broken file on the other train is not this train's
        # problem: a mobile release must not be blocked by a macOS plist, nor a
        # desktop release by the user-owned pbxproj.
        for train, other in (("mobile", "desktop"), ("desktop", "mobile")):
            with self.subTest(train=train):
                self.write_tree()
                for relative_path in release_notes.TRAIN_FILES[other]:
                    (self.repo_root / relative_path).unlink()

                release_notes.set_project_versions(self.repo_root, NEXT_VERSION[train], train)

                release_notes.check_project_versions(self.repo_root, NEXT_VERSION[train], train)

    def test_rejects_an_unknown_train(self) -> None:
        self.assert_rejected("unknown release train", "3.6.7", "tablet")
        with self.assertRaisesRegex(release_notes.ReleaseNotesError, "unknown release train"):
            release_notes.check_project_versions(self.repo_root, "3.6.6", "tablet")

    def test_refuses_a_cargo_manifest_without_a_workspace_version(self) -> None:
        self.write_tree(cargo=CARGO_FIXTURE.replace('version = "3.6.6"\n', ""))

        with self.assertRaisesRegex(release_notes.ReleaseNotesError, "found 0"):
            release_notes.set_project_versions(self.repo_root, "2026.12.0", "desktop")

    def test_refuses_a_cargo_manifest_with_two_workspace_package_tables(self) -> None:
        # A duplicate key inside one table is TOML-invalid (Cargo refuses it);
        # the duplicate that parses is a second `[workspace.package]` table.
        self.write_tree(
            cargo=CARGO_FIXTURE + '\n[workspace.package]\nversion = "3.6.6"\n'
        )

        with self.assertRaisesRegex(release_notes.ReleaseNotesError, "found 2"):
            release_notes.set_project_versions(self.repo_root, "2026.12.0", "desktop")

    def test_moves_only_the_windows_workspace_version(self) -> None:
        release_notes.set_project_versions(self.repo_root, "2026.12.0", "desktop")

        cargo_source = self.read(release_notes.WINDOWS_CARGO_FILE)
        self.assertEqual(
            cargo_source,
            CARGO_FIXTURE.replace('version = "3.6.6"', 'version = "2026.12.0"'),
        )
        # The pinned dependency versions are not the release train's.
        self.assertIn('windows = { version = "0.62" }', cargo_source)
        self.assertIn('egui = "=0.31.1"', cargo_source)

    def test_leaves_the_test_target_and_every_other_byte_alone(self) -> None:
        release_notes.set_project_versions(self.repo_root, "3.6.7", "mobile")

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
        release_notes.set_project_versions(self.repo_root, "2026.12.0", "desktop")

        plist_source = self.read(release_notes.MACOS_INFO_PLIST_FILE)
        self.assertIn("MAJOR*10000 + MINOR*100 + PATCH", plist_source)
        self.assertEqual(
            plist_source,
            PLIST_FIXTURE.replace("3.6.6", "2026.12.0").replace("30606", "20261200"),
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

        changes = release_notes.set_project_versions(self.repo_root, "3.6.7", "mobile")

        self.assertEqual(
            self.read(release_notes.IOS_PROJECT_FILE).count(
                "CURRENT_PROJECT_VERSION = 1;"
            ),
            5,
            "four shipping blocks pinned to 1, plus the untouched test target",
        )
        self.assertIn("CURRENT_PROJECT_VERSION 9 -> 1", changes[1])

    def test_rerunning_the_current_version_changes_nothing(self) -> None:
        for train in release_notes.TRAIN_CHOICES:
            release_notes.set_project_versions(self.repo_root, NEXT_VERSION[train], train)
        before = self.snapshot()

        for train in release_notes.TRAIN_CHOICES:
            release_notes.set_project_versions(self.repo_root, NEXT_VERSION[train], train)

        self.assertEqual(self.snapshot(), before)

    def test_rejects_a_version_that_goes_backwards(self) -> None:
        for train in release_notes.TRAIN_CHOICES:
            with self.subTest(train=train):
                release_notes.set_project_versions(self.repo_root, NEXT_VERSION[train], train)
                self.assert_rejected("lower than", LOWER_VERSION[train], train)

    def test_the_desktop_train_takes_only_calver(self) -> None:
        for version in (
            "3.9.1",  # SemVer: the desktop train left it after 3.9.0
            "2025.12.0",  # before the first CalVer year
            "2026.13.0",
            "2026.0.0",
            "2026.01.0",  # leading zero: Cargo's SemVer parser refuses it
            "2026.12.01",
            "02026.12.0",
        ):
            with self.subTest(version=version):
                self.assert_rejected("YYYY.M.P", version, "desktop")
        # Allowing a downgrade relaxes the order, never the format.
        self.assert_rejected("YYYY.M.P", "3.9.1", "desktop", allow_downgrade=True)

    def test_the_desktop_train_takes_a_v_prefix_like_the_mobile_train(self) -> None:
        release_notes.set_project_versions(self.repo_root, "v2026.12.0", "desktop")

        release_notes.check_project_versions(self.repo_root, "2026.12.0", "desktop")

    def test_the_desktop_train_takes_a_monthly_patch_and_the_next_months_cut(self) -> None:
        for version in ("2026.11.1", "2026.12.0", "2026.12.1", "2027.1.0"):
            with self.subTest(version=version):
                release_notes.set_project_versions(self.repo_root, version, "desktop")
                release_notes.check_project_versions(self.repo_root, version, "desktop")

    def test_a_patch_counter_past_99_is_refused(self) -> None:
        # trace: 2026.11.100 -> 2026*10_000 + 11*100 + 100 = 20_261_200, the
        # build version of 2026.12.0 — Installer would refuse that upgrade.
        release_notes.set_project_versions(self.repo_root, "2026.11.0", "desktop")
        self.assert_rejected("would collide", "2026.11.100", "desktop")

    def test_the_mobile_train_keeps_semver(self) -> None:
        release_notes.set_project_versions(self.repo_root, "3.10.0", "mobile")

        release_notes.check_project_versions(self.repo_root, "3.10.0", "mobile")

    def test_judges_a_downgrade_against_its_own_train_only(self) -> None:
        # Desktop moved ahead; mobile going to a number below desktop's is
        # not a downgrade of anything mobile has shipped.
        release_notes.set_project_versions(self.repo_root, "2026.12.0", "desktop")

        release_notes.set_project_versions(self.repo_root, "3.6.7", "mobile")

        release_notes.check_project_versions(self.repo_root, "3.6.7", "mobile")
        release_notes.check_project_versions(self.repo_root, "2026.12.0", "desktop")

    def test_allows_a_downgrade_when_asked_for_one(self) -> None:
        release_notes.set_project_versions(
            self.repo_root, "3.6.5", "mobile", allow_downgrade=True
        )

        release_notes.check_project_versions(self.repo_root, "3.6.5", "mobile")

    def test_rejects_a_version_that_is_not_three_components(self) -> None:
        self.assert_rejected("MAJOR.MINOR.PATCH", "3.6", "mobile")

    def test_rejects_shipping_targets_that_already_disagree(self) -> None:
        self.write_tree(
            pbxproj=PBXPROJ_FIXTURE.replace(
                "MARKETING_VERSION = 3.6.6;", "MARKETING_VERSION = 3.6.5;", 1
            )
        )

        self.assert_rejected("different MARKETING_VERSION", "3.6.7", "mobile")

    def test_rejects_an_unexpected_number_of_shipping_blocks(self) -> None:
        self.write_tree(
            pbxproj=PBXPROJ_FIXTURE.replace(
                "PRODUCT_BUNDLE_IDENTIFIER = com.siansiansu.TaigiKeyboard;",
                "PRODUCT_BUNDLE_IDENTIFIER = com.siansiansu.TaigiKeyboardTests;",
                1,
            )
        )

        self.assert_rejected("expected 2 iOS build settings blocks", "3.6.7", "mobile")

    def test_rejects_a_duplicate_android_version_name(self) -> None:
        self.write_tree(gradle=GRADLE_FIXTURE + '        versionName = "3.6.6"\n')

        self.assert_rejected("exactly one versionName", "3.6.7", "mobile")

    def test_rejects_a_binary_plist_it_cannot_edit_as_text(self) -> None:
        (self.repo_root / release_notes.MACOS_INFO_PLIST_FILE).write_bytes(
            plistlib.dumps(
                {"CFBundleShortVersionString": "3.6.6", "CFBundleVersion": "30606"},
                fmt=plistlib.FMT_BINARY,
            )
        )

        self.assert_rejected("cannot read", "2026.12.0", "desktop")

    def test_rejects_an_xml_plist_that_is_not_a_plist(self) -> None:
        self.write_tree(plist="{ not a plist }\n")

        self.assert_rejected("is not XML text", "2026.12.0", "desktop")

    def test_reports_a_missing_project_file_without_touching_the_others(self) -> None:
        (self.repo_root / release_notes.MACOS_INFO_PLIST_FILE).unlink()

        self.assert_rejected("missing macos/", "2026.12.0", "desktop")

    def test_restores_the_tree_when_a_later_write_fails(self) -> None:
        before = self.snapshot()

        with self.patch_write_failing_on(2):
            with self.assertRaisesRegex(
                release_notes.ReleaseNotesError, "the tree was restored"
            ):
                release_notes.set_project_versions(self.repo_root, "3.6.7", "mobile")

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
                release_notes.set_project_versions(self.repo_root, "3.6.7", "mobile")

        self.assertIn(
            'versionName = "3.6.7"', self.read(release_notes.ANDROID_GRADLE_FILE)
        )

    def test_keeps_each_files_permission_bits(self) -> None:
        modes = {}
        for relative_path in PROJECT_FILES:
            path = self.repo_root / relative_path
            path.chmod(0o644)
            modes[relative_path] = stat.S_IMODE(path.stat().st_mode)

        for train in release_notes.TRAIN_CHOICES:
            release_notes.set_project_versions(self.repo_root, NEXT_VERSION[train], train)

        for relative_path, mode in modes.items():
            self.assertEqual(
                stat.S_IMODE((self.repo_root / relative_path).stat().st_mode), mode
            )


def fragment_text(platforms: str, section: str, bullet: str) -> str:
    return f"---\nplatforms: {platforms}\nsection: {section}\n---\n{bullet}\n"


class FragmentParserTests(unittest.TestCase):
    def test_reads_platforms_in_release_page_order(self) -> None:
        fragment = release_notes.parse_fragment(
            "x.md",
            fragment_text("[linux, macos]", "Input", "- **Claim.** Detail\n  wrapped."),
        )

        self.assertEqual(fragment.platforms, ("macos", "linux"))
        self.assertEqual(fragment.section, "Input")
        self.assertEqual(fragment.bullet, "- **Claim.** Detail\n  wrapped.")

    def test_rejects_malformed_fragments(self) -> None:
        cases = {
            "no front matter": "- **Claim.**\n",
            "unclosed": "---\nplatforms: [macos]\nsection: Input\n- x\n",
            "unknown platform": fragment_text("[ios]", "Input", "- x"),
            "bare platform": fragment_text("macos", "Input", "- x"),
            "empty list": fragment_text("[]", "Input", "- x"),
            "duplicate platform": fragment_text("[macos, macos]", "Input", "- x"),
            "unknown section": fragment_text("[macos]", "Fixes", "- x"),
            "extra key": "---\nplatforms: [macos]\nsection: Input\npr: 1\n---\n- x\n",
            "missing section": "---\nplatforms: [macos]\n---\n- x\n",
            "no bullet": fragment_text("[macos]", "Input", "Plain text."),
            "empty bullet": fragment_text("[macos]", "Input", "- "),
            "two bullets": fragment_text("[macos]", "Input", "- one\n- two"),
            "blank inside": fragment_text("[macos]", "Input", "- one\n\n  two"),
            "unindented": fragment_text("[macos]", "Input", "- one\ntwo"),
        }
        for label, text in cases.items():
            with self.subTest(label), self.assertRaises(release_notes.ReleaseNotesError):
                release_notes.parse_fragment("x.md", text)

    def test_classifies_desktop_shipping_paths(self) -> None:
        shipping = (
            "macos/Sources/App.swift",
            "engine/composing/src/lib.rs",
            "dictionary/supplementary/dev/data/dev.csv",
            "web/chromeos/manifest.json",
            "web/Cargo.lock",
        )
        not_shipping = (
            "engine/swift-ffi/src/lib.rs",
            "engine/android-jni/src/lib.rs",
            "engine/composing/tests/walker.rs",
            "engine/test-support/src/lib.rs",
            "macos/Tests/CandidateMetricsTests.swift",
            "linux/README.md",
            "ios/Sources/Keyboard.swift",
            "tools/release_notes.py",
            "changelog/unreleased/x.md",
        )
        for path in shipping:
            with self.subTest(path):
                self.assertTrue(release_notes.is_desktop_shipping_path(path))
        for path in not_shipping:
            with self.subTest(path):
                self.assertFalse(release_notes.is_desktop_shipping_path(path))


CHANGELOG_INDEX = """# Changelog

## Desktop — macOS + Windows + Linux + ChromeOS

- [v3.9.0](changelog/desktop-v3.9.0.md)

## Mobile — iOS + Android

- [v3.9.0](changelog/mobile-v3.9.0.md)
"""


class GitRepositoryTestCase(unittest.TestCase):
    """A throwaway repository whose main line merges pull requests like GitHub does."""

    def setUp(self) -> None:
        self.temp_dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp_dir.cleanup)
        self.repo_root = Path(self.temp_dir.name)
        self.git("init", "--quiet", "--initial-branch=main")
        self.git("config", "user.name", "Test")
        self.git("config", "user.email", "test@example.invalid")
        (self.repo_root / "CHANGELOG.md").write_text(CHANGELOG_INDEX, encoding="utf-8")
        (self.repo_root / "changelog" / "unreleased").mkdir(parents=True)
        (self.repo_root / "changelog" / "unreleased" / "README.md").write_text("format\n")
        self.git("add", "-A")
        self.git("commit", "--quiet", "-m", "initial")

    def git(self, *arguments: str) -> str:
        return subprocess.run(
            ["git", "-C", str(self.repo_root), *arguments],
            check=True,
            capture_output=True,
            text=True,
        ).stdout

    def write(self, relative_path: str, text: str) -> None:
        path = self.repo_root / relative_path
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")

    def head(self) -> str:
        return self.git("rev-parse", "HEAD").strip()

    def commit(self, files: dict[str, str], message: str) -> None:
        for relative_path, text in files.items():
            self.write(relative_path, text)
        self.git("add", "-A")
        self.git("commit", "--quiet", "-m", message)

    def merge_pull_request(self, number: int, files: dict[str, str]) -> None:
        """Commit `files` on a branch and merge it with GitHub's merge-commit subject."""
        self.git("switch", "--quiet", "-c", f"pr-{number}")
        self.commit(files, f"change {number}")
        self.git("switch", "--quiet", "main")
        self.git(
            "merge", "--quiet", "--no-ff", f"pr-{number}",
            "-m", f"Merge pull request #{number} from someone/pr-{number}",
        )


def fragment_path(slug: str) -> str:
    return f"changelog/unreleased/{slug}.md"


class FragmentRequirementTests(GitRepositoryTestCase):
    def test_desktop_change_without_fragment_fails(self) -> None:
        base = self.head()
        self.commit({"engine/composing/src/lib.rs": "fn main() {}\n"}, "engine change")

        with self.assertRaisesRegex(release_notes.ReleaseNotesError, "no-changelog"):
            release_notes.check_fragments(self.repo_root, base)

    def test_desktop_change_with_fragment_passes(self) -> None:
        base = self.head()
        self.commit(
            {
                "engine/composing/src/lib.rs": "fn main() {}\n",
                fragment_path("x"): fragment_text("[macos]", "Input", "- **Claim.**"),
            },
            "engine change",
        )

        release_notes.check_fragments(self.repo_root, base)

    def test_readme_edit_is_not_a_fragment(self) -> None:
        base = self.head()
        self.commit(
            {"macos/Sources/App.swift": "//\n", "changelog/unreleased/README.md": "format, edited\n"},
            "macos change",
        )

        with self.assertRaises(release_notes.ReleaseNotesError):
            release_notes.check_fragments(self.repo_root, base)

    def test_non_desktop_change_needs_none(self) -> None:
        base = self.head()
        self.commit(
            {"ios/Sources/Keyboard.swift": "//\n", "engine/composing/tests/walker.rs": "//\n"},
            "mobile change",
        )

        release_notes.check_fragments(self.repo_root, base)

    def test_invalid_fragment_fails_even_without_base(self) -> None:
        self.write(fragment_path("bad"), fragment_text("[ios]", "Input", "- x"))

        with self.assertRaisesRegex(release_notes.ReleaseNotesError, "bad.md"):
            release_notes.check_fragments(self.repo_root)


class AssembleDesktopChangelogTests(GitRepositoryTestCase):
    def test_full_release_orders_platforms_sections_and_pull_requests(self) -> None:
        self.merge_pull_request(
            20,
            {fragment_path("late"): fragment_text("[linux, macos]", "Input", "- **Late.**")},
        )
        self.merge_pull_request(
            10,
            {
                fragment_path("early"): fragment_text("[macos]", "Input", "- **Early.**\n  More."),
                fragment_path("window"): fragment_text("[macos]", "Install", "- **Window.**"),
            },
        )
        self.merge_pull_request(
            30,
            {fragment_path("explicit"): fragment_text("[macos]", "Candidates", "- **Two.** (#5, #6)")},
        )

        changes = release_notes.assemble_desktop_changelog(self.repo_root, "2026.12.0")

        # trace: Input sorts by PR (10 before 20); Candidates keeps its own (#5, #6);
        # Install comes last in DESKTOP_SECTIONS; Windows / ChromeOS have no fragment.
        expected = """# desktop v2026.12.0

Changes for macOS and Linux.

### macOS

#### Input

- **Early.**
  More. (#10)
- **Late.** (#20)

#### Candidates

- **Two.** (#5, #6)

#### Install

- **Window.** (#10)

### Windows

No changes.

### Linux

#### Input

- **Late.** (#20)

### ChromeOS

No changes.
"""
        self.assertEqual(
            (self.repo_root / "changelog" / "desktop-v2026.12.0.md").read_text(encoding="utf-8"),
            expected,
        )
        self.assertIn(
            "## Desktop — macOS + Windows + Linux + ChromeOS\n\n"
            "- [v2026.12.0](changelog/desktop-v2026.12.0.md)\n"
            "- [v3.9.0](changelog/desktop-v3.9.0.md)\n",
            (self.repo_root / "CHANGELOG.md").read_text(encoding="utf-8"),
        )
        self.assertEqual(
            sorted(path.name for path in (self.repo_root / "changelog" / "unreleased").iterdir()),
            ["README.md"],
        )
        self.assertEqual(changes[:2], ("changelog/desktop-v2026.12.0.md", "CHANGELOG.md"))

    def test_squash_subject_and_direct_commit(self) -> None:
        self.commit(
            {fragment_path("squashed"): fragment_text("[macos]", "Input", "- **Squashed.**")},
            "fix(macos): something (#42)",
        )
        self.commit(
            {fragment_path("direct"): fragment_text("[macos]", "Input", "- **Direct.**")},
            "docs: no pull request",
        )

        release_notes.assemble_desktop_changelog(self.repo_root, "2026.12.0")

        changelog = (self.repo_root / "changelog" / "desktop-v2026.12.0.md").read_text()
        # trace: numbered entries first, the unnumbered one last (sys.maxsize).
        self.assertIn("- **Squashed.** (#42)\n- **Direct.**\n", changelog)

    def test_patch_takes_one_platform_and_keeps_the_rest(self) -> None:
        self.merge_pull_request(
            7,
            {
                fragment_path("shared"): fragment_text("[macos, linux]", "Input", "- **Shared.**"),
                fragment_path("linux-only"): fragment_text("[linux]", "Settings", "- **Linux.**"),
                fragment_path("mac-only"): fragment_text("[macos]", "Settings", "- **Mac.**"),
            },
        )

        release_notes.assemble_desktop_changelog(self.repo_root, "2026.11.1", "linux")

        self.assertEqual(
            (self.repo_root / "changelog" / "desktop-v2026.11.1.md").read_text(),
            "# desktop v2026.11.1\n\nA Linux-only patch. The other platforms are unchanged.\n\n"
            "### Linux\n\n#### Input\n\n- **Shared.** (#7)\n\n"
            "#### Settings\n\n- **Linux.** (#7)\n",
        )
        unreleased = self.repo_root / "changelog" / "unreleased"
        self.assertFalse((unreleased / "linux-only.md").exists())
        self.assertEqual(
            (unreleased / "shared.md").read_text(),
            fragment_text("[macos]", "Input", "- **Shared.**"),
        )
        self.assertTrue((unreleased / "mac-only.md").exists())
        self.assertIn(
            "- [v2026.11.1](changelog/desktop-v2026.11.1.md) — Linux-only patch\n",
            (self.repo_root / "CHANGELOG.md").read_text(),
        )

    def test_refuses_without_fragments_or_over_an_existing_file(self) -> None:
        with self.assertRaisesRegex(release_notes.ReleaseNotesError, "nothing to release"):
            release_notes.assemble_desktop_changelog(self.repo_root, "2026.12.0")

        self.merge_pull_request(1, {fragment_path("x"): fragment_text("[macos]", "Input", "- x")})
        self.write("changelog/desktop-v2026.12.0.md", "existing\n")
        with self.assertRaisesRegex(release_notes.ReleaseNotesError, "already exists"):
            release_notes.assemble_desktop_changelog(self.repo_root, "2026.12.0")
        self.assertTrue((self.repo_root / fragment_path("x")).exists())

    def test_platform_argument_must_match_the_version_kind(self) -> None:
        self.merge_pull_request(1, {fragment_path("x"): fragment_text("[macos]", "Input", "- x")})

        with self.assertRaisesRegex(release_notes.ReleaseNotesError, "patch"):
            release_notes.assemble_desktop_changelog(self.repo_root, "2026.12.1")
        with self.assertRaisesRegex(release_notes.ReleaseNotesError, "full release"):
            release_notes.assemble_desktop_changelog(self.repo_root, "2026.12.0", "macos")
        with self.assertRaisesRegex(release_notes.ReleaseNotesError, "YYYY.M.P"):
            release_notes.assemble_desktop_changelog(self.repo_root, "3.10.0")


if __name__ == "__main__":
    unittest.main()
