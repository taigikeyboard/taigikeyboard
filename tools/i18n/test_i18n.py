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
    PRODUCTION_LANGUAGES,
    build_outputs,
    kotlin_escape,
    l10n_accessor,
    load_namespace,
    res_name,
    string_key_const,
    swift_escape,
    validate_namespace,
    validate_production_completeness,
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

    def test_xml_escape_backslash(self):
        # A literal backslash must reach the Android resource parser as `\\`, and must be escaped
        # before the quote/newline rules so their introduced backslashes are not doubled.
        self.assertEqual(xml_escape("a\\b"), "a\\\\b")
        self.assertEqual(xml_escape('a\\"b'), 'a\\\\\\"b')

    def test_kotlin_escape(self):
        self.assertEqual(kotlin_escape('a"$b'), 'a\\"\\$b')

    def test_swift_escape(self):
        self.assertEqual(swift_escape('a"b'), 'a\\"b')
        self.assertEqual(swift_escape("a\\(b)"), "a\\\\(b)")
        self.assertEqual(swift_escape("a\nb"), "a\\nb")


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

    def test_halfwidth_comma_in_hanji_rejected(self):
        with self.assertRaisesRegex(ValueError, "full-width comma"):
            self._validate({"k": _android_key({"hanji": "拍開,才會收著", "en": "x"})})

    def test_halfwidth_comma_in_other_languages_allowed(self):
        # Romanization and the OS locales punctuate half-width; the rule is Hanji's alone.
        self._validate({"k": _android_key({"hanji": "拍開，才會收著", "en": "a, b", "tailo": "phah-khui, tsiah"})})

    def test_comma_between_digits_in_hanji_allowed(self):
        # A numeric separator is how a number is written, not how a sentence is punctuated.
        self._validate({"k": _android_key({"hanji": "上限 30,000 項", "en": "x"})})

    def test_comma_with_a_digit_on_only_one_side_in_hanji_rejected(self):
        # The exemption needs a digit on BOTH sides — a comma that merely touches a number is
        # still sentence punctuation.
        for text in ("上限 30,項", "上限 ,000 項"):
            with self.subTest(text=text):
                with self.assertRaisesRegex(ValueError, "full-width comma"):
                    self._validate({"k": _android_key({"hanji": text, "en": "x"})})

    def test_valid_passes(self):
        self._validate({"k": _android_key({"hanji": "字", "en": "x"})})


