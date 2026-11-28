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
    TITLE_PREFIX,
    TITLE_RE,
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


def title(key=KEY, language="en"):
    # The title the translate page fills in (assets/translate.js in the website repository).
    return f"{TITLE_PREFIX}{key} ({language})"


def form_body(suggestion="Start page", newline="\n"):
    # The shape GitHub renders an issue form into.
    return f"### {LABEL_SUGGESTION}{newline}{newline}{suggestion}{newline}"


class FormLabelsTest(unittest.TestCase):
    def test_labels_match_the_issue_template(self):
        labels = re.findall(r"^\s+label: (.+)$", TEMPLATE.read_text(encoding="utf-8"), re.M)
        self.assertEqual(tuple(labels), FORM_LABELS)

    def test_template_title_is_the_prefix_the_script_parses(self):
        template_title = re.search(r'^title: "(.*)"$', TEMPLATE.read_text(encoding="utf-8"), re.M).group(1)
        self.assertEqual(template_title, TITLE_PREFIX)
        self.assertIsNotNone(TITLE_RE.fullmatch(title()))

    def test_workflow_starts_only_for_the_form(self):
        # The job's `if:` matches the suggestion heading; a rename that misses it would silence the workflow.
        self.assertIn(f"contains(github.event.issue.body, '### {LABEL_SUGGESTION}')", WORKFLOW.read_text(encoding="utf-8"))


class ParseBodyTest(unittest.TestCase):
    def test_sections_and_no_response(self):
        sections = parse_body(form_body())
        self.assertEqual(sections[LABEL_SUGGESTION], "Start page")

    def test_no_response_is_empty(self):
        self.assertEqual(parse_body(form_body(suggestion="_No response_"))[LABEL_SUGGESTION], "")

    def test_crlf_and_lone_cr_become_line_feeds(self):
        sections = parse_body(form_body(suggestion="one\rtwo", newline="\r\n"))
        self.assertEqual(sections[LABEL_SUGGESTION], "one\ntwo")

    def test_blank_issue_is_not_a_form(self):
        self.assertIsNone(parse_body("The keyboard crashes when I type."))

    def test_other_headings_stay_in_the_text(self):
        sections = parse_body(form_body(suggestion="### Start\npage"))
        self.assertEqual(sections[LABEL_SUGGESTION], "### Start\npage")

    def test_a_repeated_form_heading_cannot_override_a_field(self):
        sections = parse_body(form_body(suggestion=f"Start page\n### {LABEL_SUGGESTION}\n\nOther"))
        self.assertEqual(sections[LABEL_SUGGESTION], "Start page")


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

    def assert_rejected(self, body, fragment, issue_title=None):
        with self.assertRaises(SuggestionError) as caught:
            apply(self.root, issue_title or title(), body)
        self.assertIn(fragment, str(caught.exception))
        self.assertEqual(self.nav.read_text(encoding="utf-8"), self.original)

    def test_changes_only_that_value(self):
        old = self.english()
        new = old + " page"
        outputs = apply(self.root, title(), form_body(suggestion=new))
        self.assertEqual(outputs, {"status": "ok", "key": KEY, "language": "en", "path": "i18n/nav.json"})
        # Same bytes except the one value: the hand formatting survives.
        expected = self.original.replace(
            f'"en": {json.dumps(old, ensure_ascii=False)}', f'"en": {json.dumps(new, ensure_ascii=False)}', 1
        )
        self.assertEqual(self.nav.read_text(encoding="utf-8"), expected)

    def test_title_language_is_case_and_space_insensitive(self):
        self.assertEqual(apply(self.root, f"  [i18n] {KEY} ( EN ) ", form_body())["language"], "en")

    def test_escapes_quotes_and_backslashes(self):
        apply(self.root, title(), form_body(suggestion='Start "main" \\ page'))
        self.assertEqual(self.english(), 'Start "main" \\ page')

    def test_non_form_is_skipped(self):
        self.assertEqual(apply(self.root, title(), "Some other report"), {"status": "skip"})

    def test_unknown_namespace(self):
        self.assert_rejected(form_body(), "title", title(key="../secrets.tabHome"))

    def test_unknown_key(self):
        self.assert_rejected(form_body(), "title", title(key="nav.noSuchKey"))

    def test_content_files_are_not_keys(self):
        self.assert_rejected(form_body(), "title", title(key="content.faqs"))

    def test_unknown_language_lists_the_languages(self):
        self.assert_rejected(form_body(), "hanji, en, ja, tailo, poj", title(language="zh"))

    def test_full_width_parentheses_are_accepted(self):
        self.assertEqual(apply(self.root, f"[i18n] {KEY}（en）", form_body())["key"], KEY)

    def test_edited_title(self):
        self.assert_rejected(form_body(), "title", "Please fix the home tab")

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


