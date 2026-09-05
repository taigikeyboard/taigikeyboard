//! Custom-dictionary cross-mode search-key derivation (v3.6.1 R3).
//!
//! WRITE ([`derive_custom_search_keys`]): from a stored raw custom-dictionary
//! roman, materialize the full {tl, poj, tps} × {num, notone, abbrev} (+ TPS
//! er/or dialect variant) search-key bundle. A query typed in ANY input mode
//! (TL / POJ / TPS-Bopomofo) can then find the entry, mirroring the system
//! dictionary's three-family FST (`dictionary/build/create_fst.py:128-144`).
//!
//! QUERY ([`derive_custom_query_key`]): from the user's current raw input +
//! settings input mode, produce the single family-native key tagged with its
//! primary form. The platform matches it against the entry's {primary-form,
//! abbrev} rows in the same family.
//!
//! Byte-identity contract: the custom search keys are matched ONLY within
//! `custom_dictionary.db`'s own SQLite query — never against the system FST —
//! so they need NOT byte-match the build pipeline. The invariant that matters
//! is WRITE-key == QUERY-key for the same word in the same family, which holds
//! because both sides flow through THIS module's ops. The raw `roman` column
//! and the engine's `composing::shadow::custom_toneless_key` lattice path are
//! untouched (they read `roman`, not these keys).

// 自訂詞庫跨輸入模式搜尋鍵衍生 (R3)。寫入端把單一 roman 展成 tl/poj/tps × num/notone/abbrev
//   多家族鍵 (對齊系統字典三索引);查詢端依當前 input + mode 產生單一家族鍵。鍵只在
//   custom_dictionary.db 內比對,不碰系統 FST,故只要 write==query 同字即可,raw roman 路徑不動。

use crate::api::{
    contains_tps, parse_input_mode, poj_display_to_tl_display, tl_display_to_poj_display,
    to_tone_number, InputMode,
};
use crate::derivation::{derive_abbrev, derive_notone};
use crate::tps::{
    from_zhuyin, is_tps_tone_mark, normalize_tps_tone8_scalar, tps_abbrev_from_tl,
    tps_notone_from_tl, tps_notone_or_variant, tps_num_from_tl,
};
use std::collections::HashSet;

/// Family prefixes — mirror the system-dict FST key families.
const FAMILY_TL: &str = "tl";
const FAMILY_POJ: &str = "poj";
const FAMILY_TPS: &str = "tps";

/// Forms — mirror the existing `notone` / `abbrev` / `roman_num` columns.
const FORM_NUM: &str = "num";
const FORM_NOTONE: &str = "notone";
const FORM_ABBREV: &str = "abbrev";

/// One materialized search key. `family` / `form` are stable string tags
/// shared with the SQLite side table; `key` is the fused family-native search
/// string. Native struct (dispatch maps it to the proto `CustomSearchKey`).
pub(crate) struct CustomSearchKey {
    pub family: &'static str,
    pub form: &'static str,
    pub key: String,
}

/// WRITE side — full search-key bundle for a stored custom-dict roman.
pub(crate) fn derive_custom_search_keys(roman: &str) -> Vec<CustomSearchKey> {
    if roman.trim().is_empty() {
        return Vec::new();
    }
    // Canonical TL display form (mode-independent identity surface). A Bopomofo
    // dirty-row (the DB schema does not forbid one) folds via `from_zhuyin`; a
    // POJ / TL latin display folds via `poj_display_to_tl_display` (identity
    // for TL input — it only rewrites ch→ts / oa→ua etc.).
    let tl = if contains_tps(roman) {
        from_zhuyin(roman)
    } else {
        poj_display_to_tl_display(roman)
    };
    let poj = tl_display_to_poj_display(&tl);

    let mut keys = Vec::new();
    push_latin_family(&mut keys, FAMILY_TL, &tl);
    push_latin_family(&mut keys, FAMILY_POJ, &poj);
    push_tps_family(&mut keys, &tl);
    dedup_nonempty(keys)
}