class ProductionCompletenessTest(unittest.TestCase):
    # A fully-authored key: every production language present + non-empty. tailo/poj joined the roster
    # at promotion (R5-2 / R6-2), so a complete key now carries all five.
    _FULL = {"hanji": "字", "en": "x", "ja": "字", "tailo": "jī", "poj": "jī"}

    def _entries(self, values):
        return [("ns", "k", {"scope": {"platforms": ["android"], "surfaces": ["host"]}, "values": values})]

    def _without(self, lang):
        values = dict(self._FULL)
        del values[lang]
        return self._entries(values)

    def test_production_languages_roster_matches_platform_order(self):
        # CROSS-PLATFORM INVARIANT (INVARIANT_DISPLAY_LANGUAGE_PRODUCTION_ROSTER) — same set + order as the
        # platform productionLanguages rosters (ios DisplayLanguage.swift / android DisplayLanguage.kt §37).
        self.assertEqual(PRODUCTION_LANGUAGES, ("hanji", "en", "ja", "tailo", "poj"))

    def test_all_production_languages_passes(self):
        validate_production_completeness(self._entries(dict(self._FULL)))

    def test_missing_ja_rejected(self):
        with self.assertRaises(ValueError) as ctx:
            validate_production_completeness(self._without("ja"))
        self.assertIn("ja", str(ctx.exception))

    def test_missing_en_rejected(self):
        with self.assertRaises(ValueError) as ctx:
            validate_production_completeness(self._without("en"))
        self.assertIn("en", str(ctx.exception))

    def test_missing_tailo_rejected(self):
        # tailo joined the production roster (R5-2) — its absence now fails completeness.
        with self.assertRaises(ValueError) as ctx:
            validate_production_completeness(self._without("tailo"))
        self.assertIn("tailo", str(ctx.exception))

    def test_missing_poj_rejected(self):
        # poj joined the production roster (R6-2) — its absence now fails completeness.
        with self.assertRaises(ValueError) as ctx:
            validate_production_completeness(self._without("poj"))
        self.assertIn("poj", str(ctx.exception))

    def test_empty_production_value_rejected(self):
        values = dict(self._FULL)
        values["en"] = ""
        with self.assertRaises(ValueError):
            validate_production_completeness(self._entries(values))

    def test_build_outputs_enforces_when_flagged(self):
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            _write_namespace(repo, "probe", {"k": _android_key({"hanji": "字", "en": "x"})})  # partial
            build_outputs(repo)  # default: completeness off -> partial fixture allowed
            with self.assertRaises(ValueError):
                build_outputs(repo, enforce_production_completeness=True)


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

    def test_generated_map_partial_fixture_emits_tailo_only(self):
        # GeneratedMap path (tailo/poj have no OS locale): a fixture with tailo but no poj lands in both
        # platform maps, while poj stays empty. Neither value may leak into the iOS String Catalog.
        # This partial shape is exercised only by unit fixtures via the default (unenforced) build; the real
        # real source is held to tailo/poj completeness by validate_production_completeness (see below).
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            _write_namespace(repo, "probe", {"k": _ios_key({"hanji": "字", "tailo": "jī"})})
            outputs = build_outputs(repo)
            taigi_map = outputs[f"{i18n_lib.GEN_PKG_DIR}/GeneratedTaigiStrings.kt"]
            self.assertIn('StringKey.PROBE_K to "jī"', taigi_map)
            self.assertIn("private val poj: Map<StringKey, String> = emptyMap()", taigi_map)
            ios_map = outputs[f"{i18n_lib.IOS_GEN_DIR}/GeneratedTaigiStrings.swift"]
            self.assertIn('.probeK: "jī"', ios_map)
            self.assertIn("private static let poj: [StringKey: String] = [:]", ios_map)
            xcstrings = outputs[i18n_lib.IOS_XCSTRINGS]
            self.assertNotIn("nan-Latn-TW-x-tailo", xcstrings)
            self.assertNotIn("nan-Latn-TW-x-poj", xcstrings)

    def test_generated_map_emits_both_tailo_and_poj(self):
        # Authored as a lockstep pair (tailo + poj both hand-authored): both land in Kotlin and Swift maps,
        # while the iOS catalog stays native-language-only.
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            # Distinct tailo/poj so the map is proven to carry the poj value, not a copy of tailo.
            _write_namespace(repo, "probe", {"k": _ios_key({"hanji": "字", "tailo": "tsuā", "poj": "chōa"})})
            outputs = build_outputs(repo)
            taigi_map = outputs[f"{i18n_lib.GEN_PKG_DIR}/GeneratedTaigiStrings.kt"]
            self.assertIn('StringKey.PROBE_K to "tsuā"', taigi_map)  # under `private val tailo`
            self.assertIn('StringKey.PROBE_K to "chōa"', taigi_map)  # under `private val poj`
            self.assertNotIn("emptyMap()", taigi_map)
            ios_map = outputs[f"{i18n_lib.IOS_GEN_DIR}/GeneratedTaigiStrings.swift"]
            self.assertIn('.probeK: "tsuā"', ios_map)
            self.assertIn('.probeK: "chōa"', ios_map)
            xcstrings = outputs[i18n_lib.IOS_XCSTRINGS]
            self.assertNotIn("nan-Latn-TW-x-tailo", xcstrings)
            self.assertNotIn("nan-Latn-TW-x-poj", xcstrings)


