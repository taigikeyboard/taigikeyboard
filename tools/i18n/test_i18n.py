#!/usr/bin/env python3
# Tests for the i18n codegen core. Run: python3 tools/i18n/test_i18n.py

from __future__ import annotations  # keep `dict | None` annotations importable on system Python 3.9

import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import i18n_lib
from i18n_lib import (
    DUPLICATE_KEY_ERROR,
    build_outputs,
    kotlin_escape,
    l10n_accessor,
    load_namespace,
    pseudo,
    res_name,
    string_key_const,
    validate_namespace,
    xml_escape,
)


def _write_namespace(repo_root: Path, namespace: str, keys: dict) -> None:
    src_dir = repo_root / "i18n"
    src_dir.mkdir(parents=True, exist_ok=True)
    (src_dir / f"{namespace}.json").write_text(
        json.dumps({"namespace": namespace, "keys": keys}, ensure_ascii=False),
        encoding="utf-8",
    )


def _android_key(values: dict, placeholders: dict | None = None) -> dict:
    entry = {"scope": {"platforms": ["android"], "surfaces": ["host"]}, "values": values}
    if placeholders is not None:
        entry["placeholders"] = placeholders
    return entry


class NameTransformTest(unittest.TestCase):
    def test_res_name(self):
        self.assertEqual(res_name("settings", "inputMode"), "i18n_settings_inputMode")

    def test_string_key_const(self):
        self.assertEqual(string_key_const("settings", "inputMode"), "SETTINGS_INPUT_MODE")

    def test_l10n_accessor(self):
        self.assertEqual(l10n_accessor("settings", "inputMode"), "settingsInputMode")


class EscapingTest(unittest.TestCase):
    def test_xml_escape(self):
        self.assertEqual(xml_escape("a & b < c"), "a &amp; b &lt; c")
        self.assertEqual(xml_escape("@home"), "\\@home")

    def test_kotlin_escape(self):
        self.assertEqual(kotlin_escape('a"$b'), 'a\\"\\$b')


class PseudoTest(unittest.TestCase):
    def test_preserves_placeholders_and_wraps(self):
        out = pseudo("Imported {count}")
        self.assertTrue(out.startswith("⟦") and out.endswith("⟧"))
        self.assertIn("{count}", out)  # placeholder verbatim, not accented

    def test_inflates_hanji_only_string(self):
        out = pseudo("台語齒盤")
        self.assertGreater(len(out), len("台語齒盤"))  # length-inflated even with no ASCII


class LoadNamespaceTest(unittest.TestCase):
    def test_rejects_duplicate_keys(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "dup.json"
            path.write_text('{"a": 1, "a": 2}', encoding="utf-8")
            with self.assertRaises(ValueError) as ctx:
                load_namespace(path)
            self.assertIn(DUPLICATE_KEY_ERROR, str(ctx.exception))


class ValidateTest(unittest.TestCase):
    def _validate(self, keys):
        validate_namespace("ns", {"namespace": "ns", "keys": keys}, Path("ns.json"))

    def test_missing_base_language(self):
        with self.assertRaises(ValueError):
            self._validate({"k": _android_key({"en": "x"})})

    def test_empty_value_rejected(self):
        with self.assertRaises(ValueError):
            self._validate({"k": _android_key({"hanji": "字", "en": ""})})

    def test_unknown_language_rejected(self):
        with self.assertRaises(ValueError):
            self._validate({"k": _android_key({"hanji": "字", "fr": "x"})})

    def test_placeholder_mismatch_rejected(self):
        with self.assertRaises(ValueError):
            self._validate({"k": _android_key({"hanji": "{a}", "en": "{b}"})})

    def test_declared_placeholder_mismatch_rejected(self):
        with self.assertRaises(ValueError):
            self._validate({"k": _android_key({"hanji": "{a}"}, placeholders={"b": "int"})})

    def test_bad_scope_rejected(self):
        with self.assertRaises(ValueError):
            self._validate({"k": {"scope": {"platforms": ["windows"], "surfaces": ["host"]}, "values": {"hanji": "字"}}})

    def test_valid_passes(self):
        self._validate({"k": _android_key({"hanji": "字", "en": "x"})})


class BuildOutputsTest(unittest.TestCase):
    def test_scope_filter_excludes_ios_only_and_emits_en(self):
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            _write_namespace(
                repo,
                "probe",
                {
                    "shared": _android_key({"hanji": "共用", "en": "Shared"}),
                    "iosOnly": {"scope": {"platforms": ["ios"], "surfaces": ["host"]}, "values": {"hanji": "iOS"}},
                },
            )
            outputs = build_outputs(repo)
            default_xml = outputs[f"{i18n_lib.ANDROID_RES_ROOT}/values/strings_i18n.xml"]
            self.assertIn("i18n_probe_shared", default_xml)
            self.assertNotIn("i18n_probe_iosOnly", default_xml)  # ios-only excluded from android
            # en value present -> values-en emitted; no ja -> no values-ja
            self.assertIn(f"{i18n_lib.ANDROID_RES_ROOT}/values-en/strings_i18n.xml", outputs)
            self.assertNotIn(f"{i18n_lib.ANDROID_RES_ROOT}/values-ja/strings_i18n.xml", outputs)
            self.assertIn("Shared", outputs[f"{i18n_lib.ANDROID_RES_ROOT}/values-en/strings_i18n.xml"])

    def test_deterministic(self):
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            _write_namespace(repo, "probe", {"k": _android_key({"hanji": "字"})})
            self.assertEqual(build_outputs(repo), build_outputs(repo))


if __name__ == "__main__":
    unittest.main()