/// READ side — single family-native key for the current input + mode. `None`
/// for empty / residue-only input.
pub(crate) fn derive_custom_query_key(input: &str, input_mode: &str) -> Option<CustomSearchKey> {
    if input.trim().is_empty() {
        return None;
    }
    // Effective family mirrors the composing dispatch TPS upgrade
    // (`engine/composing/src/dispatch.rs:208`): raw input carrying TPS Bopomofo
    // is TPS regardless of the settings mode (Android `NormalizeMode` has no
    // TPS variant, so settings alone is insufficient).
    let family = if contains_tps(input) {
        FAMILY_TPS
    } else {
        match parse_input_mode(input_mode) {
            InputMode::Poj => FAMILY_POJ,
            InputMode::Tl | InputMode::Tps | InputMode::English => FAMILY_TL,
        }
    };

    let (form, key) = if family == FAMILY_TPS {
        // TPS tone marks are spacing modifier letters, NOT ASCII digits, so the
        // tone-aware test is glyph presence; the toneless form additionally
        // strips them to match the stored `tps:notone` key.
        let tone_aware = input.chars().any(is_tps_tone_mark);
        let form = if tone_aware { FORM_NUM } else { FORM_NOTONE };
        (form, strip_tps_input(input, tone_aware))
    } else {
        let tone_aware = input.chars().any(|c| c.is_ascii_digit());
        if tone_aware {
            (FORM_NUM, fuse_latin_numeric(input))
        } else {
            (FORM_NOTONE, derive_notone(input))
        }
    };

    if key.is_empty() {
        return None;
    }
    Some(CustomSearchKey { family, form, key })
}

// ---- WRITE helpers ------------------------------------------------------

fn push_latin_family(keys: &mut Vec<CustomSearchKey>, family: &'static str, display: &str) {
    push_key(
        keys,
        family,
        FORM_NUM,
        fuse_latin_numeric(&to_tone_number(display)),
    );
    push_key(keys, family, FORM_NOTONE, derive_notone(display));
    push_key(keys, family, FORM_ABBREV, derive_abbrev(display));
}

fn push_tps_family(keys: &mut Vec<CustomSearchKey>, tl: &str) {
    let num = tps_num_from_tl(tl);
    let notone = tps_notone_from_tl(tl);
    let abbrev = tps_abbrev_from_tl(tl);
    // Primary forms.
    push_key(keys, FAMILY_TPS, FORM_NUM, num.clone());
    push_key(keys, FAMILY_TPS, FORM_NOTONE, notone.clone());
    push_key(keys, FAMILY_TPS, FORM_ABBREV, abbrev.clone());
    // er/or dialect variants — additional same-(family, form) rows (ㄜ→ㄛ) so a
    // query producing whichever glyph the user typed still matches. Empty when
    // the form carries no ㄜ.
    push_key(keys, FAMILY_TPS, FORM_NUM, tps_notone_or_variant(&num));
    push_key(
        keys,
        FAMILY_TPS,
        FORM_NOTONE,
        tps_notone_or_variant(&notone),
    );
    push_key(
        keys,
        FAMILY_TPS,
        FORM_ABBREV,
        tps_notone_or_variant(&abbrev),
    );
}

fn push_key(
    keys: &mut Vec<CustomSearchKey>,
    family: &'static str,
    form: &'static str,
    key: String,
) {
    if !key.is_empty() {
        keys.push(CustomSearchKey { family, form, key });
    }
}

fn dedup_nonempty(keys: Vec<CustomSearchKey>) -> Vec<CustomSearchKey> {
    let mut seen = HashSet::new();
    keys.into_iter()
        .filter(|k| seen.insert((k.family, k.form, k.key.clone())))
        .collect()
}

// ---- Shared key shaping -------------------------------------------------

/// Lowercase a latin numeric form, take its base form, and drop hyphen +
/// space so it fuses to the stored `*:num` key shape (e.g. `guá-sī` →
/// `gua2-si7` → `gua2si7`).
///
/// [`taigi_unicode_base_form`] is what keeps the WRITE key (built from the
/// stored display roman, so `so͘2` / `kiaⁿ1`) and the QUERY key (built from the
/// raw keyboard buffer, so `soo2` / `kiann1`) on the same bytes — see
/// [`derive_notone`] for why the display glyphs cannot reach a key as-is. Both
/// sides run through this one function, so they cannot drift.
// num 形鍵的共同整形 — 小寫、取 base form (o͘→oo、ⁿ→nn)、去連字號/空白。
//   寫入端來自顯示形 roman (so͘2/kiaⁿ1)、查詢端來自鍵盤 raw buffer (soo2/kiann1),
//   兩邊都走這支所以不會漂移。
fn fuse_latin_numeric(numeric: &str) -> String {
    crate::taigi_unicode_base_form(&numeric.to_lowercase())
        .chars()
        .filter(|c| *c != '-' && *c != ' ')
        .collect()
}

