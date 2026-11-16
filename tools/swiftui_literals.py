#!/usr/bin/env python3
"""No SwiftUI call in `ios/` or `macos/` takes a string literal as its text.

SwiftUI reads such a literal as a `LocalizedStringKey`; `docs/contributing/i18n.md`
says why that is wrong here and what to write instead.

Flagged: a non-empty literal — an interpolation-only `"\\(x)"` included, it
extracts as `"%@"`, and a raw `#"…"#` — as the first argument of the views and modifiers listed
below, and any `LocalizedStringKey` / `LocalizedStringResource` /
`String(localized:)` / `NSLocalizedString`. The empty literal `""` (a control
with no title) is allowed. Files are matched whole, so a literal on the line
after the opening parenthesis is seen, and so is an example inside a comment.

Usage: python3 tools/swiftui_literals.py   (exit 1 lists every call to fix)
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

VIEWS = (
    "Button",
    "ColorPicker",
    "CommandMenu",
    "ContentUnavailableView",
    "DatePicker",
    "DisclosureGroup",
    "GroupBox",
    "Label",
    "LabeledContent",
    "Link",
    "Menu",
    "MenuBarExtra",
    "NavigationLink",
    "Picker",
    "ProgressView",
    "Section",
    "SecureField",
    "ShareLink",
    "Slider",
    "Stepper",
    "Tab",
    "Text",
    "TextField",
    "Toggle",
    "Window",
    "WindowGroup",
)
MODIFIERS = (
    "accessibilityHint",
    "accessibilityLabel",
    "accessibilityValue",
    "alert",
    "badge",
    "confirmationDialog",
    "help",
    "navigationBarTitle",
    "navigationSubtitle",
    "navigationTitle",
)
LOCALIZING_APIS = (
    r"LocalizedStringKey\b",
    r"LocalizedStringResource\b",
    r"String\(\s*localized:",
    r"NSLocalizedString\b",
)

# A non-empty literal, raw (`#"`) or not: `"` not followed by a closing `"`, unless that opens `"""`.
_NON_EMPTY_LITERAL = r'#*"(?!"(?!"))'
_PATTERN = re.compile(
    rf"(?:\b(?:{'|'.join(VIEWS)})|\.(?:{'|'.join(MODIFIERS)}))\(\s*{_NON_EMPTY_LITERAL}"
    rf"|{'|'.join(LOCALIZING_APIS)}"
)


def findings_in(text: str) -> list[tuple[int, str]]:
    """(1-based line, the matched call) for every localizing call in one Swift source."""
    return [
        (text.count("\n", 0, match.start()) + 1, " ".join(match.group().split()))
        for match in _PATTERN.finditer(text)
    ]


def main() -> int:
    listed = subprocess.run(
        ["git", "ls-files", "-z", "ios/*.swift", "macos/*.swift"],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    failures = [
        f"{rel}:{line}: {call}"
        for rel in filter(None, listed.split("\0"))
        for line, call in findings_in((ROOT / rel).read_text(encoding="utf-8"))
    ]
    if not failures:
        return 0
    print("\n".join(failures), file=sys.stderr)
    print(
        "swiftui-literals: use a generated string (`lang.string(.key)`), or `Text(verbatim:)`"
        " for text that is not a translation — docs/contributing/i18n.md. A comment that"
        " quotes such a call is matched too: reword it.",
        file=sys.stderr,
    )
    return 1


if __name__ == "__main__":
    sys.exit(main())
