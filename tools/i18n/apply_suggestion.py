#!/usr/bin/env python3
# Applies one translation-suggestion issue (.github/ISSUE_TEMPLATE/translation.yml) to i18n/*.json
# or i18n/content/*.json.
# Run by .github/workflows/i18n-suggestion.yml on the default branch; the issue title and body are
# untrusted.
#
# Default mode reads the issue from $GITHUB_EVENT_PATH and writes `status` to $GITHUB_OUTPUT:
#   skip    — the body is not a translation form (any other issue)
#   invalid — the contributor's input was rejected; the reply for the issue is in --message-file
#   ok      — one value changed in place; `key`, `language`, `path` are written too, validated only
# The text is named by the title the translate page fills in, `[i18n] <namespace>.<key> (<language>)`
# or, for FAQ / feature text, `[i18n] content.<file>.<item id>.<field> (<language>)`;
# the form body carries only the suggested text.
# `--check-changes PATH` runs after generate.py: it stages the change and fails when the worktree
# differs anywhere other than PATH and the generated artifacts.

import argparse
import json
import os
import re
import subprocess
import sys
import unicodedata
from pathlib import Path

from i18n_lib import IDENTIFIER_RE, PRODUCTION_LANGUAGES, build_outputs, load_namespace

REPO_ROOT = Path(__file__).resolve().parents[2]

# The form's field labels, as GitHub renders them into `### <label>` body headings. Keep in step
# with translation.yml (test_apply_suggestion.py checks).
LABEL_SUGGESTION = "Suggested text（建議的文字）"
FORM_LABELS = (LABEL_SUGGESTION,)
NO_RESPONSE = "_No response_"
TITLE_PREFIX = "[i18n] "
TITLE_RE = re.compile(r"\[i18n\]\s*(\S+)\s*\(\s*([A-Za-z]+)\s*\)")
# A Taiwanese or Japanese keyboard types full-width parentheses when someone retypes the title.
FULL_WIDTH_PARENTHESES = str.maketrans({"（": "(", "）": ")"})

# Longest value today is about 1,200 characters (web.intro, tailo).
MAX_LENGTH = 3000
# Generated Kotlin / Swift / Rust / Inno escapes handle `\n` only; any other control or invisible
# character could split or disguise a generated line, and unassigned code points (Cn, e.g. U+FFFF)
# are not valid XML.
REJECTED_CATEGORIES = {"Cc", "Cf", "Zl", "Zp", "Cs", "Co", "Cn"}

# Only the form's own headings delimit sections, so a `### ` line the contributor typed stays text.
HEADING_RE = re.compile(r"^### (" + "|".join(re.escape(label) for label in FORM_LABELS) + r")[ \t]*$", re.M)

# Checked before the namespace lookup: a future i18n/content.json namespace would be shadowed.
CONTENT_PREFIX = "content."
# Single-line content fields: list rows, navigation titles and labels in the apps.
CONTENT_SINGLE_LINE_RE = re.compile(r"\.(title|summary|paragraph[0-9]+\.attachment)$")
CONTENT_FIELD_RE = re.compile(r"(title|summary)|paragraph([1-9][0-9]*)(\.attachment)?")

UNKNOWN_KEY = (
    "The issue title does not name a text the app uses. Start again from the Fix button on the "
    "translate page and keep the title it fills in, like `[i18n] nav.tabHome (hanji)`."
)


class SuggestionError(ValueError):
    """The contributor's input was rejected; the message is safe to post back."""


def parse_body(body: str):
    """Returns {label: value} for every form section, or None when the suggestion field is absent."""
    body = body.replace("\r\n", "\n").replace("\r", "\n")
    parts = HEADING_RE.split(body)
    sections = {}
    # parts = [preamble, label1, value1, label2, value2, ...]
    for label, value in zip(parts[1::2], parts[2::2]):
        value = value.strip("\n")
        # The form renders each field once, in order; a repeat typed into a later field is ignored.
        sections.setdefault(label, "" if value.strip() == NO_RESPONSE else value)
    return sections if LABEL_SUGGESTION in sections else None


