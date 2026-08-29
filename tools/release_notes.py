"""Validate store release notes, mirror them into platform version history, and
check or set the one version number iOS, Android, and macOS share."""

from __future__ import annotations

import argparse
import json
import os
import plistlib
import re
import stat
import sys
import tempfile
from collections.abc import Iterable
from dataclasses import dataclass
from pathlib import Path

MAX_STORE_CHARACTERS = 500
ALLOWED_PREFIXES = ("New:", "Fixed:", "Improved:", "Changed:", "Updated:")
FORBIDDEN_MARKETING_PHRASES = (
    "#1",
    "best app",
    "best keyboard",
    "coming soon",
    "free for a limited time",
)
# macOS ships its own release notes: `macos/scripts/publish-release.sh` hands
# `changelog/v<version>.md` to `gh release create`. The store notes are the
# mobile surface only, so macOS-only work belongs in the detailed changelog and
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
# `MAJOR*10000 + MINOR*100 + PATCH` only stays collision-free while each of the
# lower two components fits its own decimal field: 3.1.100 and 3.2.0 both derive
# to 30200, and two releases sharing a build version is an Installer that
# silently refuses to upgrade.
MAX_MACOS_VERSION_COMPONENT = 99

# The four files that hold the release train's version number. Everything else
# — the two iOS Info.plists, the macOS package name, the update manifests, the
# Windows binaries' VERSIONINFO, both About screens — derives from one of these
# at build or publish time.
ANDROID_GRADLE_FILE = "android/app/build.gradle.kts"
IOS_PROJECT_FILE = "ios/TaigiKeyboard.xcodeproj/project.pbxproj"
MACOS_INFO_PLIST_FILE = "macos/App/Info.plist"
WINDOWS_CARGO_FILE = "windows/Cargo.toml"
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


def validate_notes(notes: PlatformNotes, path: Path | None = None) -> None:
    label = str(path) if path is not None else notes.platform
    if not 1 <= len(notes.entries) <= 5:
        raise ReleaseNotesError(f"{label}: expected 1-5 non-empty entries")

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


def _swift_quoted(value: str) -> str:
    return json.dumps(value, ensure_ascii=False)


def _kotlin_quoted(value: str) -> str:
    # JSON string escaping is otherwise compatible with a Kotlin regular string,
    # but Kotlin treats an unescaped dollar sign as string interpolation.
    return json.dumps(value, ensure_ascii=False).replace("$", "\\$")


def render_swift_entry(
    version: str, release_date: str, entries: tuple[str, ...]
) -> str:
    changes = "".join(f"            {_swift_quoted(entry)},\n" for entry in entries)
    return (
        f"        ({_swift_quoted(version)}, {_swift_quoted(release_date)}, [\n"
        f"{changes}"
        "        ]),\n"
    )


def render_kotlin_entry(
    version: str, release_date: str, entries: tuple[str, ...]
) -> str:
    changes = "".join(
        f"                    {_kotlin_quoted(entry)},\n" for entry in entries
    )
    return (
        "            VersionEntry(\n"
        f"                {_kotlin_quoted(version)},\n"
        f"                {_kotlin_quoted(release_date)},\n"
        "                listOf(\n"
        f"{changes}"
        "                ),\n"
        "            ),\n"
    )


def _balanced_entry_span(source: str, start: int) -> tuple[int, int]:
    opening = source.find("(", start)
    if opening < 0:
        raise ReleaseNotesError("version-history entry has no opening parenthesis")

    depth = 0
    in_string = False
    escaped = False
    for index in range(opening, len(source)):
        character = source[index]
        if in_string:
            if escaped:
                escaped = False
            elif character == "\\":
                escaped = True
            elif character == '"':
                in_string = False
            continue
        if character == '"':
            in_string = True
        elif character == "(":
            depth += 1
        elif character == ")":
            depth -= 1
            if depth == 0:
                end = index + 1
                if end < len(source) and source[end] == ",":
                    end += 1
                if end < len(source) and source[end] == "\n":
                    end += 1
                return start, end
    raise ReleaseNotesError("unterminated version-history entry")