class GeneratedMapCompletenessTest(unittest.TestCase):
    def _build(self, values: dict) -> None:
        # Scoped to every platform because the production flag models the REAL source set, which each
        # platform draws from — a Swift platform scoped to nothing fails its own gate first and would
        # mask the language-completeness error these cases are about.
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            _write_namespace(repo, "probe", {"k": _all_platform_key(values)})
            build_outputs(repo, enforce_production_completeness=True)

    def test_tailo_without_poj_rejected(self):
        # tailo + poj are production languages, so a half-authored pair fails the single production gate:
        # poj missing -> "missing production language(s) ['poj']". (No separate lockstep gate.)
        with self.assertRaisesRegex(ValueError, "missing production language"):
            self._build({"hanji": "字", "tailo": "jī", "ja": "字", "en": "char"})

    def test_poj_without_tailo_rejected(self):
        with self.assertRaisesRegex(ValueError, "missing production language"):
            self._build({"hanji": "字", "poj": "jī", "ja": "字", "en": "char"})

    def test_both_present_passes(self):
        self._build({"hanji": "字", "tailo": "jī", "poj": "jī", "ja": "字", "en": "char"})

    def test_neither_present_rejected(self):
        # A key authoring neither tailo nor poj fails production completeness — every picker option must render.
        with self.assertRaisesRegex(ValueError, "missing production language"):
            self._build({"hanji": "字", "ja": "字", "en": "char"})


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


def _build_probe_outputs(keys, namespace="probe") -> dict:
    """Generate every artifact from one throwaway namespace — the shape every emitter test needs."""
    with tempfile.TemporaryDirectory() as tmp:
        repo = Path(tmp)
        _write_namespace(repo, namespace, keys)
        return build_outputs(repo)


class IOSEmitTest(unittest.TestCase):
    _outputs = staticmethod(_build_probe_outputs)

    def test_xcstrings_source_language_is_native_english(self):
        catalog = json.loads(
            self._outputs({"k": _ios_key({"hanji": "字", "en": "Character"})})[i18n_lib.IOS_XCSTRINGS]
        )
        self.assertEqual(catalog["sourceLanguage"], "en")

    def test_xcstrings_keyed_by_res_name_with_only_native_authored_langs(self):
        catalog = json.loads(self._outputs({"k": _ios_key({"hanji": "字"}, comment="c")})[i18n_lib.IOS_XCSTRINGS])
        unit = catalog["strings"]["i18n_probe_k"]
        self.assertEqual(unit["comment"], "c")
        self.assertEqual(unit["localizations"], {})
        ios_map = self._outputs({"k": _ios_key({"hanji": "字"})})[
            f"{i18n_lib.IOS_GEN_DIR}/GeneratedTaigiStrings.swift"
        ]
        self.assertIn('.probeK: "字"', ios_map)

    def test_xcstrings_authored_language_uses_bcp47_tag(self):
        catalog = json.loads(self._outputs({"k": _ios_key({"hanji": "字", "en": "Word"})})[i18n_lib.IOS_XCSTRINGS])
        self.assertIn("en", catalog["strings"]["i18n_probe_k"]["localizations"])

    def test_xcstrings_never_emits_product_language_bundle_tags(self):
        outputs = self._outputs(
            {"k": _ios_key({"hanji": "字", "tailo": "jī", "poj": "jī", "en": "Word", "ja": "字"})}
        )
        xcstrings = outputs[i18n_lib.IOS_XCSTRINGS]
        self.assertNotIn("nan-Hant-TW", xcstrings)
        self.assertNotIn("nan-Latn-TW-x-tailo", xcstrings)
        self.assertNotIn("nan-Latn-TW-x-poj", xcstrings)
        self.assertEqual(set(json.loads(xcstrings)["strings"]["i18n_probe_k"]["localizations"]), {"en", "ja"})

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
        generated_map = outputs[f"{i18n_lib.IOS_GEN_DIR}/GeneratedTaigiStrings.swift"]
        self.assertIn('.probeImp: "%1$lld ok %2$lld"', generated_map)
        formats = outputs[f"{i18n_lib.IOS_GEN_DIR}/StringResolverFormats.swift"]
        self.assertIn("func probeImp(imported: Int, skipped: Int) -> String", formats)
        self.assertIn("format(.probeImp, Int64(imported), Int64(skipped))", formats)
        # Android side stays %d in the same build.
        self.assertIn("%1$d ok %2$d", outputs[f"{i18n_lib.ANDROID_RES_ROOT}/values/strings_i18n.xml"])

    def test_ios_no_format_keys_emits_comment(self):
        formats = self._outputs({"k": _ios_key({"hanji": "字"})})[f"{i18n_lib.IOS_GEN_DIR}/StringResolverFormats.swift"]
        self.assertIn("No format-arg keys", formats)
        self.assertNotIn("extension StringResolver", formats)


