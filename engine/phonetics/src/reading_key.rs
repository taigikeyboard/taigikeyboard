//! Reading identity keys: the mode a stored roman is read in and the
//! separator- / tone-insensitive reading key.

use crate::api::{contains_tps, InputMode};

/// The mode a stored custom-dictionary `roman` is read in when the user types
/// in `mode`: under TPS a TL / POJ roman (no Bopomofo) is read as TL, so its
/// walker key and its `(hanji, canonical TL)` identity agree with TL mode's;
/// otherwise `mode` itself. The one rule both derive from.
pub fn stored_roman_mode(roman: &str, mode: InputMode) -> InputMode {
    match mode {
        InputMode::Tps if !contains_tps(roman) => InputMode::Tl,
        mode => mode,
    }
}

/// Separator- and tone-insensitive reading key for collapsing romanization
/// variants of the **same syllable sequence** — `taigi`, `tai5gi2`, and
/// `tâi-gí` all map to `taigi`. Lowercases, folds the nasal marker
/// (ⁿ U+207F / ᴺ U+1D3A → `nn`), then drops every tone diacritic, ASCII
/// digit, hyphen, and space.
///
/// Reuses [`derivation::derive_notone`] (the custom-dictionary `notone`
/// derivation) verbatim so the two never drift — both want exactly
/// "strip everything that is not a base spelling letter". Used by
/// `nextword::filter` to recognize that a continuous-input raw next_tl
/// (`taigi`) and a normal-commit canonical next_tl (`tâi-gí`) are the
/// same reading and must not surface as two predictions of one word.
///
/// **Tone-INSENSITIVE** — unlike [`roman_reading_eq`] (which is
/// separator-insensitive but tone-PRESERVING), this folds tones away so
/// `tāng` (重量) and `tàng` both key to `tang`; the caller is responsible
/// for keeping genuinely distinct readings (Core Principle #7) apart by
/// only collapsing when a single canonical target exists.
///
/// **Residual** — folds ENCODING glyphs, not spelling families. The POJ
/// nasal marker `ⁿ` and the `o͘` dot U+0358 are alternate encodings of
/// `nn` / `oo` and DO fold (via `taigi_unicode_base_form`), but a genuine
/// spelling difference does not: POJ `chiah` keys to `chiah`, TL `tsiah`
/// to `tsiah` (no `ch↔ts` / `oa↔ua`). A POJ-spelled raw next_tl therefore
/// will not collapse onto its TL canonical here; the write-side R2 fix
/// (continuous commit carries canonical TL) closes that gap. R1 only needs
/// the common same-family case.
pub fn toneless_reading_key(roman: &str) -> String {
    crate::derivation::derive_notone(roman)
}

/// Two romanizations are the **same reading** when they differ only in
/// syllable separators: ASCII space (walker synth multi-word join), `-`
/// (hyphen compound), or `--` (neutral-tone khinsiann, two `-` chars). Tone
/// diacritics live inside the syllable letters and are NOT stripped, so
/// the comparison is tone-preserving:
///
/// * `roman_reading_eq("hōo guá", "hōo--guá")` → `true` (予我: synth
///   space-join vs dict khinsiann form — the same reading).
/// * `roman_reading_eq("hōo guá", "hōo-guā")` → `false` (戶外: guā tone 7
///   ≠ guá tone 2 — a different word, Core Principle #6).
///
/// Callers: the composing slot-0 promote and learned-pair fold
/// (`composing::continuous`, `composing::requests`) and the nailed-join
/// compound oracle (`lexicon::compound_hanji_exists`).
pub fn roman_reading_eq(a: &str, b: &str) -> bool {
    fn is_kept(c: &char) -> bool {
        *c != ' ' && *c != '-'
    }
    // Allocation-free: compare the separator-filtered char streams
    // directly (`Iterator::eq`). This runs per candidate in the per-
    // keystroke slot-0 scan, so it must not heap-allocate a stripped
    // `String` for each comparison.
    a.chars().filter(is_kept).eq(b.chars().filter(is_kept))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roman_reading_eq_matches_separator_variants_preserves_tone() {
        // 予我: walker space-join vs dict khinsiann — same reading.
        assert!(roman_reading_eq("hōo guá", "hōo--guá"));
        // Hyphen vs space — same reading.
        assert!(roman_reading_eq("tâi gí", "tâi-gí"));
        // 戶外: tone 7 `guā` ≠ tone 2 `guá` — different reading despite
        // sharing the separator-stripped consonant/vowel skeleton.
        assert!(!roman_reading_eq("hōo guá", "hōo-guā"));
        // Identical strings (also-是 `iā sī` synth == dict) trivially eq.
        assert!(roman_reading_eq("iā sī", "iā sī"));
        // Different syllables entirely — not a reading match.
        assert!(!roman_reading_eq("hōo guá", "tâi gí"));
        // The nailed-join oracle's shape: per-syllable picks concatenated
        // without a separator against the dictionary row.
        assert!(roman_reading_eq("ánniâ", "án-niâ"));
        assert!(!roman_reading_eq("ánniâ", "án-nia"));
    }

    // trace: derive_notone lowercases, folds ⁿ→nn, NFD, drops combining
    // tone marks + ASCII digits + '-' + ' '. So all three renderings of
    // 台語 collapse to the bare spelling "taigi".
    #[test]
    fn toneless_reading_key_folds_tone_and_separator_variants() {
        assert_eq!(toneless_reading_key("tâi-gí"), "taigi");
        assert_eq!(toneless_reading_key("taigi"), "taigi");
        assert_eq!(toneless_reading_key("tai5gi2"), "taigi");
        // canonical (diacritic + hyphen) and continuous raw (fused) agree.
        assert_eq!(
            toneless_reading_key("tâi-gí"),
            toneless_reading_key("taigi")
        );
    }

    // trace: all-tone-1 multi-syllable canonical "khong-an" vs continuous
    // raw "khongan" — separator-only difference, both toneless — still
    // collapse to one key. (The caller keeps the separator-bearing form as
    // canonical.)
    #[test]
    fn toneless_reading_key_collapses_separator_only_difference() {
        assert_eq!(toneless_reading_key("khong-an"), "khongan");
        assert_eq!(
            toneless_reading_key("khong-an"),
            toneless_reading_key("khongan")
        );
    }

    // trace: 重/tāng (重量) and 重/tàng are genuinely distinct readings
    // (Core Principle #7) but tone-INSENSITIVE folding maps both to "tang".
    // The key intentionally collides; the nextword caller must NOT collapse
    // such a bare single-syllable group (no separator → no canonical target).
    #[test]
    fn toneless_reading_key_is_tone_insensitive_distinct_readings_collide() {
        assert_eq!(toneless_reading_key("tāng"), "tang");
        assert_eq!(toneless_reading_key("tàng"), "tang");
    }

    // trace: spelling families are NOT folded (no ch↔ts). Documented R1
    // residual — POJ-spelled raw will not collapse onto TL canonical here.
    #[test]
    fn toneless_reading_key_does_not_fold_spelling_families() {
        assert_ne!(toneless_reading_key("chiah"), toneless_reading_key("tsiah"));
    }

    // trace: an ENCODING difference is not a spelling family — POJ writes the
    // same vowel/nasal as TL with a combining dot / modifier letter, so both
    // renderings are one reading and must key alike.
    #[test]
    fn toneless_reading_key_folds_poj_encoding_glyphs() {
        assert_eq!(
            toneless_reading_key("ó\u{0358}"),
            toneless_reading_key("oo")
        );
        assert_eq!(toneless_reading_key("kiaⁿ"), toneless_reading_key("kiann"));
    }
}