def _replace_latest_entry(
    source: str,
    version: str,
    rendered_entry: str,
    list_marker: str,
    entry_pattern: re.Pattern[str],
) -> str:
    marker_index = source.find(list_marker)
    if marker_index < 0:
        raise ReleaseNotesError(
            f"version-history list marker not found: {list_marker!r}"
        )
    insert_at = marker_index + len(list_marker)

    matches = list(entry_pattern.finditer(source))
    matching_version = [match for match in matches if match.group("version") == version]
    if len(matching_version) > 1:
        raise ReleaseNotesError(f"duplicate version-history entries for {version}")
    if matching_version:
        start, end = _balanced_entry_span(source, matching_version[0].start())
        source = source[:start] + source[end:]

    return source[:insert_at] + rendered_entry + source[insert_at:]


SWIFT_LIST_MARKER = (
    "static let entries: [(version: String, date: String, changes: [String])] = [\n"
)
SWIFT_ENTRY_PATTERN = re.compile(
    r'^        \("(?P<version>\d+\.\d+\.\d+)",', re.MULTILINE
)
KOTLIN_LIST_MARKER = "        listOf(\n"
KOTLIN_ENTRY_PATTERN = re.compile(
    r'^            VersionEntry\(\n                "(?P<version>\d+\.\d+\.\d+)",',
    re.MULTILINE,
)


def sync_version_history(repo_root: Path, version: str, release_date: str) -> None:
    ios_notes = load_notes(repo_root, version, "ios")
    android_notes = load_notes(repo_root, version, "android")

    swift_path = (
        repo_root / "ios/Sources/TaigiKeyboard/App/Tabs/Home/VersionHistory.swift"
    )
    kotlin_path = (
        repo_root
        / "android/app/src/main/java/com/siansiansu/taigikeyboard/content/VersionHistory.kt"
    )

    original_swift_source = swift_path.read_text(encoding="utf-8")
    original_kotlin_source = kotlin_path.read_text(encoding="utf-8")
    swift_source = _replace_latest_entry(
        original_swift_source,
        version,
        render_swift_entry(version, release_date, ios_notes.entries),
        SWIFT_LIST_MARKER,
        SWIFT_ENTRY_PATTERN,
    )
    kotlin_source = _replace_latest_entry(
        original_kotlin_source,
        version,
        render_kotlin_entry(version, release_date, android_notes.entries),
        KOTLIN_LIST_MARKER,
        KOTLIN_ENTRY_PATTERN,
    )
    originals = {swift_path: original_swift_source, kotlin_path: original_kotlin_source}
    written: list[Path] = []
    try:
        for path, rendered in (
            (swift_path, swift_source),
            (kotlin_path, kotlin_source),
        ):
            _write_atomically(path, rendered)
            written.append(path)
    except OSError:
        # Keep the two generated histories aligned even if the second write fails.
        _restore_files(originals, written)
        raise


def check_version_history(repo_root: Path, version: str) -> None:
    checks = (
        (
            "ios",
            repo_root / "ios/Sources/TaigiKeyboard/App/Tabs/Home/VersionHistory.swift",
            SWIFT_LIST_MARKER,
            SWIFT_ENTRY_PATTERN,
        ),
        (
            "android",
            repo_root
            / "android/app/src/main/java/com/siansiansu/taigikeyboard/content/VersionHistory.kt",
            KOTLIN_LIST_MARKER,
            KOTLIN_ENTRY_PATTERN,
        ),
    )
    for platform, path, marker, pattern in checks:
        notes = load_notes(repo_root, version, platform)
        source = path.read_text(encoding="utf-8")
        expected = (
            render_swift_entry(
                version, _entry_date(source, version, pattern), notes.entries
            )
            if platform == "ios"
            else render_kotlin_entry(
                version, _entry_date(source, version, pattern), notes.entries
            )
        )
        insert_at = source.find(marker) + len(marker)
        if insert_at < len(marker) or not source.startswith(expected, insert_at):
            raise ReleaseNotesError(
                f"{path}: newest entry does not exactly match {notes_path(repo_root, version, platform)}",
            )