def check_characters(text: str) -> None:
    for char in text:
        if char != "\n" and unicodedata.category(char) in REJECTED_CATEGORIES:
            raise SuggestionError(
                f"The suggested text contains an invisible or control character (U+{ord(char):04X})."
            )


def replace_value(text: str, key: str, language: str, old: str, new: str) -> str:
    """Rewrites one `"language": "old"` inside the key's block, leaving every other byte as is."""
    start = text.find(f'"{key}": {{')
    needle = f'"{language}": {json.dumps(old, ensure_ascii=False)}'
    at = text.find(needle, start) if start != -1 else -1
    if at == -1:
        raise RuntimeError(f"cannot locate {key}.{language} in the source text")
    return text[:at] + f'"{language}": {json.dumps(new, ensure_ascii=False)}' + text[at + len(needle):]


def locate_namespace_text(repo_root: Path, key_id: str):
    """`<namespace>.<key>` in i18n/<namespace>.json → (path, document, {language: text})."""
    namespace, _, key = key_id.partition(".")
    namespaces = {source.stem for source in (repo_root / "i18n").glob("*.json")}
    # Membership first: the path is opened only once the namespace is known to exist.
    if namespace not in namespaces or not IDENTIFIER_RE.fullmatch(key):
        raise SuggestionError(UNKNOWN_KEY)
    path = repo_root / "i18n" / f"{namespace}.json"
    data = load_namespace(path)
    if key not in data["keys"]:
        raise SuggestionError(UNKNOWN_KEY)
    return path, data, data["keys"][key]["values"]


def locate_content_text(repo_root: Path, key_id: str):
    """`content.<file>.<item id>.<field>` in i18n/content/<file>.json → (path, document, {language: text}).

    Fields: `title`, `summary`, `paragraph<N>` (1-based) and `paragraph<N>.attachment` — the
    localizable objects the apps decode (test_i18n.py ProductionContentTests). The translate page
    builds the same ids (assets/translate.js in the website repository).
    """
    _content, stem, item_id, field = (key_id.split(".", 3) + ["", "", ""])[:4]
    files = {source.stem for source in (repo_root / "i18n" / "content").glob("*.json")}
    match = CONTENT_FIELD_RE.fullmatch(field)
    if stem not in files or match is None:
        raise SuggestionError(UNKNOWN_KEY)
    path = repo_root / "i18n" / "content" / f"{stem}.json"
    data = json.loads(path.read_text(encoding="utf-8"))
    items = next((value for value in data.values() if isinstance(value, list)), [])
    item = next((entry for entry in items if isinstance(entry, dict) and entry.get("id") == item_id), None)
    values = None
    if item is not None and match.group(1):
        values = item.get(match.group(1))
    elif item is not None:
        paragraphs = item.get("paragraphs", [])
        number = int(match.group(2))
        if number <= len(paragraphs):
            paragraph = paragraphs[number - 1]
            values = (paragraph.get("attachment") or {}).get("text") if match.group(3) else paragraph.get("text")
    if not isinstance(values, dict):
        raise SuggestionError(UNKNOWN_KEY)
    return path, data, values