class PlaceholderTypeTest(unittest.TestCase):
    _outputs = staticmethod(_build_probe_outputs)

    def test_placeholder_type_vocabulary(self):
        # Adding a type here changes every platform's accessor signature and format spec, so the
        # roster is pinned rather than left to whatever a caller happens to declare.
        self.assertEqual(set(i18n_lib.PLACEHOLDER_TYPES), {"int", "string"})

    def test_string_placeholder_emits_a_string_arg_with_no_cast(self):
        # `string` exists for text the product does not author — a store's own error description —
        # so the punctuation around it is authored per language instead of concatenated in code.
        # Kotlin takes `%s`; Swift takes `%@` and, unlike `int`, needs no widening cast.
        outputs = self._outputs(
            {
                "why": {
                    "scope": {"platforms": ["android", "ios", "macos"], "surfaces": ["host"]},
                    "placeholders": {"reason": "string"},
                    "values": {
                        "hanji": "失敗({reason})",
                        "tailo": "sit-pāi({reason})",
                        "poj": "sit-pāi({reason})",
                        "en": "failed ({reason})",
                        "ja": "失敗（{reason}）",
                    },
                }
            },
        )
        for gen_dir in (i18n_lib.IOS_GEN_DIR, i18n_lib.MACOS_GEN_DIR):
            formats = outputs[f"{gen_dir}/StringResolverFormats.swift"]
            self.assertIn("func probeWhy(reason: String) -> String", formats)
            self.assertIn("format(.probeWhy, reason)", formats)
        kotlin = outputs[f"{i18n_lib.GEN_PKG_DIR}/StringResolverFormats.kt"]
        self.assertIn("reason: String", kotlin)
        # Each language keeps its own punctuation around the spec — the whole reason the reason is
        # a placeholder rather than something the call site concatenates.
        macos_map = outputs[f"{i18n_lib.MACOS_GEN_DIR}/GeneratedStrings.swift"]
        self.assertIn('.probeWhy: "失敗(%1$@)"', macos_map)
        self.assertIn('.probeWhy: "failed (%1$@)"', macos_map)
        self.assertIn('.probeWhy: "失敗（%1$@）"', macos_map)


def _all_platform_key(values: dict, placeholders: dict | None = None) -> dict:
    entry = {"scope": {"platforms": ["android", "ios", "macos"], "surfaces": ["host"]}, "values": values}
    if placeholders is not None:
        entry["placeholders"] = placeholders
    return entry


