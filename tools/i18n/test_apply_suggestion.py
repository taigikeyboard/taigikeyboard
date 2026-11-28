#!/usr/bin/env python3
# Tests for apply_suggestion.py. Run: python3 tools/i18n/test_apply_suggestion.py

import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from apply_suggestion import (
    FORM_LABELS,
    LABEL_CURRENT,
    LABEL_KEY,
    LABEL_LANGUAGE,
    LABEL_REASON,
    LABEL_SUGGESTION,
    SuggestionError,
    apply,
    parse_body,
    rejection_comment,
    unexpected_changes,
)

REPO_ROOT = Path(__file__).resolve().parents[2]
TEMPLATE = REPO_ROOT / ".github" / "ISSUE_TEMPLATE" / "translation.yml"
WORKFLOW = REPO_ROOT / ".github" / "workflows" / "i18n-suggestion.yml"

# Any real key works; the tests read its current values from the copied sources, so a later wording
# change to it (including one this automation opens) keeps them green.
KEY = "nav.tabHome"


def form_body(key=KEY, language="en", suggestion="Start page", reason="_No response_", newline="\n"):
    # The shape GitHub renders an issue form into.
    sections = [
        (LABEL_KEY, key),
        (LABEL_LANGUAGE, language),
        (LABEL_CURRENT, "_No response_"),
        (LABEL_SUGGESTION, suggestion),
        (LABEL_REASON, reason),
    ]
    return newline.join(f"### {label}{newline}{newline}{value}{newline}" for label, value in sections)


class FormLabelsTest(unittest.TestCase):
    def test_labels_match_the_issue_template(self):
        labels = re.findall(r"^\s+label: (.+)$", TEMPLATE.read_text(encoding="utf-8"), re.M)
        self.assertEqual(tuple(labels), FORM_LABELS)

    def test_workflow_starts_only_for_the_form(self):
        # The job's `if:` matches the key heading; a rename that misses it would silence the workflow.
        self.assertIn(f"contains(github.event.issue.body, '### {LABEL_KEY}')", WORKFLOW.read_text(encoding="utf-8"))


class ParseBodyTest(unittest.TestCase):
    def test_sections_and_no_response(self):
        sections = parse_body(form_body())
        self.assertEqual(sections[LABEL_KEY], KEY)
        self.assertEqual(sections[LABEL_REASON], "")

    def test_crlf_and_lone_cr_become_line_feeds(self):
        sections = parse_body(form_body(suggestion="one\rtwo", newline="\r\n"))
        self.assertEqual(sections[LABEL_SUGGESTION], "one\ntwo")

    def test_blank_issue_is_not_a_form(self):
        self.assertIsNone(parse_body("The keyboard crashes when I type."))

    def test_other_headings_stay_in_the_text(self):
        sections = parse_body(form_body(suggestion="### Start\npage"))
        self.assertEqual(sections[LABEL_SUGGESTION], "### Start\npage")

    def test_a_repeated_form_heading_cannot_override_a_field(self):
        sections = parse_body(form_body(reason=f"### {LABEL_LANGUAGE}\n\nja"))
        self.assertEqual(sections[LABEL_LANGUAGE], "en")


