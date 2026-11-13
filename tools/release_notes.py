"""Validate store release notes, check or set a release train's version
number, and assemble the desktop changelog from per-PR fragments.

Two trains, two numbers: `mobile` (iOS + Android share one) and `desktop`
(macOS + Windows + Linux + ChromeOS share one). They move independently; within a train the
platforms cannot drift apart."""

from __future__ import annotations

import argparse
import dataclasses
import json
import os
import plistlib
import re
import stat
import subprocess
import sys
import tempfile
from collections.abc import Iterable
from dataclasses import dataclass
from pathlib import Path

MAX_STORE_CHARACTERS = 500
# The store text is what a user reads in a list; past that it stops being read.
# The real ceiling is MAX_STORE_CHARACTERS, which Google Play enforces — this
# one is a shape check, loose enough to let a release that genuinely did eight
# visible things say so.
MAX_ENTRIES = 8
ALLOWED_PREFIXES = ("New:", "Fixed:", "Improved:", "Changed:", "Updated:")
FORBIDDEN_MARKETING_PHRASES = (
    "#1",
    "best app",
    "best keyboard",
    "coming soon",
    "free for a limited time",
)
# The desktop train ships its own release notes: `macos/scripts/publish-release.sh`
# and `windows/scripts/publish-release.sh` hand their platform's section of
# `changelog/desktop-v<version>.md` to `gh release create`. The store notes are
# the mobile surface only, so desktop-only work belongs in that desktop file and
# never in either What's New.
MACOS_TERMS = ("macOS", "Mac", "Macs", "MacBook")
# "Mac" opens ordinary English words — machine, macro, macron — so it is the one
# term matched as a whole word. Every other term takes an open tail, which keeps
# plurals and compounds ("Androids", "macOSX", "MacBooks") caught; the spellings
# that tail cannot reach from "Mac" are listed above instead.
WHOLE_WORD_ONLY_TERMS = frozenset({"Mac"})
PLATFORM_FORBIDDEN_TERMS = {
    "ios": ("Android", "Google Play", "Play Store", *MACOS_TERMS),
    "android": ("App Store", "TestFlight", *MACOS_TERMS),
}
VERSION_PATTERN = re.compile(r"^v?(\d+\.\d+\.\d+)$")
# The desktop train is CalVer `YYYY.M.P` after 3.9.0 (docs/architecture/desktop-release.md
# § Version numbers). No leading zeros: Cargo reads the workspace version as
# SemVer, which refuses them.
DESKTOP_VERSION_PATTERN = re.compile(
    r"^(?P<year>[1-9]\d{3})\.(?:[1-9]|1[0-2])\.(?:0|[1-9]\d*)$"
)
FIRST_DESKTOP_CALVER_YEAR = 2026
# `MAJOR*10000 + MINOR*100 + PATCH` only stays collision-free while each of the
# lower two components fits its own decimal field: 3.1.100 and 3.2.0 both derive
# to 30200, and two releases sharing a build version is an Installer that
# silently refuses to upgrade.
MAX_MACOS_VERSION_COMPONENT = 99

# The files that hold a release train's version number. Everything else
# — the two iOS Info.plists, the macOS package name, the update manifests, the
# Windows binaries' VERSIONINFO, both About screens — derives from one of these
# at build or publish time.
ANDROID_GRADLE_FILE = "android/app/build.gradle.kts"
IOS_PROJECT_FILE = "ios/TaigiKeyboard.xcodeproj/project.pbxproj"
MACOS_INFO_PLIST_FILE = "macos/App/Info.plist"
WINDOWS_CARGO_FILE = "windows/Cargo.toml"
# The desktop train's other Cargo workspaces: the crates Windows and Linux share
# (`desktop/`) carry the same number so a crate never reports a version its
# installer does not (docs/architecture/linux-roadmap.md L2 / L11), and
# `linux/` names the `.deb` and its control `Version` (`linux/Makefile`), and
# `macos/` holds the static library the macOS input method links
# (docs/architecture/macos-desktop-core-roadmap.md D7).
DESKTOP_SHARED_CARGO_FILE = "desktop/Cargo.toml"
LINUX_CARGO_FILE = "linux/Cargo.toml"
MACOS_CARGO_FILE = "macos/Cargo.toml"
DESKTOP_CARGO_FILES = (
    WINDOWS_CARGO_FILE,
    DESKTOP_SHARED_CARGO_FILE,
    LINUX_CARGO_FILE,
    MACOS_CARGO_FILE,
)
# The ChromeOS input method's extension manifest: the Chrome Web Store reads
# its `version`, and `chromeos-build.yml` names the draft it attaches to from it.
CHROMEOS_MANIFEST_FILE = "web/chromeos/manifest.json"
# The top-level `"version": "…"` line; `manifest_version` and
# `minimum_chrome_version` do not match because of the leading quote.
CHROMEOS_MANIFEST_VERSION_PATTERN = re.compile(
    r'^\s*"version"\s*:\s*"(?P<value>[^"]*)"', re.MULTILINE
)
# Off the train, with its own version, but its lockfile pins the desktop crates.
DESKTOP_LINKED_CARGO_FILES = ("web/Cargo.toml",)
# Which files a train owns. A version write touches exactly one train's files
# and leaves the other train's alone (USER 2026-08-29: mobile and desktop are
# numbered separately so each can ship on its own cadence).
TRAIN_FILES = {
    "mobile": (ANDROID_GRADLE_FILE, IOS_PROJECT_FILE),
    "desktop": (MACOS_INFO_PLIST_FILE, *DESKTOP_CARGO_FILES, CHROMEOS_MANIFEST_FILE),
}
TRAIN_CHOICES = tuple(TRAIN_FILES)
# Each shipping iOS target carries a Debug and a Release build-settings block, so
# its bundle identifier appears in exactly two. The test target keeps its own
# `MARKETING_VERSION = 1.0` and is deliberately absent from this mapping.
IOS_SHIPPING_BLOCK_COUNTS = {
    "com.siansiansu.TaigiKeyboard": 2,
    "com.siansiansu.TaigiKeyboard.TaigiKeyboardExtension": 2,
}
IOS_BUILD_SETTINGS_PATTERN = re.compile(
    r"buildSettings = \{(?P<body>.*?)\n\s*\};", re.DOTALL
)
ANDROID_VERSION_NAME_PATTERN = re.compile(
    r'^\s*versionName\s*=\s*"(?P<value>[^"]*)"', re.MULTILINE
)
# The workspace version: the first `version = "…"` inside `[workspace.package]`
# (every Windows crate inherits it with `version.workspace = true`).
WINDOWS_WORKSPACE_VERSION_PATTERN = re.compile(
    r'^\[workspace\.package\]\n(?:(?!\[)[^\n]*\n)*?version = "(?P<value>[^"]*)"',
    re.MULTILINE,
)
# The iOS build number is a constant: App Store Connect numbers the uploads of a
# marketing version itself, so nothing here has to track them.
IOS_BUILD_NUMBER = "1"