class MacOSEmitTest(unittest.TestCase):
    _outputs = staticmethod(_build_probe_outputs)

    def test_valid_platforms_roster(self):
        # The scope vocabulary is the platform roster; a fourth platform must be added here first.
        self.assertEqual(i18n_lib.VALID_PLATFORMS, {"ios", "android", "macos"})

    def test_map_carries_every_production_language(self):
        # The whole point of the macOS shape: en/ja resolve from the generated map too, because the
        # package ships no string catalog and no .lproj bundles. Every language gets a DISTINCT
        # sentinel so a map wired to the wrong language cannot pass.
        outputs = self._outputs(
            {
                "k": _all_platform_key(
                    {"hanji": "字", "tailo": "jī-tailo", "poj": "jī-poj", "en": "Word", "ja": "文字"},
                )
            },
        )
        macos_map = outputs[f"{i18n_lib.MACOS_GEN_DIR}/GeneratedStrings.swift"]
        for case, value in (
            ("hanji", "字"),
            ("tailo", "jī-tailo"),
            ("poj", "jī-poj"),
            ("japanese", "文字"),
            ("english", "Word"),
        ):
            # Assert the value sits under ITS OWN map, not merely somewhere in the file.
            declaration = f"    private static let {case}: [StringKey: String] = [\n"
            self.assertIn(declaration, macos_map)
            body = macos_map.split(declaration, 1)[1].split("    ]", 1)[0]
            self.assertIn(f'.probeK: "{value}",', body)
            self.assertIn(f"        case .{case}: {case}[key]", macos_map)
        # `.system` is a selection policy with no strings — the lookup must not answer for it.
        self.assertIn("case .system: nil", macos_map)

    def test_map_language_roster_is_derived_from_the_production_roster(self):
        # Promoting a sixth production language must reach the macOS map automatically: a hand-kept
        # second roster would leave that language resolving to the Hanji fallback with every gate green.
        self.assertEqual([lang for lang, _case in i18n_lib.MACOS_MAP_LANGUAGES], list(PRODUCTION_LANGUAGES))

    def test_map_emits_empty_dictionary_for_an_unauthored_language(self):
        macos_map = self._outputs({"k": _all_platform_key({"hanji": "字"})})[
            f"{i18n_lib.MACOS_GEN_DIR}/GeneratedStrings.swift"
        ]
        self.assertIn("private static let english: [StringKey: String] = [:]", macos_map)

    def test_scope_filtering_is_three_way(self):
        outputs = self._outputs(
            {
                "shared": _all_platform_key({"hanji": "共用"}),
                "macosOnly": {"scope": {"platforms": ["macos"], "surfaces": ["host"]}, "values": {"hanji": "M"}},
                "iosOnly": {"scope": {"platforms": ["ios"], "surfaces": ["host"]}, "values": {"hanji": "I"}},
                "androidOnly": {"scope": {"platforms": ["android"], "surfaces": ["host"]}, "values": {"hanji": "A"}},
            },
        )
        macos_keys = outputs[f"{i18n_lib.MACOS_GEN_DIR}/StringKey.swift"]
        ios_keys = outputs[f"{i18n_lib.IOS_GEN_DIR}/StringKey.swift"]
        android_xml = outputs[f"{i18n_lib.ANDROID_RES_ROOT}/values/strings_i18n.xml"]

        self.assertIn("probeShared", macos_keys)
        self.assertIn("probeMacosOnly", macos_keys)
        self.assertNotIn("probeIosOnly", macos_keys)
        self.assertNotIn("probeAndroidOnly", macos_keys)

        self.assertIn("probeIosOnly", ios_keys)
        self.assertNotIn("probeMacosOnly", ios_keys)

        self.assertIn("i18n_probe_shared", android_xml)
        self.assertNotIn("i18n_probe_macosOnly", android_xml)

    def test_adding_macos_scope_leaves_ios_and_android_artifacts_byte_identical(self):
        # Scoping an existing key to macOS must be additive: the guarantee that lets PR-by-PR macOS
        # adoption never touch a shipped iOS or Android string.
        keys = {"k": _ios_key({"hanji": "字", "en": "Word"})}
        before = self._outputs(keys)
        after = self._outputs({"k": _all_platform_key({"hanji": "字", "en": "Word"})})
        for path, content in before.items():
            if path.startswith(i18n_lib.MACOS_GEN_DIR):
                continue  # the macOS artifacts are exactly what the added scope is supposed to change
            self.assertEqual(after[path], content, path)

    def test_format_accessor_reuses_the_swift_facet(self):
        # IOSEmitTest already pins the %lld + Int64 lowering itself; the macOS delta is that the SAME
        # facet reaches the macOS artifacts rather than Android's %d.
        outputs = self._outputs(
            {
                "imp": _all_platform_key(
                    {"hanji": "{imported} ok {skipped}"},
                    placeholders={"imported": "int", "skipped": "int"},
                )
            },
        )
        self.assertIn('.probeImp: "%1$lld ok %2$lld"', outputs[f"{i18n_lib.MACOS_GEN_DIR}/GeneratedStrings.swift"])
        self.assertIn(
            "format(.probeImp, Int64(imported), Int64(skipped))",
            outputs[f"{i18n_lib.MACOS_GEN_DIR}/StringResolverFormats.swift"],
        )

    def test_plural_accessor_names_the_generated_map_as_the_fallback_source(self):
        # Same runtime arm-selection as iOS, but the comment must not claim a catalog macOS has not got.
        entry = {
            "scope": {"platforms": ["ios", "macos"], "surfaces": ["host"]},
            "values": _PLURAL_VALUES,
            "placeholders": _PLURAL_PLACEHOLDERS,
        }
        outputs = self._outputs({"imp": entry})
        macos_formats = outputs[f"{i18n_lib.MACOS_GEN_DIR}/StringResolverFormats.swift"]
        self.assertIn("customDict == 1 ?", macos_formats)
        self.assertIn("the generated map holds the 'other' fallback", macos_formats)
        self.assertIn("the catalog holds the 'other' fallback", outputs[f"{i18n_lib.IOS_GEN_DIR}/StringResolverFormats.swift"])

    def test_no_macos_keys_still_emits_the_same_file_set(self):
        # The artifact LIST must not depend on how keys are scoped — check.py compares a fixed set of
        # paths, so a platform dropping out of the map would silently stop being gated. What those
        # files contain when the scope is empty is a separate contract; see the production gate below.
        scoped = set(self._outputs({"k": _all_platform_key({"hanji": "字"})}))
        unscoped = set(self._outputs({"k": _ios_key({"hanji": "字"})}))
        self.assertEqual(scoped, unscoped)
        self.assertIn(f"{i18n_lib.MACOS_GEN_DIR}/StringKey.swift", unscoped)

    def test_production_build_rejects_a_swift_platform_with_no_keys(self):
        # `enum StringKey: String {}` is not legal Swift ("an enum with no cases cannot declare a raw
        # type"), so an empty Swift scope would emit a package that cannot compile. Fail at generate
        # time, naming the cause. Unit fixtures scope keys to one platform at a time on purpose, so
        # this fires only under the production flag the real CLI passes.
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            _write_namespace(
                repo,
                "probe",
                {"k": _android_key({lang: "x" for lang in PRODUCTION_LANGUAGES})},
            )
            with self.assertRaises(ValueError) as ctx:
                build_outputs(repo, enforce_production_completeness=True)
            self.assertIn("no key is scoped to 'ios'", str(ctx.exception))


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