def _entry_date(source: str, version: str, pattern: re.Pattern[str]) -> str:
    matches = [
        match for match in pattern.finditer(source) if match.group("version") == version
    ]
    if len(matches) != 1:
        raise ReleaseNotesError(
            f"expected exactly one version-history entry for {version}"
        )
    _, end = _balanced_entry_span(source, matches[0].start())
    entry = source[matches[0].start() : end]
    date_match = re.search(r'"(\d{4}/\d{2}/\d{2})"', entry)
    if date_match is None:
        raise ReleaseNotesError(
            f"version-history entry {version} has no YYYY/MM/DD date"
        )
    return date_match.group(1)


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


def parse_windows_version(cargo_source: str) -> str:
    return _sole_match(
        WINDOWS_WORKSPACE_VERSION_PATTERN,
        cargo_source,
        f"[workspace.package] version in {WINDOWS_CARGO_FILE}",
    ).group("value")


def render_windows_cargo(cargo_source: str, version: str) -> str:
    return _replaced_value(
        _sole_match(
            WINDOWS_WORKSPACE_VERSION_PATTERN,
            cargo_source,
            f"[workspace.package] version in {WINDOWS_CARGO_FILE}",
        ),
        cargo_source,
        version,
    )


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


def check_versions_in_sources(
    gradle_source: str,
    project_source: str,
    macos_plist: dict,
    version: str,
    macos_plist_path: Path,
    windows_cargo_source: str,
) -> None:
    """Hold the four already-loaded project files to one version.

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
    check_macos_plist_values(macos_plist, version, macos_plist_path)
    windows_version = parse_windows_version(windows_cargo_source)
    if windows_version != version:
        raise ReleaseNotesError(
            f"Windows workspace version is {windows_version}; expected {version}"
        )


def check_project_versions(repo_root: Path, version: str) -> None:
    macos_plist_path, macos_plist = load_macos_plist(repo_root)
    check_versions_in_sources(
        read_text_file(repo_root, ANDROID_GRADLE_FILE),
        read_text_file(repo_root, IOS_PROJECT_FILE),
        macos_plist,
        version,
        macos_plist_path,
        read_text_file(repo_root, WINDOWS_CARGO_FILE),
    )


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
    """Hold `macos/App/Info.plist` to the same version as the two mobile apps.

    All three platforms ship one version number. macOS is released separately —
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
    allow_downgrade: bool = False,
) -> tuple[str, ...]:
    """Write `version` into all three platform project files, or into none of them.

    Accepts `vMAJOR.MINOR.PATCH` or `MAJOR.MINOR.PATCH`. The rewritten contents
    are held to the same gate the release flow runs, before any real file is
    touched, so a version that gate would reject never reaches the tree.

    Four files cannot be replaced in one filesystem transaction. A write that
    fails part-way is rolled back; a rollback that also fails raises with the
    files it could not put back named in the message.
    """
    version = normalize_version(version)

    sources = {
        relative_path: read_text_file(repo_root, relative_path)
        for relative_path in (
            ANDROID_GRADLE_FILE,
            IOS_PROJECT_FILE,
            MACOS_INFO_PLIST_FILE,
            WINDOWS_CARGO_FILE,
        )
    }
    plist_source = sources[MACOS_INFO_PLIST_FILE]
    _require_xml_plist(plist_source)

    current_android = parse_android_version_name(sources[ANDROID_GRADLE_FILE])
    current_ios = parse_ios_project_versions(sources[IOS_PROJECT_FILE])
    current_macos = _plist_value(plist_source, "CFBundleShortVersionString")
    current_macos_build = _plist_value(plist_source, "CFBundleVersion")
    current_windows = parse_windows_version(sources[WINDOWS_CARGO_FILE])

    if not allow_downgrade:
        for current_version in (
            current_android,
            current_ios.marketing_version,
            current_macos,
            current_windows,
        ):
            _reject_downgrade(current_version, version)

    candidates = {
        ANDROID_GRADLE_FILE: render_android_gradle(
            sources[ANDROID_GRADLE_FILE], version
        ),
        IOS_PROJECT_FILE: render_ios_project(sources[IOS_PROJECT_FILE], version),
        MACOS_INFO_PLIST_FILE: render_macos_plist(plist_source, version),
        WINDOWS_CARGO_FILE: render_windows_cargo(sources[WINDOWS_CARGO_FILE], version),
    }

    # Validate before writing: the rewrite runs through the same gate the release
    # flow runs, so the real tree never holds a version that gate would reject.
    # Parsing the rendered plist here also proves the text edit kept it a plist.
    macos_plist_path = repo_root / MACOS_INFO_PLIST_FILE
    check_versions_in_sources(
        candidates[ANDROID_GRADLE_FILE],
        candidates[IOS_PROJECT_FILE],
        _plist_dictionary(
            candidates[MACOS_INFO_PLIST_FILE].encode("utf-8"), macos_plist_path
        ),
        version,
        macos_plist_path,
        candidates[WINDOWS_CARGO_FILE],
    )

    originals = {repo_root / relative_path: content for relative_path, content in sources.items()}
    written: list[Path] = []
    try:
        for relative_path, content in candidates.items():
            path = repo_root / relative_path
            _write_atomically(path, content)
            written.append(path)
    except OSError as error:
        # Four files cannot be replaced in one filesystem transaction; restoring
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

    return (
        f"Android: versionName {current_android} -> {version}",
        f"iOS: MARKETING_VERSION {current_ios.marketing_version} -> {version}, "
        f"CURRENT_PROJECT_VERSION {current_ios.build_number} -> {IOS_BUILD_NUMBER}",
        f"macOS: CFBundleShortVersionString {current_macos} -> {version}, "
        f"CFBundleVersion {current_macos_build} -> {macos_build_version(version)}",
        f"Windows: workspace version {current_windows} -> {version}",
    )