def forbidden_term_pattern(term: str) -> str:
    tail = "" if term in WHOLE_WORD_ONLY_TERMS else r"\w*"
    return rf"\b{re.escape(term)}{tail}\b"


class ReleaseNotesError(ValueError):
    """Raised when release-note input cannot be published safely."""


@dataclass(frozen=True)
class PlatformNotes:
    platform: str
    entries: tuple[str, ...]

    @property
    def store_text(self) -> str:
        return "\n".join(f"• {entry}" for entry in self.entries)


def normalize_version(raw_version: str) -> str:
    match = VERSION_PATTERN.fullmatch(raw_version)
    if match is None:
        raise ReleaseNotesError(
            f"version must use vMAJOR.MINOR.PATCH or MAJOR.MINOR.PATCH: {raw_version!r}",
        )
    return match.group(1)


def require_desktop_calver(version: str) -> None:
    """Refuse a desktop version that is not `YYYY.M.P`; takes a normalized version."""
    match = DESKTOP_VERSION_PATTERN.fullmatch(version)
    if match is None or int(match["year"]) < FIRST_DESKTOP_CALVER_YEAR:
        raise ReleaseNotesError(
            f"desktop version must be YYYY.M.P — year {FIRST_DESKTOP_CALVER_YEAR} or later, "
            f"month 1-12, no leading zeros (e.g. 2026.12.0, patch 2026.12.1): {version!r}",
        )


def notes_path(repo_root: Path, version: str, platform: str) -> Path:
    return repo_root / "changelog" / "store" / f"v{version}" / f"{platform}.txt"


def load_notes(repo_root: Path, version: str, platform: str) -> PlatformNotes:
    path = notes_path(repo_root, version, platform)
    try:
        raw_text = path.read_text(encoding="utf-8")
    except FileNotFoundError as error:
        raise ReleaseNotesError(f"missing canonical release notes: {path}") from error

    entries = tuple(line.strip() for line in raw_text.splitlines() if line.strip())
    notes = PlatformNotes(platform=platform, entries=entries)
    validate_notes(notes, path)
    return notes


def check_notes(repo_root: Path, version: str) -> None:
    """Both mobile platforms' canonical notes exist and validate."""
    for platform in ("ios", "android"):
        load_notes(repo_root, version, platform)


def validate_notes(notes: PlatformNotes, path: Path | None = None) -> None:
    label = str(path) if path is not None else notes.platform
    if not 1 <= len(notes.entries) <= MAX_ENTRIES:
        raise ReleaseNotesError(
            f"{label}: expected 1-{MAX_ENTRIES} non-empty entries"
        )

    for index, entry in enumerate(notes.entries, start=1):
        if entry.startswith(("-", "*", "•")):
            raise ReleaseNotesError(
                f"{label}:{index}: omit bullet markers; store formatting is generated",
            )
        if not entry.startswith(ALLOWED_PREFIXES):
            allowed = ", ".join(ALLOWED_PREFIXES)
            raise ReleaseNotesError(
                f"{label}:{index}: entry must start with one of {allowed}"
            )
        if not entry.endswith((".", "!", "?")):
            raise ReleaseNotesError(f"{label}:{index}: entry must end with punctuation")
        if re.search(r"https?://|www\.", entry, flags=re.IGNORECASE):
            raise ReleaseNotesError(
                f"{label}:{index}: URLs do not belong in What's New"
            )
        if re.search(r"(?:#\d+|PR\s*#?\d+)", entry, flags=re.IGNORECASE):
            raise ReleaseNotesError(f"{label}:{index}: remove issue and PR references")

        lowercase_entry = entry.casefold()
        for phrase in FORBIDDEN_MARKETING_PHRASES:
            if phrase.casefold() in lowercase_entry:
                raise ReleaseNotesError(
                    f"{label}:{index}: prohibited marketing phrase {phrase!r}",
                )
        for term in PLATFORM_FORBIDDEN_TERMS[notes.platform]:
            if re.search(forbidden_term_pattern(term), entry, flags=re.IGNORECASE):
                raise ReleaseNotesError(
                    f"{label}:{index}: {notes.platform} notes must not mention {term!r}",
                )

    if len(notes.store_text) > MAX_STORE_CHARACTERS:
        raise ReleaseNotesError(
            f"{label}: rendered What's New is {len(notes.store_text)} characters; "
            f"maximum is {MAX_STORE_CHARACTERS}",
        )


def read_text_file(repo_root: Path, relative_path: str) -> str:
    path = repo_root / relative_path
    try:
        return path.read_text(encoding="utf-8")
    except FileNotFoundError as error:
        raise ReleaseNotesError(f"missing {relative_path}: {path}") from error
    except (OSError, UnicodeDecodeError) as error:
        raise ReleaseNotesError(f"cannot read {path}: {error}") from error


def _ios_setting_pattern(name: str) -> re.Pattern[str]:
    return re.compile(rf"\n\s*{name} = (?P<value>[^;\n]*);")


def _plist_value_pattern(key: str) -> re.Pattern[str]:
    return re.compile(
        rf"<key>{re.escape(key)}</key>\s*<string>(?P<value>[^<]*)</string>"
    )


def _require_xml_plist(plist_source: str) -> None:
    """A binary plist has no text to edit around, and rewriting it through
    plistlib would drop the comments this file is written to carry."""
    if not plist_source.lstrip().startswith("<?xml"):
        raise ReleaseNotesError(
            f"{MACOS_INFO_PLIST_FILE} is not XML text; convert it with "
            "`plutil -convert xml1` before setting a version",
        )


def _plist_value(plist_source: str, key: str) -> str:
    return _sole_match(
        _plist_value_pattern(key),
        plist_source,
        f"{key} entry in {MACOS_INFO_PLIST_FILE}",
    ).group("value")