def _plural_key(values: dict, placeholders: dict) -> dict:
    # A format key scoped to both platforms so the Kotlin AND Swift format emitters both produce it.
    return {"scope": {"platforms": ["android", "ios"], "surfaces": ["host"]}, "values": values, "placeholders": placeholders}


# A two-count plural message: each count selects its own one/other arm independently.
_PLURAL_VALUES = {
    "hanji": "匯入 {customDict} 自訂、{frequency} 詞頻",
    "en": "Imported {customDict, plural, one {# custom entry} other {# custom entries}}, "
    "{frequency, plural, one {# frequency record} other {# frequency records}}",
}
_PLURAL_PLACEHOLDERS = {"customDict": "int", "frequency": "int"}


class PluralParseTest(unittest.TestCase):
    def test_flat_message_parses_to_lit_and_arg(self):
        self.assertEqual(
            i18n_lib.parse_message("a {x} b"),
            [("lit", "a "), ("arg", "x"), ("lit", " b")],
        )

    def test_plural_parses_arms_and_hash_is_the_count(self):
        nodes = i18n_lib.parse_message("{n, plural, one {# item} other {# items}}")
        self.assertEqual(nodes[0][0], "plural")
        self.assertEqual(nodes[0][1], "n")
        # `#` inside an arm lowers to the plural's own count argument.
        self.assertEqual(nodes[0][2]["one"], [("arg", "n"), ("lit", " item")])
        self.assertEqual(nodes[0][2]["other"], [("arg", "n"), ("lit", " items")])

    def test_names_in_order_collects_plural_names(self):
        names = i18n_lib._placeholder_names_in_order(_PLURAL_VALUES["en"])
        self.assertEqual(names, ["customDict", "frequency"])

    def test_missing_other_arm_rejected(self):
        with self.assertRaises(ValueError) as ctx:
            i18n_lib.parse_message("{n, plural, one {# item}}")
        self.assertIn("other", str(ctx.exception))

    def test_unsupported_category_rejected(self):
        with self.assertRaises(ValueError) as ctx:
            i18n_lib.parse_message("{n, plural, many {# items} other {# items}}")
        self.assertIn("category", str(ctx.exception))

    def test_nested_placeholder_in_arm_rejected(self):
        with self.assertRaises(ValueError):
            i18n_lib.parse_message("{n, plural, one {# {x}} other {# items}}")

    def test_non_plural_argument_keyword_rejected(self):
        with self.assertRaises(ValueError) as ctx:
            i18n_lib.parse_message("{n, select, other {x}}")
        self.assertIn("plural", str(ctx.exception))

    def test_icu_quoted_brace_rejected(self):
        # `'{n}'` is ICU quoting (literal braces); unsupported, must raise rather than mis-parse.
        with self.assertRaises(ValueError) as ctx:
            i18n_lib.parse_message("a '{n}' b")
        self.assertIn("quoting", str(ctx.exception))

    def test_icu_quoted_hash_in_arm_rejected(self):
        with self.assertRaises(ValueError) as ctx:
            i18n_lib.parse_message("{n, plural, one {'#' item} other {# items}}")
        self.assertIn("quoting", str(ctx.exception))

    def test_lone_apostrophe_in_prose_is_literal(self):
        # A lone apostrophe (not beginning ICU quoting) stays literal — real copy has `user's`.
        self.assertEqual(i18n_lib.parse_message("user's data"), [("lit", "user's data")])


