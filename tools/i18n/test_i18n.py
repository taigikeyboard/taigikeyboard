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


class PlaceholderValidationTest(unittest.TestCase):
    def _validate(self, keys):
        validate_namespace("ns", {"namespace": "ns", "keys": keys}, Path("ns.json"))

    def test_format_key_requires_declaration(self):
        with self.assertRaises(ValueError):
            self._validate({"k": _android_key({"hanji": "{count} 項"})})

    def test_unknown_placeholder_type_rejected(self):
        with self.assertRaises(ValueError):
            self._validate({"k": _android_key({"hanji": "{count}"}, placeholders={"count": "banana"})})

    def test_keyword_placeholder_name_rejected(self):
        with self.assertRaises(ValueError):
            self._validate({"k": _android_key({"hanji": "{fun}"}, placeholders={"fun": "int"})})

    def test_swift_keyword_placeholder_name_rejected(self):
        # `default` is a legal Kotlin identifier but a Swift keyword; the placeholder becomes a Swift
        # function parameter, so it must be rejected for the iOS emitter (Codex post-impl).
        with self.assertRaises(ValueError):
            self._validate({"k": _android_key({"hanji": "{default}"}, placeholders={"default": "int"})})

    def test_declared_but_unused_rejected(self):
        with self.assertRaises(ValueError):
            self._validate({"k": _android_key({"hanji": "no placeholder"}, placeholders={"count": "int"})})

    def test_translation_reorder_allowed(self):
        # en reorders the two placeholders vs base; allowed because substitution is by name.
        self._validate({"k": _android_key({"hanji": "{a} {b}", "en": "{b} {a}"}, placeholders={"a": "int", "b": "int"})})

    def test_valid_format_key_passes(self):
        self._validate({"k": _android_key({"hanji": "{imported} ok {skipped}"}, placeholders={"imported": "int", "skipped": "int"})})


class FormatEmitTest(unittest.TestCase):
    def test_named_to_positional_reorder_and_typed_signature(self):
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            _write_namespace(
                repo,
                "probe",
                {
                    "imp": _android_key(
                        {"hanji": "{imported} ok {skipped}", "en": "{skipped} dup {imported}"},
                        placeholders={"imported": "int", "skipped": "int"},
                    ),
                },
            )
            outputs = build_outputs(repo)
            default_xml = outputs[f"{i18n_lib.ANDROID_RES_ROOT}/values/strings_i18n.xml"]
            self.assertIn("%1$d ok %2$d", default_xml)
            # Reorder maps by name: skipped stays %2$d, imported stays %1$d even though en flips order.
            en_xml = outputs[f"{i18n_lib.ANDROID_RES_ROOT}/values-en/strings_i18n.xml"]
            self.assertIn("%2$d dup %1$d", en_xml)
            formats = outputs[f"{i18n_lib.GEN_PKG_DIR}/StringResolverFormats.kt"]
            self.assertIn("fun StringResolver.probeImp(imported: Int, skipped: Int): String", formats)
            self.assertIn("formatString(StringKey.PROBE_IMP, imported, skipped)", formats)
            # A format key must NOT get a plain L10n getter (would leak the raw template).
            self.assertNotIn("probeImp", outputs[f"{i18n_lib.GEN_PKG_DIR}/L10n.kt"])
            # Kotlin-map source must escape the inserted `$` (`%1\$d`); the unescaped `%1$d` would be a
            # Kotlin string template (`$d` interpolation) and fail to compile.
            pseudo_map = outputs[f"{i18n_lib.GEN_PKG_DIR}/GeneratedPseudoStrings.kt"]
            self.assertIn("%1\\$d", pseudo_map)
            self.assertNotIn("%1$d", pseudo_map)

    def test_literal_percent_escaped_in_format_key(self):
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            _write_namespace(repo, "probe", {"pct": _android_key({"hanji": "{count}% done"}, placeholders={"count": "int"})})
            outputs = build_outputs(repo)
            self.assertIn("%1$d%% done", outputs[f"{i18n_lib.ANDROID_RES_ROOT}/values/strings_i18n.xml"])

    def test_plain_key_percent_untouched(self):
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            _write_namespace(repo, "probe", {"k": _android_key({"hanji": "100% sure"})})
            outputs = build_outputs(repo)
            self.assertIn("100% sure", outputs[f"{i18n_lib.ANDROID_RES_ROOT}/values/strings_i18n.xml"])

    def test_no_format_keys_emits_placeholder_comment(self):
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            _write_namespace(repo, "probe", {"k": _android_key({"hanji": "字"})})
            formats = build_outputs(repo)[f"{i18n_lib.GEN_PKG_DIR}/StringResolverFormats.kt"]
            self.assertIn("No format-arg keys", formats)
            self.assertNotIn("import", formats)  # no unused imports when there are no accessors


def _ios_key(values: dict, placeholders: dict | None = None, comment: str | None = None) -> dict:
    entry = {"scope": {"platforms": ["android", "ios"], "surfaces": ["host"]}, "values": values}
    if placeholders is not None:
        entry["placeholders"] = placeholders
    if comment is not None:
        entry["comment"] = comment
    return entry