def _sole_match(
    pattern: re.Pattern[str], source: str, description: str
) -> re.Match[str]:
    matches = list(pattern.finditer(source))
    if len(matches) != 1:
        raise ReleaseNotesError(
            f"expected exactly one {description}; found {len(matches)}",
        )
    return matches[0]


def _replaced_value(match: re.Match[str], source: str, new_value: str) -> str:
    """`source` with this match's `value` group swapped; every other byte kept."""
    return source[: match.start("value")] + new_value + source[match.end("value") :]


def parse_workspace_version(cargo_source: str, cargo_file: str = WINDOWS_CARGO_FILE) -> str:
    return _sole_match(
        WINDOWS_WORKSPACE_VERSION_PATTERN,
        cargo_source,
        f"[workspace.package] version in {cargo_file}",
    ).group("value")


def render_workspace_cargo(
    cargo_source: str, version: str, cargo_file: str = WINDOWS_CARGO_FILE
) -> str:
    return _replaced_value(
        _sole_match(
            WINDOWS_WORKSPACE_VERSION_PATTERN,
            cargo_source,
            f"[workspace.package] version in {cargo_file}",
        ),
        cargo_source,
        version,
    )


def _chromeos_manifest_version_match(manifest_source: str) -> re.Match[str]:
    return _sole_match(
        CHROMEOS_MANIFEST_VERSION_PATTERN,
        manifest_source,
        f'"version" entry in {CHROMEOS_MANIFEST_FILE}',
    )


def parse_chromeos_manifest_version(manifest_source: str) -> str:
    """The manifest's `version` as the store reads it: parsed as JSON, so a
    file a rewrite broke fails here rather than at upload."""
    try:
        manifest = json.loads(manifest_source)
    except json.JSONDecodeError as error:
        raise ReleaseNotesError(f"{CHROMEOS_MANIFEST_FILE} is not valid JSON: {error}") from error
    version = manifest.get("version") if isinstance(manifest, dict) else None
    if not isinstance(version, str):
        raise ReleaseNotesError(f"{CHROMEOS_MANIFEST_FILE} has no string \"version\"")
    return version


def render_chromeos_manifest(manifest_source: str, version: str) -> str:
    """Swap the version in the manifest text, keeping its formatting (a JSON
    dump would rewrite every line)."""
    return _replaced_value(
        _chromeos_manifest_version_match(manifest_source), manifest_source, version
    )


# The Windows workspace is the desktop train's source of truth; the names stay.
parse_windows_version = parse_workspace_version
render_windows_cargo = render_workspace_cargo


def parse_android_version_name(gradle_source: str) -> str:
    return _sole_match(
        ANDROID_VERSION_NAME_PATTERN,
        gradle_source,
        f"versionName assignment in {ANDROID_GRADLE_FILE}",
    ).group("value")


@dataclass(frozen=True)
class IOSProjectVersions:
    """What the shipping targets' build-settings blocks declare today."""

    marketing_version: str
    build_number: str


def parse_ios_project_versions(project_source: str) -> IOSProjectVersions:
    """Read MARKETING_VERSION and CURRENT_PROJECT_VERSION off the shipping targets.

    Raises unless every shipping bundle identifier owns exactly the expected
    number of build-settings blocks and each of those blocks declares each
    setting exactly once — the shape both the checker and the writer rely on.
    """
    blocks = list(IOS_BUILD_SETTINGS_PATTERN.finditer(project_source))
    marketing_versions: list[str] = []
    build_numbers: list[str] = []
    for bundle_id, expected_count in IOS_SHIPPING_BLOCK_COUNTS.items():
        matching_blocks = [
            match
            for match in blocks
            if f"PRODUCT_BUNDLE_IDENTIFIER = {bundle_id};" in match.group("body")
        ]
        if len(matching_blocks) != expected_count:
            raise ReleaseNotesError(
                f"expected {expected_count} iOS build settings blocks for {bundle_id}; "
                f"found {len(matching_blocks)}",
            )
        for match in matching_blocks:
            body = match.group("body")
            marketing_versions.append(
                _sole_match(
                    _ios_setting_pattern("MARKETING_VERSION"),
                    body,
                    f"MARKETING_VERSION in a {bundle_id} build-settings block",
                ).group("value")
            )
            build_numbers.append(
                _sole_match(
                    _ios_setting_pattern("CURRENT_PROJECT_VERSION"),
                    body,
                    f"CURRENT_PROJECT_VERSION in a {bundle_id} build-settings block",
                ).group("value")
            )
    # The App Store treats the app and its keyboard extension as one upload, so
    # either setting differing between them is rejected at submission.
    distinct_marketing_versions = sorted(set(marketing_versions))
    if len(distinct_marketing_versions) != 1:
        raise ReleaseNotesError(
            f"iOS shipping targets declare different MARKETING_VERSION values: {distinct_marketing_versions}",
        )
    distinct_build_numbers = sorted(set(build_numbers))
    if len(distinct_build_numbers) != 1:
        raise ReleaseNotesError(
            f"iOS app and extension CURRENT_PROJECT_VERSION values differ: {distinct_build_numbers}",
        )
    return IOSProjectVersions(
        distinct_marketing_versions[0], distinct_build_numbers[0]
    )


def check_mobile_versions_in_sources(
    gradle_source: str,
    project_source: str,
    version: str,
) -> None:
    """Hold the two already-loaded mobile project files to one version.

    Taking sources rather than a repo root is what lets `set_project_versions`
    run the real gate over the rewrite it is about to make, instead of writing
    first and checking afterwards.
    """
    android_version = parse_android_version_name(gradle_source)
    if android_version != version:
        raise ReleaseNotesError(
            f"Android versionName is {android_version}; expected {version}"
        )

    ios_versions = parse_ios_project_versions(project_source)
    if ios_versions.marketing_version != version:
        raise ReleaseNotesError(
            f"iOS MARKETING_VERSION is {ios_versions.marketing_version}; expected {version}",
        )


