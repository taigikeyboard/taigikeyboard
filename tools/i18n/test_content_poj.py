#!/usr/bin/env python3
# Tests for the content/*.json poj-derivation tool. Run: python3 tools/i18n/test_content_poj.py
# The taigi-converter Node bridge is mocked so these stay pure-Python (no Node, no subprocess).

from __future__ import annotations  # keep subscripted annotations importable on system Python 3.9

import contextlib
import io
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent))

import derive_content_poj
from derive_content_poj import DeriveError


def _fake_convert(tailo: str) -> str:
    # Deterministic stand-in for convert_tl_to_poj_strict: the two transliterations the real bridge applies
    # to these fixtures (ts->ch, oo->o͘). Enough to assert derivation + protection without Node.
    return tailo.replace("ts", "ch").replace("oo", "o͘")


def _canonical(doc: dict) -> str:
    # Byte-exact serialization the tool requires of its sources — call the tool's own helper so the test
    # oracle can never drift from the production canonical form.
    return derive_content_poj.canonical_json(doc)


def _feature(fid: str, title: dict, paragraphs: list, summary: dict | None = None) -> dict:
    entry = {"id": fid, "title": title, "icon": {"ios": "x", "android": "y"}}
    if summary is not None:
        entry["summary"] = summary
    entry["paragraphs"] = paragraphs
    return entry


def _features_doc(features: list) -> dict:
    return {"features": features}


def _write(directory: Path, name: str, doc_or_text) -> Path:
    path = directory / name
    text = doc_or_text if isinstance(doc_or_text, str) else _canonical(doc_or_text)
    path.write_text(text, encoding="utf-8")
    return path


class _ContentToolTestCase(unittest.TestCase):
    # Shared setup: mock the Node bridge with the deterministic stand-in + a fresh tempdir per test.
    def setUp(self):
        self._convert = mock.patch.object(derive_content_poj, "convert_tl_to_poj_strict", _fake_convert)
        self._convert.start()
        self.addCleanup(self._convert.stop)
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.dir = Path(self._tmp.name)


