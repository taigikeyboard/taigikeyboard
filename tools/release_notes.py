"""Validate store release notes and mirror them into platform version history."""

from __future__ import annotations

import argparse
import json
import re
import sys
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
PLATFORM_FORBIDDEN_TERMS = {
    "ios": ("Android", "Google Play", "Play Store"),
    "android": ("App Store", "TestFlight"),
}
VERSION_PATTERN = re.compile(r"^v?(\d+\.\d+\.\d+)$")


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
            if term.casefold() in lowercase_entry:
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
    try:
        swift_path.write_text(swift_source, encoding="utf-8")
        kotlin_path.write_text(kotlin_source, encoding="utf-8")
    except OSError:
        # Keep the two generated histories aligned even if the second write fails.
        swift_path.write_text(original_swift_source, encoding="utf-8")
        kotlin_path.write_text(original_kotlin_source, encoding="utf-8")
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


def check_project_versions(repo_root: Path, version: str) -> None:
    gradle_source = (repo_root / "android/app/build.gradle.kts").read_text(
        encoding="utf-8"
    )
    android_match = re.search(
        r'^\s*versionName\s*=\s*"([^"]+)"', gradle_source, re.MULTILINE
    )
    if android_match is None or android_match.group(1) != version:
        actual = android_match.group(1) if android_match else "missing"
        raise ReleaseNotesError(f"Android versionName is {actual}; expected {version}")

    project_source = (
        repo_root / "ios/TaigiKeyboard.xcodeproj/project.pbxproj"
    ).read_text(encoding="utf-8")
    settings_blocks = re.findall(
        r"buildSettings = \{(?P<body>.*?)\n\s*\};", project_source, re.DOTALL
    )
    expected_bundle_ids = {
        "com.siansiansu.TaigiKeyboard": 2,
        "com.siansiansu.TaigiKeyboard.TaigiKeyboardExtension": 2,
    }
    build_numbers: set[str] = set()
    for bundle_id, expected_count in expected_bundle_ids.items():
        matching_blocks = [
            body
            for body in settings_blocks
            if f"PRODUCT_BUNDLE_IDENTIFIER = {bundle_id};" in body
        ]
        if len(matching_blocks) != expected_count:
            raise ReleaseNotesError(
                f"expected {expected_count} iOS build settings blocks for {bundle_id}; "
                f"found {len(matching_blocks)}",
            )
        for body in matching_blocks:
            marketing_match = re.search(r"MARKETING_VERSION = ([^;]+);", body)
            build_match = re.search(r"CURRENT_PROJECT_VERSION = ([^;]+);", body)
            if marketing_match is None or marketing_match.group(1) != version:
                actual = marketing_match.group(1) if marketing_match else "missing"
                raise ReleaseNotesError(
                    f"iOS MARKETING_VERSION for {bundle_id} is {actual}; expected {version}",
                )
            if build_match is None:
                raise ReleaseNotesError(
                    f"CURRENT_PROJECT_VERSION missing for {bundle_id}"
                )
            build_numbers.add(build_match.group(1))
    if len(build_numbers) != 1:
        raise ReleaseNotesError(
            f"iOS app and extension CURRENT_PROJECT_VERSION values differ: {sorted(build_numbers)}",
        )


def _parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "command",
        choices=("sync", "check", "check-versions", "print"),
    )
    parser.add_argument("--version", required=True)
    parser.add_argument("--platform", choices=("ios", "android"))
    parser.add_argument(
        "--date", help="Release date in YYYY/MM/DD form; required for sync"
    )
    parser.add_argument(
        "--repo-root", type=Path, default=Path(__file__).resolve().parents[1]
    )
    return parser.parse_args()


def main() -> int:
    args = _parse_args()
    try:
        version = normalize_version(args.version)
        repo_root = args.repo_root.resolve()
        if args.command == "sync":
            if (
                args.date is None
                or re.fullmatch(r"\d{4}/\d{2}/\d{2}", args.date) is None
            ):
                raise ReleaseNotesError("sync requires --date YYYY/MM/DD")
            sync_version_history(repo_root, version, args.date)
            check_version_history(repo_root, version)
        elif args.command == "check":
            check_version_history(repo_root, version)
        elif args.command == "check-versions":
            check_project_versions(repo_root, version)
        else:
            if args.platform is None:
                raise ReleaseNotesError("print requires --platform")
            print(load_notes(repo_root, version, args.platform).store_text)
    except ReleaseNotesError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