/// Strip a raw TPS Bopomofo input to its fused search key: always drop hyphen +
/// whitespace; drop TPS tone marks too when `keep_tone_marks` is false (the
/// toneless `notone` form).
///
/// Tone-8 normalization: platform keyboards type the standalone modifier-letter
/// dot `U+02D9`, but the stored `tps:num` key uses the combining form `U+0307`
/// (`tps_num_from_tl` → `to_zhuyin(encode_safe=false)`). Canonicalize `U+02D9 →
/// U+0307` BEFORE classification so the kept (`num`) form matches the stored key
/// and the dropped (`notone`) form still strips it — mirrors
/// `lexicon::key_normalizer::normalize_tps_key_body`.
fn strip_tps_input(input: &str, keep_tone_marks: bool) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        if ch == '-' || ch.is_whitespace() {
            continue;
        }
        let normalized = normalize_tps_tone8_scalar(ch);
        if !keep_tone_marks && is_tps_tone_mark(normalized) {
            continue;
        }
        out.push(normalized);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Convenience: does the bundle contain a (family, form, key) row?
    fn has(keys: &[CustomSearchKey], family: &str, form: &str, key: &str) -> bool {
        keys.iter()
            .any(|k| k.family == family && k.form == form && k.key == key)
    }

    /// A query key counts as "findable" against a stored bundle when some
    /// stored row in the same family has the query's primary form OR `abbrev`
    /// and an equal key (the platform SQL matches `form IN (primary, abbrev)`).
    fn query_hits_stored(stored: &[CustomSearchKey], q: &CustomSearchKey) -> bool {
        stored.iter().any(|k| {
            k.family == q.family && (k.form == q.form || k.form == FORM_ABBREV) && k.key == q.key
        })
    }

    // trace: roman="chiah" (POJ spelling of 食/tsia̍h, toneless). canonical TL =
    // poj_display_to_tl_display("chiah") = "tsiah"; poj = "chiah".
    #[test]
    fn poj_stored_is_findable_via_tl_and_poj() {
        let stored = derive_custom_search_keys("chiah");
        assert!(has(&stored, FAMILY_TL, FORM_NOTONE, "tsiah"));
        assert!(has(&stored, FAMILY_POJ, FORM_NOTONE, "chiah"));

        let q_tl = derive_custom_query_key("tsiah", "tl").unwrap();
        assert!(
            query_hits_stored(&stored, &q_tl),
            "TL query must find POJ-stored entry"
        );
        let q_poj = derive_custom_query_key("chiah", "poj").unwrap();
        assert!(
            query_hits_stored(&stored, &q_poj),
            "POJ query must find POJ-stored entry"
        );
    }

    // trace: roman="tsiah" (TL). canonical TL = "tsiah"; poj = "chiah".
    #[test]
    fn tl_stored_is_findable_via_poj() {
        let stored = derive_custom_search_keys("tsiah");
        let q_poj = derive_custom_query_key("chiah", "poj").unwrap();
        assert!(
            query_hits_stored(&stored, &q_poj),
            "POJ query must find TL-stored entry"
        );
    }

    // The Bopomofo a TPS user types for 食 IS tps_notone_from_tl("tsiah").
    // Storing the latin form must make that Bopomofo query hit, and the query
    // must classify as the tps family from the raw input (not the settings).
    #[test]
    fn tps_bopomofo_query_finds_latin_stored() {
        let stored = derive_custom_search_keys("tsiah");
        let bopomofo = tps_notone_from_tl("tsiah");
        assert!(!bopomofo.is_empty());
        // input_mode "tl" on purpose — effective family must upgrade to tps
        // because the raw input contains Bopomofo.
        let q = derive_custom_query_key(&bopomofo, "tl").unwrap();
        assert_eq!(q.family, FAMILY_TPS);
        assert_eq!(q.form, FORM_NOTONE);
        assert!(
            query_hits_stored(&stored, &q),
            "TPS query must find latin-stored entry"
        );
    }

    // trace: roman="chia̍h" (POJ display, tone 8). tl num = to_tone_number(
    // "tsia̍h") fused = "tsiah8"; user typing TL numeric "tsiah8" hits it.
    #[test]
    fn tone_aware_cross_mode_num() {
        let stored = derive_custom_search_keys("chia̍h");
        assert!(has(&stored, FAMILY_TL, FORM_NUM, "tsiah8"));
        let q = derive_custom_query_key("tsiah8", "tl").unwrap();
        assert_eq!(q.form, FORM_NUM);
        assert!(query_hits_stored(&stored, &q));
    }

    // trace: roman="guá-sī" (我是). tl abbrev = "gs"; user typing "gs" toneless
    // produces (tl, notone, "gs") which the platform matches against the
    // stored abbrev row.
    #[test]
    fn abbrev_cross_form_match() {
        let stored = derive_custom_search_keys("guá-sī");
        assert!(has(&stored, FAMILY_TL, FORM_ABBREV, "gs"));
        let q = derive_custom_query_key("gs", "tl").unwrap();
        assert!(
            query_hits_stored(&stored, &q),
            "abbrev row must satisfy a notone-form query"
        );
    }

    // 食 (chia̍h / tsia̍h, tone 8): the stored tps:num key uses the combining
    // dot U+0307; a TPS keyboard types the standalone modifier dot U+02D9. The
    // query must normalize U+02D9 → U+0307 so the tone-aware TPS lookup hits.
    #[test]
    fn tps_tone8_query_normalizes_modifier_dot_to_combining() {
        let stored = derive_custom_search_keys("chia̍h");
        let stored_num = stored
            .iter()
            .find(|k| k.family == FAMILY_TPS && k.form == FORM_NUM)
            .expect("tps num key present");
        assert!(
            stored_num.key.contains('\u{0307}'),
            "stored tps:num uses combining U+0307"
        );
        // Simulate the keyboard tone-8 form (standalone dot U+02D9).
        let keyboard_input = stored_num.key.replace('\u{0307}', "\u{02d9}");
        assert!(keyboard_input.contains('\u{02d9}'));
        let q = derive_custom_query_key(&keyboard_input, "tl").unwrap();
        assert_eq!(q.family, FAMILY_TPS);
        assert_eq!(q.form, FORM_NUM, "tone mark present → num form");
        assert_eq!(q.key, stored_num.key, "U+02D9 normalized back to U+0307");
        assert!(query_hits_stored(&stored, &q));
    }

    // 2026-08-28 user report (backlog B1): custom entry `băng-só͘-khó͘` (絆創膏).
    // POJ writes the vowel /ɔ/ as `o` + U+0358; the raw keyboard buffer holds
    // the ASCII `oo` the user typed (the oo double-tap rewrite is display-only).
    // trace: stored poj display = "băng-só͘-khó͘"; derive_notone lowercases,
    // folds U+0358 → "o", NFD-strips the breve + acutes → "bangsookhoo" — the
    // same bytes the POJ query "bangsookhoo" produces. Pre-fix the stored key
    // was "bangsokho", so the entry died at the SECOND `o` under the platform
    // prefix match.
    #[test]
    fn poj_oo_dot_entry_is_findable_by_ascii_oo_query() {
        let stored = derive_custom_search_keys("băng-só͘-khó͘");
        assert!(has(&stored, FAMILY_POJ, FORM_NOTONE, "bangsookhoo"));
        assert!(has(&stored, FAMILY_TL, FORM_NOTONE, "bangsookhoo"));

        let q_poj = derive_custom_query_key("bangsookhoo", "poj").unwrap();
        assert!(
            query_hits_stored(&stored, &q_poj),
            "POJ ASCII `oo` query must find an `o͘`-stored entry"
        );
        // TL control — this path was never broken.
        let q_tl = derive_custom_query_key("bangsookhoo", "tl").unwrap();
        assert!(query_hits_stored(&stored, &q_tl));
    }

    // Same fold in the tone-aware branch: the stored `poj:num` key is built
    // from the display form (`bang9so͘2kho͘2`), the query from the raw buffer
    // (`bang9soo2khoo2`).
    #[test]
    fn poj_oo_dot_entry_is_findable_by_numeric_ascii_query() {
        let stored = derive_custom_search_keys("băng-só͘-khó͘");
        assert!(has(&stored, FAMILY_POJ, FORM_NUM, "bang9soo2khoo2"));
        let q = derive_custom_query_key("bang9soo2khoo2", "poj").unwrap();
        assert_eq!(q.form, FORM_NUM);
        assert!(query_hits_stored(&stored, &q));
    }

    // Nasal sibling of the same defect: POJ display `kiaⁿ` → `to_tone_number`
    // keeps ⁿ (`kiaⁿ1`), the keyboard buffer holds `kiann1`. `derive_notone`
    // already folded ⁿ→nn, so only the numeric form was broken.
    #[test]
    fn poj_nasal_entry_is_findable_by_numeric_ascii_query() {
        let stored = derive_custom_search_keys("kiaⁿ");
        assert!(has(&stored, FAMILY_POJ, FORM_NUM, "kiann1"));
        let q = derive_custom_query_key("kiann1", "poj").unwrap();
        assert!(query_hits_stored(&stored, &q));
    }

    // `o͘` must NOT collapse onto a bare `o`: 芋 `ō͘` and 蚵 `ô` are different
    // words, and the pre-fix key builder made both `o`.
    #[test]
    fn oo_dot_does_not_collapse_onto_bare_o() {
        let dotted = derive_custom_search_keys("ō͘");
        let bare = derive_custom_search_keys("ô");
        assert!(has(&dotted, FAMILY_POJ, FORM_NOTONE, "oo"));
        assert!(has(&bare, FAMILY_POJ, FORM_NOTONE, "o"));
        let q_bare = derive_custom_query_key("o", "poj").unwrap();
        assert!(
            !query_hits_stored(&dotted, &q_bare),
            "a bare `o` query must not exact-match the `o͘` entry's key"
        );
    }

    // The dot is a scalar of its own in both NFC and NFD, and a tone mark may
    // sit either side of it depending on how the text was produced. All four
    // shapes are the same word and must key the same.
    #[test]
    fn oo_dot_folds_under_every_composition_and_mark_order() {
        let expected = derive_custom_query_key("soo", "poj").unwrap().key;
        for spelling in [
            "so\u{0358}",         // toneless
            "s\u{00f3}\u{0358}",  // NFC `ó` + dot
            "so\u{0301}\u{0358}", // NFD, tone before dot
            "so\u{0358}\u{0301}", // NFD, dot before tone
            "SO\u{0358}",         // uppercase base
        ] {
            let q = derive_custom_query_key(spelling, "poj").unwrap();
            assert_eq!(q.key, expected, "spelling {spelling:?} must key as `soo`");
        }
    }

    // The nasal marker has an uppercase-modifier twin (ᴺ U+1D3A) a stored
    // roman can carry; both fold to `nn` in either form.
    #[test]
    fn nasal_marker_folds_in_both_glyphs_and_cases() {
        let expected = derive_custom_query_key("kiann1", "poj").unwrap().key;
        for spelling in ["kia\u{207f}1", "kia\u{1d3a}1", "KIA\u{207f}1"] {
            let q = derive_custom_query_key(spelling, "poj").unwrap();
            assert_eq!(
                q.key, expected,
                "spelling {spelling:?} must key as `kiann1`"
            );
        }
    }

    #[test]
    fn empty_and_residue_yield_nothing() {
        assert!(derive_custom_search_keys("").is_empty());
        assert!(derive_custom_search_keys("   ").is_empty());
        assert!(derive_custom_query_key("", "tl").is_none());
        assert!(derive_custom_query_key("   ", "tl").is_none());
    }

    #[test]
    fn bundle_has_no_duplicate_rows() {
        let stored = derive_custom_search_keys("tsiah");
        let mut seen = HashSet::new();
        for k in &stored {
            assert!(
                seen.insert((k.family, k.form, k.key.as_str())),
                "duplicate row {}:{}:{}",
                k.family,
                k.form,
                k.key
            );
        }
    }
}