class PluralValidateTest(unittest.TestCase):
    def _validate(self, keys: dict) -> None:
        validate_namespace("probe", {"namespace": "probe", "keys": keys}, Path("probe.json"))

    def test_translation_plural_with_flat_base_passes(self):
        self._validate({"k": _plural_key(_PLURAL_VALUES, _PLURAL_PLACEHOLDERS)})

    def test_plural_in_base_language_rejected(self):
        with self.assertRaises(ValueError) as ctx:
            self._validate({"k": _plural_key({"hanji": "{n, plural, one {#} other {#}}"}, {"n": "int"})})
        self.assertIn("must not use plural", str(ctx.exception))


class PluralEmitTest(unittest.TestCase):
    def _outputs(self) -> dict:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            _write_namespace(repo, "probe", {"imp": _plural_key(_PLURAL_VALUES, _PLURAL_PLACEHOLDERS)})
            return build_outputs(repo)

    def test_android_resource_holds_other_arm(self):
        # Native resources render the `other` arm — the runtime fallback the typed accessor overrides.
        en_xml = self._outputs()[f"{i18n_lib.ANDROID_RES_ROOT}/values-en/strings_i18n.xml"]
        self.assertIn("%1$d custom entries, %2$d frequency records", en_xml)
        self.assertNotIn("plural", en_xml)
        self.assertNotIn("custom entry,", en_xml)  # the singular arm never reaches the resource

    def test_kotlin_accessor_selects_arm_by_language_and_count(self):
        formats = self._outputs()[f"{i18n_lib.GEN_PKG_DIR}/StringResolverFormats.kt"]
        self.assertIn("if (displayLanguage == DisplayLanguage.ENGLISH)", formats)
        # Each count selects its own arm independently (assert the 1st AND 2nd placeholder).
        self.assertIn('(if (customDict == 1) "%1\\$d custom entry" else "%1\\$d custom entries")', formats)
        self.assertIn('(if (frequency == 1) "%2\\$d frequency record" else "%2\\$d frequency records")', formats)
        self.assertIn("formatTemplate(", formats)
        self.assertIn("formatString(StringKey.PROBE_IMP, customDict, frequency)", formats)  # else fallback
        self.assertIn("import com.siansiansu.taigikeyboard.i18n.DisplayLanguage", formats)
        self.assertIn("import com.siansiansu.taigikeyboard.i18n.formatTemplate", formats)

    def test_swift_accessor_selects_arm_by_language_and_count(self):
        formats = self._outputs()[f"{i18n_lib.IOS_GEN_DIR}/StringResolverFormats.swift"]
        self.assertIn("if language == .english", formats)
        # Each count selects its own arm independently (assert the 1st AND 2nd placeholder).
        self.assertIn('(customDict == 1 ? "%1$lld custom entry" : "%1$lld custom entries")', formats)
        self.assertIn('(frequency == 1 ? "%2$lld frequency record" : "%2$lld frequency records")', formats)
        self.assertIn("formatTemplate(", formats)
        self.assertIn("format(.probeImp, Int64(customDict), Int64(frequency))", formats)  # default fallback

    def test_plural_authored_for_non_english_rejected(self):
        # Only English carries a plural selector (CLDR en: n == 1); a plural in another language would
        # silently apply the wrong rule, so the codegen rejects it loudly.
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            _write_namespace(
                repo,
                "probe",
                {"imp": _plural_key({"hanji": "{n} x", "ja": "{n, plural, one {#} other {#}}"}, {"n": "int"})},
            )
            with self.assertRaises(ValueError) as ctx:
                build_outputs(repo)
            self.assertIn("only English", str(ctx.exception))