class ApplyTest(unittest.TestCase):
    # One copy of i18n/ for the class; each test restores nav.json, the only file apply() writes here.
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        cls.root = Path(cls.tmp.name)
        shutil.copytree(REPO_ROOT / "i18n", cls.root / "i18n")
        cls.nav = cls.root / "i18n" / "nav.json"
        cls.original = cls.nav.read_text(encoding="utf-8")

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def tearDown(self):
        self.nav.write_text(self.original, encoding="utf-8")

    def english(self):
        return json.loads(self.nav.read_text(encoding="utf-8"))["keys"]["tabHome"]["values"]["en"]

    def assert_rejected(self, body, fragment):
        with self.assertRaises(SuggestionError) as caught:
            apply(self.root, body)
        self.assertIn(fragment, str(caught.exception))
        self.assertEqual(self.nav.read_text(encoding="utf-8"), self.original)

    def test_changes_only_that_value(self):
        old = self.english()
        new = old + " page"
        outputs = apply(self.root, form_body(suggestion=new))
        self.assertEqual(outputs, {"status": "ok", "key": KEY, "language": "en", "path": "i18n/nav.json"})
        # Same bytes except the one value: the hand formatting survives.
        expected = self.original.replace(
            f'"en": {json.dumps(old, ensure_ascii=False)}', f'"en": {json.dumps(new, ensure_ascii=False)}', 1
        )
        self.assertEqual(self.nav.read_text(encoding="utf-8"), expected)

    def test_language_is_case_and_space_insensitive(self):
        self.assertEqual(apply(self.root, form_body(language="  EN "))["language"], "en")

    def test_escapes_quotes_and_backslashes(self):
        apply(self.root, form_body(suggestion='Start "main" \\ page'))
        self.assertEqual(self.english(), 'Start "main" \\ page')

    def test_non_form_is_skipped(self):
        self.assertEqual(apply(self.root, "Some other report"), {"status": "skip"})

    def test_unknown_namespace(self):
        self.assert_rejected(form_body(key="../secrets.tabHome"), "text key")

    def test_unknown_key(self):
        self.assert_rejected(form_body(key="nav.noSuchKey"), "text key")

    def test_content_files_are_not_keys(self):
        self.assert_rejected(form_body(key="content.faqs"), "text key")

    def test_unknown_language(self):
        self.assert_rejected(form_body(language="zh"), "language")

    def test_empty_suggestion(self):
        self.assert_rejected(form_body(suggestion="_No response_"), "empty")

    def test_same_as_current(self):
        self.assert_rejected(form_body(suggestion=self.english()), "same")

    def test_too_long(self):
        self.assert_rejected(form_body(suggestion="a" * 3001), "longer")

    def test_rejects_invisible_control_and_unassigned_characters(self):
        # NUL, tab, NEL, line separator, right-to-left override, zero-width space, noncharacter.
        for char in ("\x00", "\t", "\u0085", " ", "‮", "​", "￿"):
            with self.subTest(codepoint=f"U+{ord(char):04X}"):
                self.assert_rejected(form_body(suggestion=f"Start{char}page"), "character")

    def test_generator_rule_is_reported_and_reverted(self):
        # A placeholder that no other language carries fails the generator's placeholder check.
        self.assert_rejected(form_body(suggestion="Start {count}"), "app text rule")


class ChangeGuardTest(unittest.TestCase):
    def test_source_and_artifacts_pass(self):
        changed = {"i18n/nav.json", "a.kt"}
        self.assertEqual(unexpected_changes(changed, {"i18n/nav.json", "a.kt", "b.swift"}, "i18n/nav.json"), [])

    def test_other_files_are_reported(self):
        changed = {"i18n/nav.json", "Makefile"}
        self.assertEqual(unexpected_changes(changed, {"i18n/nav.json"}, "i18n/nav.json"), ["Makefile"])

    def test_missing_source_is_reported(self):
        changed = {"a.kt"}
        self.assertEqual(unexpected_changes(changed, {"i18n/nav.json", "a.kt"}, "i18n/nav.json"), ["(missing i18n/nav.json)"])


class MainTest(unittest.TestCase):
    # The workflow reads `status`, `key`, `language`, `path` from $GITHUB_OUTPUT and posts
    # --message-file. One scratch repo for the class: the cases below never change a file.
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        cls.root = Path(cls.tmp.name)
        scripts = cls.root / "repo" / "tools" / "i18n"
        scripts.mkdir(parents=True)
        for name in ("apply_suggestion.py", "i18n_lib.py"):
            shutil.copy(Path(__file__).resolve().parent / name, scripts / name)
        shutil.copytree(REPO_ROOT / "i18n", cls.root / "repo" / "i18n")
        cls.script = scripts / "apply_suggestion.py"

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def run_main(self, body):
        event, output, message = (self.root / name for name in ("event.json", "output.txt", "message.md"))
        event.write_text(json.dumps({"issue": {"body": body}}), encoding="utf-8")
        output.write_text("", encoding="utf-8")
        message.unlink(missing_ok=True)
        environment = dict(os.environ, GITHUB_EVENT_PATH=str(event), GITHUB_OUTPUT=str(output))
        subprocess.run([sys.executable, str(self.script), "--message-file", str(message)], check=True, env=environment)
        outputs = dict(line.split("=", 1) for line in output.read_text(encoding="utf-8").splitlines())
        return outputs, (message.read_text(encoding="utf-8") if message.exists() else None)

    def test_invalid_writes_only_the_status_and_a_reply(self):
        outputs, message = self.run_main(form_body(language="zh"))
        self.assertEqual(outputs, {"status": "invalid"})
        self.assertIn("language", message)

    def test_non_form_is_skipped(self):
        outputs, message = self.run_main("Not a form")
        self.assertEqual(outputs, {"status": "skip"})
        self.assertIsNone(message)


class CommentTest(unittest.TestCase):
    def test_message_cannot_close_the_fence(self):
        comment = rejection_comment("x\n~~~~\n@someone")
        self.assertEqual(comment.count("~~~~"), 2)


if __name__ == "__main__":
    unittest.main()