def check_desktop_versions_in_sources(
    macos_plist: dict,
    macos_plist_path: Path,
    cargo_sources: dict[str, str],
    chromeos_manifest_source: str,
    version: str,
) -> None:
    """Hold the already-loaded desktop project files to one version."""
    check_macos_plist_values(macos_plist, version, macos_plist_path)
    for cargo_file in DESKTOP_CARGO_FILES:
        workspace_version = parse_workspace_version(cargo_sources[cargo_file], cargo_file)
        if workspace_version != version:
            raise ReleaseNotesError(
                f"{cargo_file} workspace version is {workspace_version}; expected {version}"
            )
    manifest_version = parse_chromeos_manifest_version(chromeos_manifest_source)
    if manifest_version != version:
        raise ReleaseNotesError(
            f"{CHROMEOS_MANIFEST_FILE} version is {manifest_version}; expected {version}"
        )


def check_project_versions(repo_root: Path, version: str, train: str) -> None:
    """Hold one train's project files to `version`; the other train is not read."""
    if train == "mobile":
        check_mobile_versions_in_sources(
            read_text_file(repo_root, ANDROID_GRADLE_FILE),
            read_text_file(repo_root, IOS_PROJECT_FILE),
            version,
        )
        return
    if train == "desktop":
        macos_plist_path, macos_plist = load_macos_plist(repo_root)
        check_desktop_versions_in_sources(
            macos_plist,
            macos_plist_path,
            {cargo_file: read_text_file(repo_root, cargo_file) for cargo_file in DESKTOP_CARGO_FILES},
            read_text_file(repo_root, CHROMEOS_MANIFEST_FILE),
            version,
        )
        return
    raise ReleaseNotesError(f"unknown release train {train!r}; expected one of {TRAIN_CHOICES}")


def macos_build_version(version: str) -> str:
    """The dotted-integer package version `MAJOR.MINOR.PATCH` derives into.

    Mirrors `macos/scripts/release-app.sh`, which enforces the same rule at
    package time. Duplicated rather than shelled out to because this gate runs
    before any macOS build exists.

    Takes a bare `MAJOR.MINOR.PATCH` — run `normalize_version` on anything that
    came from a caller.
    """
    match = VERSION_PATTERN.fullmatch(version)
    if match is None:
        raise ReleaseNotesError(
            f"version must use MAJOR.MINOR.PATCH: {version!r}",
        )
    major, minor, patch = (int(part) for part in match.group(1).split("."))
    for name, component in (("minor", minor), ("patch", patch)):
        if component > MAX_MACOS_VERSION_COMPONENT:
            raise ReleaseNotesError(
                f"{version}: {name} version {component} exceeds "
                f"{MAX_MACOS_VERSION_COMPONENT}; the macOS build version "
                f"MAJOR*10000 + MINOR*100 + PATCH would collide with another release",
            )
    return str(major * 10_000 + minor * 100 + patch)


def _plist_dictionary(raw_plist: bytes, path: Path) -> dict:
    """The plist's root dictionary, or a message naming what is wrong with it."""
    try:
        plist = plistlib.loads(raw_plist)
    except Exception as error:
        raise ReleaseNotesError(f"{path} is not a readable plist: {error}") from error

    # A plist root may be any property-list type; only a dictionary is an
    # Info.plist, and `.get` on a list would surface as an AttributeError.
    if not isinstance(plist, dict):
        raise ReleaseNotesError(
            f"{path} does not contain a dictionary at its root",
        )
    return plist


def load_macos_plist(repo_root: Path) -> tuple[Path, dict]:
    path = repo_root / MACOS_INFO_PLIST_FILE
    try:
        raw_plist = path.read_bytes()
    except FileNotFoundError as error:
        raise ReleaseNotesError(f"missing macOS Info.plist: {path}") from error
    except OSError as error:
        raise ReleaseNotesError(
            f"cannot read macOS Info.plist {path}: {error}",
        ) from error
    return path, _plist_dictionary(raw_plist, path)


def check_macos_version(repo_root: Path, version: str) -> None:
    """Hold `macos/App/Info.plist` to the desktop train's version.

    macOS and Windows ship one version number. macOS is released separately —
    its own script, its own GitHub release — so nothing else fails when its
    plist is left behind, and a stale `CFBundleVersion` is an Installer that
    silently refuses to upgrade.
    """
    path, plist = load_macos_plist(repo_root)
    check_macos_plist_values(plist, version, path)


def check_macos_plist_values(plist: dict, version: str, path: Path) -> None:
    short_version = plist.get("CFBundleShortVersionString")
    if short_version != version:
        actual = short_version if short_version is not None else "missing"
        raise ReleaseNotesError(
            f"macOS CFBundleShortVersionString is {actual}; expected {version}",
        )

    expected_build = macos_build_version(version)
    build_version = plist.get("CFBundleVersion")
    if build_version != expected_build:
        actual = build_version if build_version is not None else "missing"
        raise ReleaseNotesError(
            f"macOS CFBundleVersion is {actual}; expected {expected_build} "
            f"(MAJOR*10000 + MINOR*100 + PATCH of {version})",
        )


def render_android_gradle(gradle_source: str, version: str) -> str:
    return _replaced_value(
        _sole_match(
            ANDROID_VERSION_NAME_PATTERN,
            gradle_source,
            f"versionName assignment in {ANDROID_GRADLE_FILE}",
        ),
        gradle_source,
        version,
    )


def render_ios_project(project_source: str, version: str) -> str:
    """Rewrite the shipping blocks' two version settings, every other byte kept.

    The test target's `MARKETING_VERSION = 1.0` is not a release version, so the
    rewrite is scoped by bundle identifier rather than applied to the file.
    """
    def rewrite_block(match: re.Match[str]) -> str:
        block = match.group(0)
        if not any(
            f"PRODUCT_BUNDLE_IDENTIFIER = {bundle_id};" in block
            for bundle_id in IOS_SHIPPING_BLOCK_COUNTS
        ):
            return block
        for name, value in (
            ("MARKETING_VERSION", version),
            ("CURRENT_PROJECT_VERSION", IOS_BUILD_NUMBER),
        ):
            block = _replaced_value(
                _sole_match(_ios_setting_pattern(name), block, name), block, value
            )
        return block

    return IOS_BUILD_SETTINGS_PATTERN.sub(rewrite_block, project_source)


def render_macos_plist(plist_source: str, version: str) -> str:
    """Swap both version values in the plist text.

    Text, not `plistlib.dump` or `PlistBuddy`: both rebuild the file and drop the
    hand-written XML comments, including the one above these very keys.
    """
    _require_xml_plist(plist_source)
    rendered = plist_source
    for key, value in (
        ("CFBundleShortVersionString", version),
        ("CFBundleVersion", macos_build_version(version)),
    ):
        rendered = _replaced_value(
            _sole_match(
                _plist_value_pattern(key),
                rendered,
                f"{key} entry in {MACOS_INFO_PLIST_FILE}",
            ),
            rendered,
            value,
        )
    return rendered