class DeriveFileTests(_ContentToolTestCase):
    def _derive(self, doc) -> dict:
        path = _write(self.dir, "tab1-features.json", doc)
        _, _, new_text, _ = derive_content_poj._derive_file(path)
        return json.loads(new_text)

    def test_derives_poj_for_every_localizable_path_type(self):
        doc = _features_doc([
            _feature(
                "f1",
                title={"hanji": "標題", "tailo": "tsng"},
                summary={"hanji": "摘要", "tailo": "tsmm"},
                paragraphs=[
                    {"text": {"hanji": "段落", "tailo": "tsoo"}},
                    {"text": {"hanji": "圖", "tailo": "too"},
                     "attachment": {"type": "navigation", "text": {"hanji": "去", "tailo": "tsk"},
                                    "destination": "d", "icon": {"ios": "i", "android": "a"}}},
                ],
            )
        ])
        out = self._derive(doc)
        feat = out["features"][0]
        self.assertEqual(feat["title"]["poj"], _fake_convert("tsng"))
        self.assertEqual(feat["summary"]["poj"], _fake_convert("tsmm"))
        self.assertEqual(feat["paragraphs"][0]["text"]["poj"], _fake_convert("tsoo"))
        self.assertEqual(feat["paragraphs"][1]["text"]["poj"], _fake_convert("too"))
        self.assertEqual(feat["paragraphs"][1]["attachment"]["text"]["poj"], _fake_convert("tsk"))

    def test_poj_inserted_after_tailo_in_canonical_order(self):
        doc = _features_doc([_feature(
            "f1", title={"hanji": "h", "tailo": "ts", "en": "E", "ja": "J"}, paragraphs=[])])
        out = self._derive(doc)
        self.assertEqual(list(out["features"][0]["title"].keys()), ["hanji", "tailo", "poj", "en", "ja"])

    def test_object_without_tailo_left_unchanged(self):
        doc = _features_doc([_feature("f1", title={"hanji": "h"}, paragraphs=[])])
        out = self._derive(doc)
        self.assertNotIn("poj", out["features"][0]["title"])

    def test_stale_poj_replaced_by_fresh_derivation(self):
        doc = _features_doc([_feature(
            "f1", title={"hanji": "h", "tailo": "tsoo", "poj": "STALE"}, paragraphs=[])])
        out = self._derive(doc)
        self.assertEqual(out["features"][0]["title"]["poj"], _fake_convert("tsoo"))

    def test_protected_literal_kept_verbatim(self):
        # The quoted spelling example `'oo'` stays while surrounding `ts` still converts.
        with mock.patch.dict(derive_content_poj.POJ_PROTECT, {"features[f1].title": ["'oo'"]}, clear=True):
            doc = _features_doc([_feature(
                "f1", title={"hanji": "h", "tailo": "tsa 'oo' tsb"}, paragraphs=[])])
            out = self._derive(doc)
        poj = out["features"][0]["title"]["poj"]
        self.assertIn("'oo'", poj)            # protected literal NOT transliterated to 'o͘'
        self.assertIn("cha", poj)             # surrounding ts -> ch still applied
        self.assertIn("chb", poj)

    def test_protected_literal_does_not_overmatch_normal_word(self):
        # Regression for the bare-`oo` bug: protecting the quoted spelling `'oo'` must NOT freeze the `oo`
        # inside a real word like `tsoo` (組), which must still transliterate to `cho͘`.
        with mock.patch.dict(derive_content_poj.POJ_PROTECT, {"features[f1].title": ["'oo'"]}, clear=True):
            doc = _features_doc([_feature(
                "f1", title={"hanji": "h", "tailo": "tsoo 'oo' end"}, paragraphs=[])])
            out = self._derive(doc)
        poj = out["features"][0]["title"]["poj"]
        self.assertIn("cho͘", poj)             # real word `tsoo` -> `cho͘` (NOT frozen)
        self.assertIn("'oo'", poj)            # quoted spelling example stays verbatim
        self.assertNotIn("tsoo", poj)

    def test_stale_protect_literal_not_in_tailo_rejected(self):
        with mock.patch.dict(derive_content_poj.POJ_PROTECT, {"features[f1].title": ["'oo'"]}, clear=True):
            doc = _features_doc([_feature("f1", title={"hanji": "h", "tailo": "no spelling here"},
                                          paragraphs=[])])
            path = _write(self.dir, "tab1-features.json", doc)
            with self.assertRaises(DeriveError):
                derive_content_poj._derive_file(path)

    def test_duplicate_id_rejected(self):
        doc = _features_doc([
            _feature("dup", title={"hanji": "a"}, paragraphs=[]),
            _feature("dup", title={"hanji": "b"}, paragraphs=[]),
        ])
        path = _write(self.dir, "tab1-features.json", doc)
        with self.assertRaises(DeriveError):
            derive_content_poj._derive_file(path)

    def test_malformed_tree_missing_paragraphs_rejected(self):
        doc = {"features": [{"id": "f1", "title": {"hanji": "h"}, "icon": {"ios": "x", "android": "y"}}]}
        path = _write(self.dir, "tab1-features.json", doc)
        with self.assertRaises(DeriveError):
            derive_content_poj._derive_file(path)

    def test_non_dict_root_rejected(self):
        # JSON root that is an array (or any non-object) must raise DeriveError, not a raw AttributeError.
        path = _write(self.dir, "tab1-features.json", "[]")
        with self.assertRaises(DeriveError):
            derive_content_poj._derive_file(path)

    def test_protect_path_without_tailo_rejected(self):
        # A protected path whose object has no tailo at all is a stale POJ_PROTECT entry — must fail, never
        # silently skip (which would let `--check` pass on a path that should have protected poj).
        with mock.patch.dict(derive_content_poj.POJ_PROTECT, {"features[f1].title": ["'oo'"]}, clear=True):
            doc = _features_doc([_feature("f1", title={"hanji": "h"}, paragraphs=[])])
            path = _write(self.dir, "tab1-features.json", doc)
            with self.assertRaises(DeriveError):
                derive_content_poj._derive_file(path)

    def test_duplicate_key_rejected(self):
        text = '{\n  "features": [],\n  "features": []\n}'
        path = _write(self.dir, "tab1-features.json", text)
        with self.assertRaises(ValueError):
            derive_content_poj._derive_file(path)

    def test_non_string_value_rejected(self):
        doc = _features_doc([_feature("f1", title={"hanji": "h", "tailo": 5}, paragraphs=[])])
        path = _write(self.dir, "tab1-features.json", doc)
        with self.assertRaises(DeriveError):
            derive_content_poj._derive_file(path)

    def test_missing_hanji_rejected(self):
        doc = _features_doc([_feature("f1", title={"tailo": "ts"}, paragraphs=[])])
        path = _write(self.dir, "tab1-features.json", doc)
        with self.assertRaises(DeriveError):
            derive_content_poj._derive_file(path)

    def test_unexpected_key_rejected(self):
        doc = _features_doc([_feature("f1", title={"hanji": "h", "note": "x"}, paragraphs=[])])
        path = _write(self.dir, "tab1-features.json", doc)
        with self.assertRaises(DeriveError):
            derive_content_poj._derive_file(path)

    def test_stray_hanji_object_off_known_path_rejected(self):
        doc = _features_doc([_feature("f1", title={"hanji": "h"}, paragraphs=[])])
        doc["features"][0]["icon"]["hanji"] = "stowaway"  # hanji where no LocalizedContentText belongs
        path = _write(self.dir, "tab1-features.json", doc)
        with self.assertRaises(DeriveError):
            derive_content_poj._derive_file(path)

    def test_noncanonical_form_rejected(self):
        # Valid JSON but pretty-printed differently (extra spacing) — refuse rather than reflow.
        text = json.dumps(_features_doc([_feature("f1", title={"hanji": "h"}, paragraphs=[])]),
                          ensure_ascii=False, indent=4)
        path = _write(self.dir, "tab1-features.json", text)
        with self.assertRaises(DeriveError):
            derive_content_poj._derive_file(path)

    def test_nan_rejected(self):
        text = '{\n  "features": [],\n  "x": NaN\n}'
        path = _write(self.dir, "tab1-features.json", text)
        with self.assertRaises(ValueError):
            derive_content_poj._derive_file(path)

    def test_combining_marks_bopomofo_and_quotes_round_trip(self):
        # Combining tone marks (o̍ / o͘), 方音符號 (U+31A0 block) and quote/backslash pass through the fake
        # converter and must re-serialize byte-clean.
        tricky = "tsia̍h o͘ ㄗㄘㄙ \"q\" \\b"
        doc = _features_doc([_feature("f1", title={"hanji": tricky, "tailo": tricky}, paragraphs=[])])
        out = self._derive(doc)
        self.assertEqual(out["features"][0]["title"]["poj"], _fake_convert(tricky))