def apply(repo_root: Path, title: str, body: str) -> dict:
    """Applies the suggestion in place and returns the workflow outputs."""
    sections = parse_body(body)
    if sections is None:
        return {"status": "skip"}

    named = TITLE_RE.fullmatch(title.translate(FULL_WIDTH_PARENTHESES).strip())
    if named is None:
        raise SuggestionError(UNKNOWN_KEY)
    key_id = named.group(1)
    is_content = key_id.startswith(CONTENT_PREFIX)
    locate = locate_content_text if is_content else locate_namespace_text
    path, data, values = locate(repo_root, key_id)

    language = named.group(2).lower()
    if language not in PRODUCTION_LANGUAGES or not isinstance(values.get(language), str):
        raise SuggestionError(f"{UNKNOWN_KEY} The language in parentheses is one of: {', '.join(PRODUCTION_LANGUAGES)}.")

    new = sections.get(LABEL_SUGGESTION, "").strip()
    if not new:
        raise SuggestionError("The suggested text is empty.")
    if len(new) > MAX_LENGTH:
        raise SuggestionError(f"The suggested text is longer than {MAX_LENGTH} characters.")
    check_characters(new)
    if is_content and CONTENT_SINGLE_LINE_RE.search(key_id) and "\n" in new:
        raise SuggestionError("This text is shown on one line in the app, so the suggestion cannot contain a line break.")
    old = values[language]
    if new == old:
        raise SuggestionError("The suggested text is the same as the current text.")

    original = path.read_text(encoding="utf-8")
    if is_content and original != json.dumps(data, ensure_ascii=False, indent=2):
        # Re-serializing moves only one value only while the file is canonical (test_i18n.py checks).
        raise RuntimeError(f"{path.name} is not in canonical json.dumps(indent=2) form")
    # `values` lives inside `data`, which was parsed from `original`: after this assignment `data` is
    # the original document with exactly that one value replaced.
    values[language] = new
    if is_content:
        # Canonical (checked above), so re-serializing moves nothing else. No generator reads them.
        updated = json.dumps(data, ensure_ascii=False, indent=2)
    else:
        updated = replace_value(original, key_id.partition(".")[2], language, old, new)
    if json.loads(updated) != data:
        raise RuntimeError(f"editing {key_id} changed more than {language}")

    path.write_text(updated, encoding="utf-8")
    if not is_content:
        try:
            # The generator's own rules (placeholders, punctuation, completeness) judge the new value.
            build_outputs(repo_root, enforce_production_completeness=True)
        except ValueError as exc:
            path.write_text(original, encoding="utf-8")
            raise SuggestionError(f"The suggested text breaks an app text rule:\n\n{exc}") from exc
        except BaseException:
            path.write_text(original, encoding="utf-8")
            raise
    return {"status": "ok", "key": key_id, "language": language, "path": path.relative_to(repo_root).as_posix()}


def unexpected_changes(changed: set, allowed: set, source: str) -> list:
    """Paths the commit must not carry; the source itself must be among the changes."""
    unexpected = sorted(changed - allowed)
    if source not in changed:
        unexpected.append(f"(missing {source})")
    return unexpected


def check_changes(repo_root: Path, source: str) -> int:
    allowed = set(build_outputs(repo_root, enforce_production_completeness=True)) | {source}
    listing = subprocess.check_output(["git", "diff", "--name-only", "-z"], cwd=repo_root).decode()
    changed = set(listing.split("\0")) - {""}
    unexpected = unexpected_changes(changed, allowed, source)
    if unexpected:
        print(f"i18n suggestion: unexpected worktree changes {unexpected}", file=sys.stderr)
        return 1
    subprocess.run(["git", "add", "--", *sorted(changed)], cwd=repo_root, check=True)
    return 0


def rejection_comment(message: str) -> str:
    fence = "~~~~"
    return (
        "This suggestion could not be applied. Edit the issue to fix it and it will be tried again.\n"
        "這个建議無法度套用，請改 issue 內容，改好會閣試一擺。\n\n"
        f"{fence}\n{message.replace(fence, '~ ~ ~ ~')}\n{fence}\n"
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--message-file", type=Path)
    parser.add_argument("--check-changes", metavar="PATH")
    args = parser.parse_args()
    if args.check_changes:
        return check_changes(REPO_ROOT, args.check_changes)

    event = json.loads(Path(os.environ["GITHUB_EVENT_PATH"]).read_text(encoding="utf-8"))
    try:
        issue = event["issue"]
        outputs = apply(REPO_ROOT, issue.get("title") or "", issue.get("body") or "")
    except SuggestionError as exc:
        args.message_file.write_text(rejection_comment(str(exc)), encoding="utf-8")
        outputs = {"status": "invalid"}
    # Every value is single-line by construction: a fixed status word, a key that names an existing text,
    # a language from PRODUCTION_LANGUAGES, a path built from both.
    with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as output:
        for name, value in outputs.items():
            output.write(f"{name}={value}\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