def _reject_downgrade(current_version: str, version: str) -> None:
    match = VERSION_PATTERN.fullmatch(current_version)
    if match is None:
        return
    current = tuple(int(part) for part in match.group(1).split("."))
    if tuple(int(part) for part in version.split(".")) < current:
        raise ReleaseNotesError(
            f"{version} is lower than the {current_version} already in the tree; "
            "stores refuse a version that goes backwards — pass --allow-downgrade "
            "if that is deliberate",
        )


def _write_atomically(path: Path, content: str) -> None:
    """Replace `path` in one step, keeping its permission bits.

    A temporary file is created 0600, and `os.replace` carries that mode onto the
    destination — so the mode has to be copied back before the swap, or every run
    would quietly turn the project files owner-only.
    """
    original_mode = stat.S_IMODE(path.stat().st_mode)
    handle = tempfile.NamedTemporaryFile(
        "w", encoding="utf-8", newline="", dir=path.parent, delete=False
    )
    try:
        with handle:
            handle.write(content)
        os.chmod(handle.name, original_mode)
        os.replace(handle.name, path)
    except OSError:
        Path(handle.name).unlink(missing_ok=True)
        raise


def _restore_files(originals: dict[Path, str], written: Iterable[Path]) -> list[Path]:
    """Put back what a failed run already wrote; name whatever could not be put back."""
    unrestored: list[Path] = []
    for path in written:
        try:
            _write_atomically(path, originals[path])
        except OSError:
            unrestored.append(path)
    return unrestored


def set_project_versions(
    repo_root: Path,
    version: str,
    train: str,
    allow_downgrade: bool = False,
) -> tuple[str, ...]:
    """Write `version` into one train's two project files, or into neither.

    Accepts `vMAJOR.MINOR.PATCH` or `MAJOR.MINOR.PATCH`; the desktop train
    takes only `YYYY.M.P` (`require_desktop_calver`). The rewritten contents
    are held to the same gate the release flow runs, before any real file is
    touched, so a version that gate would reject never reaches the tree. The
    other train's files are never read, let alone written.

    Two files cannot be replaced in one filesystem transaction. A write that
    fails part-way is rolled back; a rollback that also fails raises with the
    files it could not put back named in the message.
    """
    version = normalize_version(version)
    if train not in TRAIN_FILES:
        raise ReleaseNotesError(
            f"unknown release train {train!r}; expected one of {TRAIN_CHOICES}"
        )
    if train == "desktop":
        require_desktop_calver(version)

    sources = {
        relative_path: read_text_file(repo_root, relative_path)
        for relative_path in TRAIN_FILES[train]
    }

    if train == "mobile":
        current_android = parse_android_version_name(sources[ANDROID_GRADLE_FILE])
        current_ios = parse_ios_project_versions(sources[IOS_PROJECT_FILE])
        current_versions = (current_android, current_ios.marketing_version)
        candidates = {
            ANDROID_GRADLE_FILE: render_android_gradle(
                sources[ANDROID_GRADLE_FILE], version
            ),
            IOS_PROJECT_FILE: render_ios_project(sources[IOS_PROJECT_FILE], version),
        }
        changes = (
            f"Android: versionName {current_android} -> {version}",
            f"iOS: MARKETING_VERSION {current_ios.marketing_version} -> {version}, "
            f"CURRENT_PROJECT_VERSION {current_ios.build_number} -> {IOS_BUILD_NUMBER}",
        )
    else:
        plist_source = sources[MACOS_INFO_PLIST_FILE]
        _require_xml_plist(plist_source)
        current_macos = _plist_value(plist_source, "CFBundleShortVersionString")
        current_macos_build = _plist_value(plist_source, "CFBundleVersion")
        current_workspaces = {
            cargo_file: parse_workspace_version(sources[cargo_file], cargo_file)
            for cargo_file in DESKTOP_CARGO_FILES
        }
        current_chromeos = parse_chromeos_manifest_version(sources[CHROMEOS_MANIFEST_FILE])
        current_versions = (current_macos, *current_workspaces.values(), current_chromeos)
        candidates = {
            MACOS_INFO_PLIST_FILE: render_macos_plist(plist_source, version),
            **{
                cargo_file: render_workspace_cargo(sources[cargo_file], version, cargo_file)
                for cargo_file in DESKTOP_CARGO_FILES
            },
            CHROMEOS_MANIFEST_FILE: render_chromeos_manifest(
                sources[CHROMEOS_MANIFEST_FILE], version
            ),
        }
        changes = (
            f"macOS: CFBundleShortVersionString {current_macos} -> {version}, "
            f"CFBundleVersion {current_macos_build} -> {macos_build_version(version)}",
            *(
                f"{cargo_file}: workspace version {current} -> {version}"
                for cargo_file, current in current_workspaces.items()
            ),
            f"{CHROMEOS_MANIFEST_FILE}: version {current_chromeos} -> {version}",
        )

    # A train's number never goes backwards: the stores refuse a lower mobile
    # version, and a lower desktop version is a macOS Installer that refuses
    # the upgrade (`CFBundleVersion` derives from it) plus a Windows dictionary
    # stamp that goes back in time.
    if not allow_downgrade:
        for current_version in current_versions:
            _reject_downgrade(current_version, version)

    # Validate before writing: the rewrite runs through the same gate the release
    # flow runs, so the real tree never holds a version that gate would reject.
    # Parsing the rendered plist here also proves the text edit kept it a plist.
    if train == "mobile":
        check_mobile_versions_in_sources(
            candidates[ANDROID_GRADLE_FILE], candidates[IOS_PROJECT_FILE], version
        )
    else:
        macos_plist_path = repo_root / MACOS_INFO_PLIST_FILE
        check_desktop_versions_in_sources(
            _plist_dictionary(
                candidates[MACOS_INFO_PLIST_FILE].encode("utf-8"), macos_plist_path
            ),
            macos_plist_path,
            {cargo_file: candidates[cargo_file] for cargo_file in DESKTOP_CARGO_FILES},
            candidates[CHROMEOS_MANIFEST_FILE],
            version,
        )

    originals = {repo_root / relative_path: content for relative_path, content in sources.items()}
    written: list[Path] = []
    try:
        for relative_path, content in candidates.items():
            path = repo_root / relative_path
            _write_atomically(path, content)
            written.append(path)
    except OSError as error:
        # Two files cannot be replaced in one filesystem transaction; restoring
        # what already landed is what keeps a failed run from leaving the train
        # split across two versions. Whatever stopped the write can stop the
        # restore too, so say which files that left behind rather than claim a
        # rollback that did not happen.
        unrestored = _restore_files(originals, written)
        outcome = (
            "restoring them failed too — "
            f"{', '.join(str(path.relative_to(repo_root)) for path in unrestored)} "
            f"still hold {version} and must be checked by hand"
            if unrestored
            else "the tree was restored"
        )
        raise ReleaseNotesError(
            f"could not write the project files: {error}; {outcome}",
        ) from error

    return changes