class MainTests(_ContentToolTestCase):
    def setUp(self):
        super().setUp()
        self.root = self.dir
        (self.root / "content").mkdir()
        # One features file + one faqs file, both canonical, both with a derivable tailo.
        self.features = _write(self.root / "content", "tab1-features.json",
                               _features_doc([_feature("f1", title={"hanji": "h", "tailo": "tsoo"},
                                                       paragraphs=[])]))
        self.faqs = _write(self.root / "content", "tab1-faq.json",
                           {"faqs": [_feature("q1", title={"hanji": "q", "tailo": "tsa"}, paragraphs=[])]})
        self._patches = [
            mock.patch.object(derive_content_poj, "REPO_ROOT", self.root),
            mock.patch.object(derive_content_poj, "POJ_PROTECT", {}),
        ]
        for patch in self._patches:
            patch.start()
            self.addCleanup(patch.stop)

    def _run(self, *argv) -> tuple[int, str, str]:
        out, err = io.StringIO(), io.StringIO()
        with mock.patch.object(sys, "argv", ["derive_content_poj.py", *argv]):
            with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
                code = derive_content_poj.main()
        return code, out.getvalue(), err.getvalue()

    def test_run_writes_poj_then_check_is_fresh(self):
        code, _, _ = self._run()
        self.assertEqual(code, 0)
        self.assertEqual(json.loads(self.features.read_text())["features"][0]["title"]["poj"],
                         _fake_convert("tsoo"))
        check_code, out, _ = self._run("--check")
        self.assertEqual(check_code, 0)
        self.assertIn("fresh", out)

    def test_check_detects_drift_without_writing(self):
        before = self.features.read_text(encoding="utf-8")
        code, _, err = self._run("--check")  # poj missing -> stale
        self.assertEqual(code, 1)
        self.assertIn("stale", err)
        self.assertEqual(self.features.read_text(encoding="utf-8"), before)  # no write on --check

    def test_converter_failure_writes_nothing(self):
        before = self.features.read_text(encoding="utf-8")
        with mock.patch.object(derive_content_poj, "convert_tl_to_poj_strict",
                               mock.Mock(side_effect=RuntimeError("bridge down"))):
            code, _, err = self._run()
        self.assertEqual(code, 1)
        self.assertIn("bridge down", err)
        self.assertEqual(self.features.read_text(encoding="utf-8"), before)

    def test_unknown_protect_path_fails(self):
        with mock.patch.object(derive_content_poj, "POJ_PROTECT", {"features[nope].title": ["x"]}):
            code, _, err = self._run()
        self.assertEqual(code, 1)
        self.assertIn("unknown path", err)