class ContentApplyTest(unittest.TestCase):
    # i18n/content/*.json: nested FAQ / feature text, addressed as content.<file>.<item id>.<field>.
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        cls.root = Path(cls.tmp.name)
        shutil.copytree(REPO_ROOT / "i18n", cls.root / "i18n")
        cls.faq = cls.root / "i18n" / "content" / "faq.json"
        cls.original = cls.faq.read_text(encoding="utf-8")
        cls.item = json.loads(cls.original)["faqs"][0]

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def tearDown(self):
        self.faq.write_text(self.original, encoding="utf-8")

    def key(self, field):
        return f"content.faq.{self.item['id']}.{field}"

    def test_title_changes_only_that_value(self):
        new = self.item["title"]["en"] + " now"
        outputs = apply(self.root, title(self.key("title")), form_body(suggestion=new))
        self.assertEqual(outputs["path"], "i18n/content/faq.json")
        wanted = json.loads(self.original)
        wanted["faqs"][0]["title"]["en"] = new
        # Canonical form kept, so the diff is that one line.
        self.assertEqual(self.faq.read_text(encoding="utf-8"), json.dumps(wanted, ensure_ascii=False, indent=2))

    def test_paragraph_is_one_based(self):
        apply(self.root, title(self.key("paragraph1")), form_body(suggestion="First paragraph."))
        self.assertEqual(json.loads(self.faq.read_text(encoding="utf-8"))["faqs"][0]["paragraphs"][0]["text"]["en"], "First paragraph.")

    def test_attachment_text(self):
        number = next(index for index, paragraph in enumerate(self.item["paragraphs"], 1) if "text" in (paragraph.get("attachment") or {}))
        apply(self.root, title(self.key(f"paragraph{number}.attachment")), form_body(suggestion="See the steps."))
        paragraph = json.loads(self.faq.read_text(encoding="utf-8"))["faqs"][0]["paragraphs"][number - 1]
        self.assertEqual(paragraph["attachment"]["text"]["en"], "See the steps.")

    def test_unknown_item_field_or_paragraph(self):
        for key_id in ("content.faq.noSuchItem.title", self.key("icon"), self.key("paragraph0"),
                       self.key(f"paragraph{len(self.item['paragraphs']) + 1}"), "content.nofile.x.title"):
            with self.subTest(key_id=key_id), self.assertRaises(SuggestionError):
                apply(self.root, title(key_id), form_body())
        self.assertEqual(self.faq.read_text(encoding="utf-8"), self.original)


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

    def run_main(self, issue_title, body):
        event, output, message = (self.root / name for name in ("event.json", "output.txt", "message.md"))
        event.write_text(json.dumps({"issue": {"title": issue_title, "body": body}}), encoding="utf-8")
        output.write_text("", encoding="utf-8")
        message.unlink(missing_ok=True)
        environment = dict(os.environ, GITHUB_EVENT_PATH=str(event), GITHUB_OUTPUT=str(output))
        subprocess.run([sys.executable, str(self.script), "--message-file", str(message)], check=True, env=environment)
        outputs = dict(line.split("=", 1) for line in output.read_text(encoding="utf-8").splitlines())
        return outputs, (message.read_text(encoding="utf-8") if message.exists() else None)

    def test_invalid_writes_only_the_status_and_a_reply(self):
        outputs, message = self.run_main(title(language="zh"), form_body())
        self.assertEqual(outputs, {"status": "invalid"})
        self.assertIn("title", message)

    def test_non_form_is_skipped(self):
        outputs, message = self.run_main("Crash on start", "Not a form")
        self.assertEqual(outputs, {"status": "skip"})
        self.assertIsNone(message)


class CommentTest(unittest.TestCase):
    def test_message_cannot_close_the_fence(self):
        comment = rejection_comment("x\n~~~~\n@someone")
        self.assertEqual(comment.count("~~~~"), 2)


if __name__ == "__main__":
    unittest.main()