# Desktop changelog fragments. Each pull request that changes what a desktop user sees carries one fragment,
# `changelog/unreleased/<slug>.md`; a release only assembles them into
# `changelog/desktop-v<version>.md` (docs/architecture/release-automation-roadmap.md
# D2: no model writes release notes). Format: changelog/unreleased/README.md.
FRAGMENTS_DIR = "changelog/unreleased"
FRAGMENTS_README = "README.md"
CHANGELOG_INDEX_FILE = "CHANGELOG.md"
DESKTOP_CHANGELOG_HEADING = "## Desktop — macOS + Windows + Linux + ChromeOS"
# Release-page order. Keys are what a fragment writes; values are the headings.
DESKTOP_PLATFORMS = {
    "macos": "macOS",
    "windows": "Windows",
    "linux": "Linux",
    "chromeos": "ChromeOS",
}
# Release-page order of the `####` headings under each platform.
DESKTOP_SECTIONS = (
    "Input",
    "Phonetic Symbols",
    "Candidates",
    "Candidate Window",
    "Settings",
    "Appearance",
    "Updates",
    "Install",
)
# Paths whose change reaches a desktop installer: the four shells, the crates
# they share, the engine, the shipped data (`assets/` — a `dictionary/` source
# edit ships only through the artifacts it regenerates there), the UI strings,
# and what `web/Makefile`'s `chromeos` target packs: the extension, the wasm
# crates, and the few `web/src/` files it copies (the web editor stays out).
DESKTOP_SHIPPING_PREFIXES = (
    "macos/",
    "windows/",
    "linux/",
    "desktop/",
    "engine/",
    "assets/",
    "i18n/",
    "web/chromeos/",
    "web/crates/",
    "web/Cargo.",
    "web/Makefile",
    "web/scripts/stage-chromeos.mjs",
    "web/src/settings.js",
    "web/src/i18n.js",
    "web/src/learning.js",
    "web/src/style.css",
    "web/src/chevron.svg",
    "web/src/index.html",
    "web/src/generated/",
)
# Inside those trees: the mobile-only engine bridges, tests and docs ship to no
# desktop user.
DESKTOP_NON_SHIPPING_PREFIXES = (
    "engine/swift-ffi/",
    "engine/android-jni/",
    "engine/test-support/",
)
FRAGMENT_PLATFORMS_PATTERN = re.compile(r"^\[(?P<items>[^\]]*)\]$")
# A bullet that already names its pull request(s) keeps them: `(#12)`, `(#12, #15)`.
PULL_REQUEST_REFERENCE_PATTERN = re.compile(r"\(#(?P<first>\d+)(?:, #\d+)*\)$")
# How a pull request lands on main's first-parent line: a merge commit, or a
# squash commit whose subject GitHub suffixes with the number.
PULL_REQUEST_SUBJECT_PATTERNS = (
    re.compile(r"^Merge pull request #(?P<number>\d+) "),
    re.compile(r"\(#(?P<number>\d+)\)$"),
)


@dataclass(frozen=True)
class ChangelogFragment:
    name: str
    platforms: tuple[str, ...]
    section: str
    bullet: str

    def render(self) -> str:
        return (
            f"---\nplatforms: [{', '.join(self.platforms)}]\nsection: {self.section}\n---\n"
            f"{self.bullet}\n"
        )


def parse_fragment(name: str, text: str) -> ChangelogFragment:
    """Read one fragment, refusing anything outside the format in the README."""
    def fail(reason: str) -> ReleaseNotesError:
        return ReleaseNotesError(f"{FRAGMENTS_DIR}/{name}: {reason}")

    lines = text.splitlines()
    if not lines or lines[0] != "---":
        raise fail("must start with a `---` front-matter line")
    try:
        closing = lines.index("---", 1)
    except ValueError:
        raise fail("front matter has no closing `---`") from None
    fields: dict[str, str] = {}
    for line in lines[1:closing]:
        key, separator, value = line.partition(":")
        key = key.strip()
        if not separator or key not in ("platforms", "section") or key in fields:
            raise fail(f"front matter takes `platforms:` and `section:` once each, got {line!r}")
        fields[key] = value.strip()
    if set(fields) != {"platforms", "section"}:
        raise fail("front matter needs both `platforms:` and `section:`")

    match = FRAGMENT_PLATFORMS_PATTERN.fullmatch(fields["platforms"])
    platforms = tuple(item.strip() for item in match["items"].split(",")) if match else ()
    if not platforms or any(platform not in DESKTOP_PLATFORMS for platform in platforms):
        raise fail(
            f"platforms must be a list like [macos, linux] drawn from {list(DESKTOP_PLATFORMS)}, "
            f"got {fields['platforms']!r}",
        )
    if len(set(platforms)) != len(platforms):
        raise fail(f"platforms lists one twice: {fields['platforms']!r}")
    if fields["section"] not in DESKTOP_SECTIONS:
        raise fail(f"section must be one of {list(DESKTOP_SECTIONS)}, got {fields['section']!r}")

    body = "\n".join(lines[closing + 1 :]).strip("\n").split("\n")
    if not body or not body[0].startswith("- ") or not body[0][2:].strip():
        raise fail("body must be one `- ` bullet")
    if any(not line.startswith("  ") or not line.strip() for line in body[1:]):
        raise fail("body is one bullet: continuation lines are indented two spaces, no blank lines")
    return ChangelogFragment(
        name=name,
        platforms=tuple(platform for platform in DESKTOP_PLATFORMS if platform in platforms),
        section=fields["section"],
        bullet="\n".join(body),
    )


