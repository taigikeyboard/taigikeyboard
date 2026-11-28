#!/usr/bin/env python3
# Applies one translation-suggestion issue (.github/ISSUE_TEMPLATE/translation.yml) to i18n/*.json.
# Run by .github/workflows/i18n-suggestion.yml on the default branch; the issue body is untrusted.
#
# Default mode reads the issue from $GITHUB_EVENT_PATH and writes `status` to $GITHUB_OUTPUT:
#   skip    — the body is not a translation form (any other issue)
# The text is named by the title the translate page fills in, `[i18n] <namespace>.<key> (<language>)`;
# the form body carries only the suggested text and its reason.
#   invalid — the contributor's input was rejected; the reply for the issue is in --message-file
#   ok      — one value changed in place; `key`, `language`, `path` are written too, validated only
# `--check-changes PATH` runs after generate.py: it stages the change and fails when the worktree
# differs anywhere other than PATH and the generated artifacts.

import argparse
import copy
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
LABEL_REASON = "Why（原因）"
FORM_LABELS = (LABEL_SUGGESTION, LABEL_REASON)
NO_RESPONSE = "_No response_"
TITLE_RE = re.compile(r"\[i18n\]\s*(\S+)\s*\(\s*([A-Za-z]+)\s*\)")

# Longest value today is about 1,200 characters (web.intro, tailo).
MAX_LENGTH = 3000
# Generated Kotlin / Swift / Rust / Inno escapes handle `\n` only; any other control or invisible
# character could split or disguise a generated line, and unassigned code points (Cn, e.g. U+FFFF)
# are not valid XML.
REJECTED_CATEGORIES = {"Cc", "Cf", "Zl", "Zp", "Cs", "Co", "Cn"}

# Only the form's own headings delimit sections, so a `### ` line the contributor typed stays text.
HEADING_RE = re.compile(r"^### (" + "|".join(re.escape(label) for label in FORM_LABELS) + r")[ \t]*$", re.M)

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


def apply(repo_root: Path, title: str, body: str) -> dict:
    """Applies the suggestion in place and returns the workflow outputs."""
    sections = parse_body(body)
    if sections is None:
        return {"status": "skip"}

    named = TITLE_RE.fullmatch(title.strip())
    if named is None:
        raise SuggestionError(UNKNOWN_KEY)
    key_id = named.group(1)
    namespace, _, key = key_id.partition(".")
    path = repo_root / "i18n" / f"{namespace}.json"
    namespaces = {source.stem for source in (repo_root / "i18n").glob("*.json")}
    # Membership first: the path is opened only once the namespace is known to exist.
    data = load_namespace(path) if namespace in namespaces and IDENTIFIER_RE.fullmatch(key) else None
    if data is None or key not in data["keys"]:
        raise SuggestionError(UNKNOWN_KEY)

    language = named.group(2).lower()
    if language not in PRODUCTION_LANGUAGES:
        raise SuggestionError(UNKNOWN_KEY)

    new = sections.get(LABEL_SUGGESTION, "").strip()
    if not new:
        raise SuggestionError("The suggested text is empty.")
    if len(new) > MAX_LENGTH:
        raise SuggestionError(f"The suggested text is longer than {MAX_LENGTH} characters.")
    check_characters(new)
    old = data["keys"][key]["values"][language]
    if new == old:
        raise SuggestionError("The suggested text is the same as the current text.")

    original = path.read_text(encoding="utf-8")
    updated = replace_value(original, key, language, old, new)
    expected = copy.deepcopy(data)
    expected["keys"][key]["values"][language] = new
    if json.loads(updated) != expected:
        raise RuntimeError(f"editing {key_id} changed more than {language}")

    path.write_text(updated, encoding="utf-8")
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
    # Every value is single-line by construction: a fixed status word, an identifier-shaped key,
    # a language from PRODUCTION_LANGUAGES, a path built from both.
    with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as output:
        for name, value in outputs.items():
            output.write(f"{name}={value}\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