# In-app content (i18n/content/*.json) is a nested tree with its OWN schema, separate from the flat
# i18n/*.json namespaces and NOT emitted by `make i18n` (the codegen glob is non-recursive — i18n_lib.py
# scans i18n/*.json only, so the i18n/content/ subfolder is skipped); the platforms decode it directly.
# The constants below mirror the iOS/Android LocalizedContentText model.
CONTENT_FILES = ["i18n/content/features.json", "i18n/content/faq.json"]
EXPECTED_CONTENT_LANGS = {"hanji", "tailo", "poj", "en", "ja"}
EXPECTED_CONTENT_STRING_COUNT = 80


def _content_canonical(doc: dict) -> str:
    # The one serialization content/*.json is stored in — json.dumps(indent=2, ensure_ascii=False).
    return json.dumps(doc, ensure_ascii=False, indent=2)


class ProductionContentTests(unittest.TestCase):
    # Smoke test against the REAL committed content/*.json (no Node, no mock). A normal iOS/Android build
    # does NOT prove the bundled JSON decodes — the iOS loader silently returns [] on a decode failure — so
    # this gates that production content is well-formed, canonical, and complete in all 5 languages.
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
        repo_root = Path(__file__).resolve().parents[2]
        total = 0
        for rel in CONTENT_FILES:
            path = repo_root / rel
            text = path.read_text(encoding="utf-8")
            doc = json.loads(text)
            self.assertEqual(text, _content_canonical(doc), f"{rel} is not in canonical json.dumps(indent=2) form")
            for obj in self._localizable(doc):
                total += 1
                self.assertEqual(set(obj), EXPECTED_CONTENT_LANGS,
                                 f"{rel}: localizable object missing/extra languages: {sorted(obj)}")
                for lang, value in obj.items():
                    self.assertIsInstance(value, str, f"{rel}: {lang} is not a string")
                    self.assertTrue(value.strip(), f"{rel}: {lang} is empty")
        self.assertEqual(total, EXPECTED_CONTENT_STRING_COUNT, "expected 80 localizable content strings")


if __name__ == "__main__":
    unittest.main()
