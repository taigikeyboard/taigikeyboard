"""tools/verify_khpoo_append.py — appended-line detection, format and dedup checks.

Run from `dictionary/`:
    python3 -m pytest tests/test_verify_khpoo_append.py -q
"""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

BASE_DIR = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(BASE_DIR))

from tools.verify_khpoo_append import appended_lines, check_added, keys_of

BASE = "儲,thîr\r\n司,sir\r\n"
BASE_KEYS = keys_of([("儲", "thîr"), ("司", "sir")])


def check(head: str, dictionary_keys: set | None = None) -> list[str]:
    return check_added(appended_lines(BASE, head), BASE_KEYS, dictionary_keys or set())


class AppendedLinesTests(unittest.TestCase):
    def test_appended_line_is_numbered_after_the_base(self):
        self.assertEqual(appended_lines(BASE, BASE + "八,pueh\r\n"), [(3, "八,pueh\r\n")])

    def test_unchanged_file_appends_nothing(self):
        self.assertEqual(appended_lines(BASE, BASE), [])

    def test_reordered_or_edited_lines_are_not_an_append(self):
        for head in ("司,sir\r\n儲,thîr\r\n", "儲,thîr\r\n司,su\r\n八,pueh\r\n"):
            with self.subTest(head=head):
                self.assertIsNone(appended_lines(BASE, head))


class CheckAddedTests(unittest.TestCase):
    def test_new_pair_passes(self):
        self.assertEqual(check(BASE + "八,pueh\r\n"), [])

    def test_malformed_line_fails(self):
        cases = {
            "八,pueh\n": "CRLF",
            "八,pueh": "CRLF",
            '"八,月",pueh-gue̍h\r\n': "no quotes",
            "\ufeff八,pueh\r\n": "byte-order mark",
            "八,pueh,鹿港\r\n": "two non-empty fields",
            "八,\r\n": "two non-empty fields",
            # "i" + U+0302 (NFD) composes to "î" under NFC
            "儲,thi\u0302r\r\n": "NFC",
        }
        for line, message in cases.items():
            with self.subTest(line=line):
                [error] = check(BASE + line)
                self.assertIn(message, error)

    def test_unnormalised_tl_names_the_normal_form(self):
        # trace: normalize_roman("Peh Gue̍h") → NFC, " " → "-", lower → "peh-gue̍h"
        [error] = check(BASE + "八月,Peh Gue̍h\r\n")
        self.assertIn("`八月,peh-gue̍h`", error)

    def test_duplicate_pair_fails(self):
        cases = {
            "司,sir\r\n": ("already in khpoo", set()),
            "八,pueh\r\n": ("already in the dictionary", {("八", "pueh")}),
            # dictionary.csv keeps "bô kâng" (preserve_spaces); the merge keys it "bô-kâng"
            "無仝,bô-kâng\r\n": ("already in the dictionary", keys_of([("無仝", "bô kâng")])),
            "八,pueh\r\n八,pueh\r\n": ("added twice", set()),
        }
        for lines, (message, dictionary_keys) in cases.items():
            with self.subTest(lines=lines):
                [error] = check(BASE + lines, dictionary_keys)
                self.assertIn(message, error)

    def test_same_hanzi_other_reading_passes(self):
        # word identity is the (hanzi, tl) pair: 八/peh in the dictionary does not cover 八/pueh
        self.assertEqual(check(BASE + "八,pueh\r\n", {("八", "peh")}), [])


if __name__ == "__main__":
    unittest.main()