def load_fragments(repo_root: Path) -> list[ChangelogFragment]:
    directory = repo_root / FRAGMENTS_DIR
    if not directory.is_dir():
        return []
    return [
        parse_fragment(path.name, path.read_text(encoding="utf-8"))
        for path in sorted(directory.glob("*.md"))
        if path.name != FRAGMENTS_README
    ]


def is_desktop_shipping_path(path: str) -> bool:
    if not path.startswith(DESKTOP_SHIPPING_PREFIXES):
        return False
    if path.startswith(DESKTOP_NON_SHIPPING_PREFIXES) or path.endswith(".md"):
        return False
    # `tests/` in Rust, `Tests/` in Swift.
    return "tests" not in (directory.lower() for directory in path.split("/")[:-1])


def _git(repo_root: Path, *arguments: str) -> str:
    result = subprocess.run(
        # quotepath off: a non-ASCII path comes back as itself, not octal-escaped.
        ["git", "-C", str(repo_root), "-c", "core.quotepath=false", *arguments],
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        raise ReleaseNotesError(f"git {' '.join(arguments)} failed: {result.stderr.strip()}")
    return result.stdout


def check_fragments(repo_root: Path, base: str | None = None) -> None:
    """Validate every fragment; with `base`, also require one when desktop code changed.

    `base` is the commit the change is compared against (on a pull request,
    the main it merges into).
    """
    load_fragments(repo_root)
    if base is None:
        return
    changed = _git(repo_root, "diff", "--name-only", "--no-renames", base, "HEAD").splitlines()
    shipping = [path for path in changed if is_desktop_shipping_path(path)]
    if not shipping:
        return
    added = _git(
        repo_root, "diff", "--name-only", "--no-renames", "--diff-filter=AM", base, "HEAD",
        "--", FRAGMENTS_DIR,
    ).splitlines()
    if any(path.endswith(".md") and Path(path).name != FRAGMENTS_README for path in added):
        return
    listed = ", ".join(shipping[:5]) + (" …" if len(shipping) > 5 else "")
    raise ReleaseNotesError(
        f"this change touches desktop-shipping files ({listed}) but adds no "
        f"{FRAGMENTS_DIR}/*.md fragment — add one ({FRAGMENTS_DIR}/{FRAGMENTS_README}), "
        "or label the pull request `no-changelog` if no desktop user can see it",
    )


def fragment_pull_request(repo_root: Path, fragment: ChangelogFragment) -> int | None:
    """The pull request whose landing on main's first-parent line added `fragment`."""
    subjects = _git(
        repo_root, "log", "--first-parent", "--no-renames", "--diff-filter=A", "--format=%s",
        "HEAD", "--", f"{FRAGMENTS_DIR}/{fragment.name}",
    ).splitlines()
    if not subjects:
        return None
    for pattern in PULL_REQUEST_SUBJECT_PATTERNS:
        match = pattern.search(subjects[0])
        if match:
            return int(match["number"])
    return None


def _join_names(names: list[str]) -> str:
    return names[0] if len(names) == 1 else f"{', '.join(names[:-1])} and {names[-1]}"


def _render_platform(platform: str, entries: list[tuple[str, str]]) -> list[str]:
    lines = [f"### {DESKTOP_PLATFORMS[platform]}", ""]
    if not entries:
        return [*lines, "No changes.", ""]
    for section in DESKTOP_SECTIONS:
        bullets = [bullet for entry_section, bullet in entries if entry_section == section]
        if bullets:
            lines += [f"#### {section}", "", *bullets, ""]
    return lines


def _insert_changelog_link(index_source: str, version: str, note: str) -> str:
    link = f"- [v{version}](changelog/desktop-v{version}.md)"
    if link in index_source:
        raise ReleaseNotesError(f"{CHANGELOG_INDEX_FILE} already links v{version}")
    lines = index_source.split("\n")
    try:
        heading = lines.index(DESKTOP_CHANGELOG_HEADING)
    except ValueError:
        raise ReleaseNotesError(
            f"{CHANGELOG_INDEX_FILE} has no {DESKTOP_CHANGELOG_HEADING!r} heading"
        ) from None
    position = next(
        (index for index in range(heading + 1, len(lines)) if lines[index].startswith(("- ", "## "))),
        len(lines),
    )
    if position == len(lines) or not lines[position].startswith("- "):
        raise ReleaseNotesError(f"{CHANGELOG_INDEX_FILE}: the desktop list is empty")
    lines.insert(position, link + note)
    return "\n".join(lines)


def _numbered_bullets(
    repo_root: Path, fragments: list[ChangelogFragment]
) -> list[tuple[ChangelogFragment, str]]:
    """Each fragment with its bullet ending in `(#N)`, in pull-request order.

    A bullet that already names its pull request keeps it; otherwise the number
    comes from git. Entries with no number go last, by file name.
    """
    numbered: list[tuple[int, ChangelogFragment, str]] = []
    for fragment in fragments:
        reference = PULL_REQUEST_REFERENCE_PATTERN.search(fragment.bullet)
        if reference:
            numbered.append((int(reference["first"]), fragment, fragment.bullet))
            continue
        number = fragment_pull_request(repo_root, fragment)
        if number is None:
            print(
                f"warning: {FRAGMENTS_DIR}/{fragment.name} names no pull request and "
                "none added it on main — the entry goes out without one",
                file=sys.stderr,
            )
            numbered.append((sys.maxsize, fragment, fragment.bullet))
        else:
            numbered.append((number, fragment, f"{fragment.bullet} (#{number})"))
    numbered.sort(key=lambda entry: (entry[0], entry[1].name))
    return [(fragment, bullet) for _, fragment, bullet in numbered]


def render_desktop_changelog(
    version: str, platform: str | None, bullets: list[tuple[ChangelogFragment, str]]
) -> str:
    """The release page: a lead line, then each shipped platform's sections."""
    platforms = [platform] if platform else list(DESKTOP_PLATFORMS)
    entries = {
        name: [(fragment.section, bullet) for fragment, bullet in bullets if name in fragment.platforms]
        for name in platforms
    }
    if platform:
        lead = f"A {DESKTOP_PLATFORMS[platform]}-only patch. The other platforms are unchanged."
    else:
        touched = [DESKTOP_PLATFORMS[name] for name in platforms if entries[name]]
        lead = f"Changes for {_join_names(touched)}."
    lines = [f"# desktop v{version}", "", lead, ""]
    for name in platforms:
        lines += _render_platform(name, entries[name])
    return "\n".join(lines).rstrip("\n") + "\n"


def assemble_desktop_changelog(
    repo_root: Path, version: str, platform: str | None = None
) -> tuple[str, ...]:
    """Write `changelog/desktop-v<version>.md` from the fragments and consume them.

    A full release (`YYYY.M.0`) takes every fragment and lists every platform.
    A patch takes `platform` alone: a fragment naming only it is deleted, one
    naming others too loses that platform and stays for the next full release,
    so a fix that already shipped is never announced twice. Returns the paths
    written or removed, relative to `repo_root`.
    """
    version = normalize_version(version)
    require_desktop_calver(version)
    is_patch = not version.endswith(".0")
    if is_patch and platform is None:
        raise ReleaseNotesError(f"{version} is a patch: pass --platform")
    if not is_patch and platform is not None:
        raise ReleaseNotesError(f"{version} is a full release: it ships every platform, drop --platform")
    target = repo_root / "changelog" / f"desktop-v{version}.md"
    if target.exists():
        raise ReleaseNotesError(f"{target.relative_to(repo_root)} already exists")
    if _git(repo_root, "rev-parse", "--is-shallow-repository").strip() == "true":
        raise ReleaseNotesError(
            "shallow clone: fragment pull request numbers come from history — fetch it in full"
        )

    fragments = [
        fragment for fragment in load_fragments(repo_root)
        if platform is None or platform in fragment.platforms
    ]
    if not fragments:
        scope = f"{DESKTOP_PLATFORMS[platform]} " if platform else ""
        raise ReleaseNotesError(f"no {scope}fragments in {FRAGMENTS_DIR}: nothing to release")
    changelog = render_desktop_changelog(version, platform, _numbered_bullets(repo_root, fragments))
    index_path = repo_root / CHANGELOG_INDEX_FILE
    index_note = f" — {DESKTOP_PLATFORMS[platform]}-only patch" if platform else ""
    index_source = _insert_changelog_link(
        index_path.read_text(encoding="utf-8"), version, index_note
    )

    # Everything is computed and checked above; a failure from here on leaves a
    # partial result that `git checkout -- . && git clean -f changelog` undoes.
    changes = [str(target.relative_to(repo_root)), CHANGELOG_INDEX_FILE]
    target.write_text(changelog, encoding="utf-8")
    _write_atomically(index_path, index_source)
    for fragment in fragments:
        path = repo_root / FRAGMENTS_DIR / fragment.name
        remaining = tuple(name for name in fragment.platforms if platform and name != platform)
        if remaining:
            _write_atomically(path, dataclasses.replace(fragment, platforms=remaining).render())
        else:
            path.unlink()
        changes.append(f"{FRAGMENTS_DIR}/{fragment.name}")
    return tuple(changes)


def _parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    def add_command(name: str, help_text: str, takes_version: bool = True) -> argparse.ArgumentParser:
        subparser = subparsers.add_parser(name, help=help_text)
        if takes_version:
            subparser.add_argument("--version", required=True)
        subparser.add_argument(
            "--repo-root", type=Path, default=Path(__file__).resolve().parents[1]
        )
        return subparser

    add_command("check", "Validate both platforms' canonical notes")
    check_versions = add_command(
        "check-versions", "Check one release train's project versions"
    )
    set_versions = add_command(
        "set-versions", "Write the version into one release train's project files"
    )
    for train_command in (check_versions, set_versions):
        train_command.add_argument(
            "--train",
            choices=TRAIN_CHOICES,
            required=True,
            help="mobile = iOS + Android; desktop = macOS + Windows + Linux + ChromeOS",
        )
    set_versions.add_argument(
        "--allow-downgrade",
        action="store_true",
        help="Permit a version lower than the one already in the tree",
    )
    printer = add_command("print", "Print one platform's store text")
    printer.add_argument("--platform", choices=("ios", "android"), required=True)
    fragment_checker = add_command(
        "check-fragments",
        "Validate the desktop changelog fragments in changelog/unreleased/",
        takes_version=False,
    )
    fragment_checker.add_argument(
        "--base",
        help="Also require a fragment when HEAD changes desktop-shipping files since this commit",
    )
    assembler = add_command(
        "assemble", "Write changelog/desktop-v<version>.md from the fragments and consume them"
    )
    assembler.add_argument("--train", choices=("desktop",), required=True)
    assembler.add_argument(
        "--platform",
        choices=tuple(DESKTOP_PLATFORMS),
        help="The one platform a patch (YYYY.M.P, P > 0) ships; omit for YYYY.M.0",
    )

    return parser.parse_args()


def refresh_desktop_lockfiles(repo_root: Path) -> None:
    """Rewrite the desktop crates' member versions in each lockfile that pins them.

    `--workspace` leaves every third-party pin alone. Without it the release
    commit carries stale member versions, and the first hosted build rewrites
    the lockfile — which its "changed nothing tracked" check refuses.
    """
    for cargo_file in (*DESKTOP_CARGO_FILES, *DESKTOP_LINKED_CARGO_FILES):
        result = subprocess.run(
            ["cargo", "update", "--workspace", "--quiet", "--manifest-path", str(repo_root / cargo_file)],
            check=False,
        )
        if result.returncode != 0:
            raise ReleaseNotesError(f"cargo update --workspace failed for {cargo_file}")


def main() -> int:
    args = _parse_args()
    try:
        repo_root = args.repo_root.resolve()
        if args.command == "check-fragments":
            check_fragments(repo_root, args.base)
            return 0
        if args.command == "assemble":
            for change in assemble_desktop_changelog(repo_root, args.version, args.platform):
                print(change)
            return 0
        version = normalize_version(args.version)
        if args.command == "check":
            check_notes(repo_root, version)
        elif args.command == "check-versions":
            check_project_versions(repo_root, version, args.train)
        elif args.command == "set-versions":
            for change in set_project_versions(
                repo_root, version, args.train, allow_downgrade=args.allow_downgrade
            ):
                print(change)
            if args.train == "desktop":
                refresh_desktop_lockfiles(repo_root)
        elif args.command == "print":
            print(load_notes(repo_root, version, args.platform).store_text)
        else:
            raise ReleaseNotesError(f"unhandled command {args.command!r}")
    except ReleaseNotesError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