class ProductionContentTests(unittest.TestCase):
    # Smoke test against the REAL committed content/*.json (no Node, no mock). A normal iOS/Android build
    # does NOT prove the bundled JSON decodes — the iOS loader silently returns [] on a decode failure — so
    # this gates that production content is well-formed, canonical, and complete in all 5 languages.
    EXPECTED_LANGS = {"hanji", "tailo", "poj", "en", "ja"}

    def _localizable(self, doc):
        items = doc.get("features")
        if items is None:
            items = doc.get("faqs")
        out = []
        for entry in items:
            out.append(entry["title"])
            if "summary" in entry:
                out.append(entry["summary"])
            for paragraph in entry["paragraphs"]:
                out.append(paragraph["text"])
                attachment = paragraph.get("attachment")
                if isinstance(attachment, dict) and "text" in attachment:
                    out.append(attachment["text"])
        return out

    def test_production_content_well_formed_canonical_and_complete(self):
        total = 0
        for rel in derive_content_poj.CONTENT_FILES:
            path = derive_content_poj.REPO_ROOT / rel
            text = path.read_text(encoding="utf-8")
            doc = json.loads(text)
            self.assertEqual(text, _canonical(doc), f"{rel} is not in canonical json.dumps(indent=2) form")
            for obj in self._localizable(doc):
                total += 1
                self.assertEqual(set(obj), self.EXPECTED_LANGS,
                                 f"{rel}: localizable object missing/extra languages: {sorted(obj)}")
                for lang, value in obj.items():
                    self.assertIsInstance(value, str, f"{rel}: {lang} is not a string")
                    self.assertTrue(value.strip(), f"{rel}: {lang} is empty")
        self.assertEqual(total, 80, "expected 80 localizable content strings")


class AtomicWriteTests(unittest.TestCase):
    def test_refuses_to_write_through_symlink(self):
        with tempfile.TemporaryDirectory() as tmp:
            real = Path(tmp) / "real.json"
            real.write_text("{}", encoding="utf-8")
            link = Path(tmp) / "link.json"
            link.symlink_to(real)
            with self.assertRaises(DeriveError):
                derive_content_poj._atomic_write(link, "{}")

    def test_preserves_source_file_mode(self):
        import os
        import stat
        with tempfile.TemporaryDirectory() as tmp:
            target = Path(tmp) / "f.json"
            target.write_text("old", encoding="utf-8")
            os.chmod(target, 0o644)
            derive_content_poj._atomic_write(target, "new")
            self.assertEqual(stat.S_IMODE(os.stat(target).st_mode), 0o644)
            self.assertEqual(target.read_text(encoding="utf-8"), "new")


if __name__ == "__main__":
    unittest.main()