def _parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    def add_command(name: str, help_text: str) -> argparse.ArgumentParser:
        subparser = subparsers.add_parser(name, help=help_text)
        subparser.add_argument("--version", required=True)
        subparser.add_argument(
            "--repo-root", type=Path, default=Path(__file__).resolve().parents[1]
        )
        return subparser

    sync = add_command("sync", "Render the canonical notes into both app histories")
    sync.add_argument("--date", help="Release date in YYYY/MM/DD form", required=True)
    add_command("check", "Validate the canonical notes and both app histories")
    add_command("check-versions", "Check all three platform project versions")
    set_versions = add_command(
        "set-versions", "Write the version into all three platform project files"
    )
    set_versions.add_argument(
        "--allow-downgrade",
        action="store_true",
        help="Permit a version lower than the one already in the tree",
    )
    printer = add_command("print", "Print one platform's store text")
    printer.add_argument("--platform", choices=("ios", "android"), required=True)

    return parser.parse_args()


def main() -> int:
    args = _parse_args()
    try:
        version = normalize_version(args.version)
        repo_root = args.repo_root.resolve()
        if args.command == "sync":
            if re.fullmatch(r"\d{4}/\d{2}/\d{2}", args.date) is None:
                raise ReleaseNotesError("sync requires --date YYYY/MM/DD")
            sync_version_history(repo_root, version, args.date)
            check_version_history(repo_root, version)
        elif args.command == "check":
            check_version_history(repo_root, version)
        elif args.command == "check-versions":
            check_project_versions(repo_root, version)
        elif args.command == "set-versions":
            for change in set_project_versions(
                repo_root, version, allow_downgrade=args.allow_downgrade
            ):
                print(change)
        else:
            print(load_notes(repo_root, version, args.platform).store_text)
    except ReleaseNotesError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