class IOSEmitTest(unittest.TestCase):
    def _outputs(self, keys, namespace="probe"):
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            _write_namespace(repo, namespace, keys)
            return build_outputs(repo)

    def test_xcstrings_source_language_is_hanji_not_en(self):
        # Codex Q1: sourceLanguage MUST be the Hanji base. "en" makes Xcode emit the synthetic key as
        # its own en value, defeating the runtime sentinel fallback so English renders the raw key.
        catalog = json.loads(self._outputs({"k": _ios_key({"hanji": "字"})})[i18n_lib.IOS_XCSTRINGS])
        self.assertEqual(catalog["sourceLanguage"], i18n_lib.BCP47_HANJI)

    def test_xcstrings_keyed_by_res_name_with_only_authored_langs(self):
        catalog = json.loads(self._outputs({"k": _ios_key({"hanji": "字"}, comment="c")})[i18n_lib.IOS_XCSTRINGS])
        unit = catalog["strings"]["i18n_probe_k"]
        self.assertEqual(unit["comment"], "c")
        # Only Hanji authored -> only the nan-Hant-TW localization exists; en/ja absent (no fake fill).
        self.assertEqual(list(unit["localizations"]), ["nan-Hant-TW"])
        self.assertEqual(unit["localizations"]["nan-Hant-TW"]["stringUnit"]["value"], "字")

    def test_xcstrings_authored_language_uses_bcp47_tag(self):
        catalog = json.loads(self._outputs({"k": _ios_key({"hanji": "字", "en": "Word"})})[i18n_lib.IOS_XCSTRINGS])
        self.assertIn("en", catalog["strings"]["i18n_probe_k"]["localizations"])

    def test_ios_scope_filter_excludes_android_only(self):
        outputs = self._outputs(
            {
                "shared": _ios_key({"hanji": "共用"}),
                "androidOnly": {"scope": {"platforms": ["android"], "surfaces": ["host"]}, "values": {"hanji": "A"}},
            },
        )
        string_key_swift = outputs[f"{i18n_lib.IOS_GEN_DIR}/StringKey.swift"]
        self.assertIn('case probeShared = "i18n_probe_shared"', string_key_swift)
        self.assertNotIn("androidOnly", string_key_swift)  # android-only key absent from iOS catalog

    def test_ios_format_uses_lld_and_int64_cast(self):
        # iOS %lld (Swift Int is 64-bit) + Int64() cast — NOT Android's %d.
        outputs = self._outputs(
            {"imp": _ios_key({"hanji": "{imported} ok {skipped}"}, placeholders={"imported": "int", "skipped": "int"})},
        )
        catalog = json.loads(outputs[i18n_lib.IOS_XCSTRINGS])
        self.assertEqual(catalog["strings"]["i18n_probe_imp"]["localizations"]["nan-Hant-TW"]["stringUnit"]["value"], "%1$lld ok %2$lld")
        formats = outputs[f"{i18n_lib.IOS_GEN_DIR}/StringResolverFormats.swift"]
        self.assertIn("func probeImp(imported: Int, skipped: Int) -> String", formats)
        self.assertIn("format(.probeImp, Int64(imported), Int64(skipped))", formats)
        # Android side stays %d in the same build.
        self.assertIn("%1$d ok %2$d", outputs[f"{i18n_lib.ANDROID_RES_ROOT}/values/strings_i18n.xml"])

    def test_ios_pseudo_map_is_debug_gated(self):
        pseudo_swift = self._outputs({"k": _ios_key({"hanji": "字"})})[f"{i18n_lib.IOS_GEN_DIR}/GeneratedPseudoStrings.swift"]
        self.assertIn("#if DEBUG", pseudo_swift)
        self.assertIn("#endif", pseudo_swift)
        self.assertIn(".probeK:", pseudo_swift)

    def test_ios_no_format_keys_emits_comment(self):
        formats = self._outputs({"k": _ios_key({"hanji": "字"})})[f"{i18n_lib.IOS_GEN_DIR}/StringResolverFormats.swift"]
        self.assertIn("No format-arg keys", formats)
        self.assertNotIn("extension StringResolver", formats)


class GlobalValidationTest(unittest.TestCase):
    def test_duplicate_accessor_across_namespaces_rejected(self):
        # l10n_accessor("common","fooBar") == l10n_accessor("commonFoo","bar") == "commonFooBar".
        # Both keys are valid lowerCamelCase per-namespace, so only the cross-namespace global check
        # catches the collision that would emit two identical Swift cases / Kotlin vals.
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            _write_namespace(repo, "common", {"fooBar": _android_key({"hanji": "1"})})
            _write_namespace(repo, "commonFoo", {"bar": _android_key({"hanji": "2"})})
            with self.assertRaises(ValueError) as ctx:
                build_outputs(repo)
            self.assertIn("duplicate accessor", str(ctx.exception))


if __name__ == "__main__":
    unittest.main()
