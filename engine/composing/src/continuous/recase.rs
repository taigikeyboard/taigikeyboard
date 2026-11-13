//! Candidate roman casing: the typed segment's case on a fetched roman,
//! and the POJ display rewrite of step 5.

use lexicon::RawCandidate;

/// v3.5.8 — derive the keyboard `LetterCase` intent from a user raw
/// input segment so a continuous candidate's roman mirrors the case the
/// user actually typed for *that* span (`Hittui` → segment `Hit`
/// Uppercased, segment `tui` Lowercased). Only alphabetic chars count
/// (tone digits / hyphens ignored); no alpha → `Lowercased`. This is
/// the engine-owned continuous casing of Codex pre-impl 2A locus =
/// candidate construction — the single source of candidate casing (the
/// platform `SuggestionCaseTransformer`s it bypassed were removed 2026-10-04).
fn raw_segment_letter_case(raw_seg: &str) -> phonetics::case_transform::LetterCase {
    use phonetics::case_transform::LetterCase;
    let mut alphas = raw_seg.chars().filter(|c| c.is_alphabetic());
    let Some(first) = alphas.next() else {
        return LetterCase::Lowercased;
    };
    let rest_all_upper = alphas.all(|c| c.is_uppercase());
    if first.is_uppercase() && rest_all_upper {
        LetterCase::CapsLocked
    } else if first.is_uppercase() {
        LetterCase::Uppercased
    } else {
        LetterCase::Lowercased
    }
}

/// Apply [`raw_segment_letter_case`] of `raw_seg` to `roman` via the
/// tone-letter-aware, raise-only `phonetics::case_transform::raise_case`.
/// The raw span only derives the case intent, never sliced against the
/// roman, so a toneless-ASCII raw vs tone-diacritic roman length mismatch
/// is a non-issue (Codex pre-impl 2026-05-18).
pub(super) fn recase_roman(roman: &str, raw_seg: &str, mode: phonetics::InputMode) -> String {
    phonetics::case_transform::raise_case(roman, raw_segment_letter_case(raw_seg), mode)
}

/// [`recase_roman`] over a whole batch that shares one raw segment. The
/// case intent is derived once; a lowercase segment leaves every roman
/// untouched, so the common path allocates nothing.
pub(super) fn recase_all(
    candidates: &mut [RawCandidate],
    raw_seg: &str,
    mode: phonetics::InputMode,
) {
    let case = raw_segment_letter_case(raw_seg);
    if case == phonetics::case_transform::LetterCase::Lowercased {
        return;
    }
    for candidate in candidates {
        candidate.roman = phonetics::case_transform::raise_case(&candidate.roman, case, mode);
    }
}

/// v3.5.9 C-4 (renamed from `render_roman_for_mode`) — POJ display rewrite
/// for a continuous candidate's presentation `roman`: `oo`→`o͘`, `nn`→`ⁿ`,
/// `ua`→`oa`, … via [`phonetics::api::tl_display_to_poj_display`], plus a
/// POJ-aware case restore. Not mode routing. Not the TL↔POJ canonical-form
/// pipeline; canonical commit keys stay under
/// [`phonetics::api::canonical_tl_form`]. This helper is step 5 of
/// [`assemble_candidates`] and runs only when the caller is in POJ mode
/// (gate at `assemble_candidates`); the signature is POJ-only so the
/// gate cannot drift to silent identity-on-misuse.
///
/// The TL→POJ rewriter only title-cases each `-` / space token (it checks
/// `first.is_uppercase()` then stops), so the letters after the first are
/// re-raised here per token with `match_case` — the same raise-only,
/// letter-by-letter rule `convert_syllable` uses for the preedit. Per
/// token, never whole-string: the rewriter can change a token's letter
/// count (`nn` → `ⁿ`, `oo` → `o͘`), so the alignment restarts at every
/// separator; and a whole-string `LetterCase` read the POJ nasal marker
/// (`ⁿ`, alphabetic and lowercase) as "not all caps", collapsing a Caps
/// Lock custom `KENG-LÂM SU-Īⁿ` to `Keng-Lâm Su-Īⁿ` and a mixed `KENG-lâm`
/// to `Keng-lâm` (retro Codex review of #89, 2026-09-22). Presentation
/// only — never feed `display_text` (the canonical commit /
/// `user_frequency.db` key) here.
///
/// [`assemble_candidates`]: super::assemble::assemble_candidates
pub(super) fn recase_tl_as_poj_display(roman: &str) -> String {
    let poj = phonetics::api::tl_display_to_poj_display(roman);
    let mut out = String::with_capacity(poj.len());
    // The rewriter keeps every `-` / space, so both strings split into the
    // same tokens; `split_inclusive` carries the separator (a non-letter,
    // passed through by `match_case`) at the end of each piece.
    for (source, target) in roman
        .split_inclusive(['-', ' '])
        .zip(poj.split_inclusive(['-', ' ']))
    {
        out.push_str(&phonetics::case_transform::match_case(
            target,
            source,
            phonetics::InputMode::Poj,
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use lexicon::dictionary_reader::NO_ROWID;
    use lexicon::COVERAGE_KIND_FULL;
    use ranking::WALKER_COST_UNPRICED;

    // ----- v3.5.8 2A — per-segment case-from-raw -----
    #[test]
    fn raw_segment_letter_case_maps_typed_intent() {
        use phonetics::case_transform::LetterCase;
        // Titlecase raw (the `Hittui → Hit` segment): first alpha upper,
        // rest not all upper → Uppercased (capitalize-first).
        assert_eq!(raw_segment_letter_case("Hit"), LetterCase::Uppercased);
        // Lowercase raw (the `tui` segment) → Lowercased.
        assert_eq!(raw_segment_letter_case("tui"), LetterCase::Lowercased);
        // All-caps → CapsLocked (full upper).
        assert_eq!(raw_segment_letter_case("HIT"), LetterCase::CapsLocked);
        // Tone digits / hyphens are not alphabetic → ignored.
        assert_eq!(raw_segment_letter_case("Tai5"), LetterCase::Uppercased);
        assert_eq!(raw_segment_letter_case("tai-uan"), LetterCase::Lowercased);
        // No alphabetic char → defaults to Lowercased (never panics).
        assert_eq!(raw_segment_letter_case("123"), LetterCase::Lowercased);
        assert_eq!(raw_segment_letter_case(""), LetterCase::Lowercased);
    }

    // ----- v3.5.8 — POJ-display render of the continuous candidate roman -----

    #[test]
    fn recase_tl_as_poj_display_rewrites_oo_and_nn() {
        // The reported bug: in POJ mode the continuous best candidate
        // must show `oo`→`o͘` (o + U+0358) and `nn`→`ⁿ` (U+207F),
        // not raw TL.
        assert_eq!(recase_tl_as_poj_display("oo"), "o\u{0358}");
        assert_eq!(recase_tl_as_poj_display("goo"), "go\u{0358}");
        assert_eq!(recase_tl_as_poj_display("sann"), "sa\u{207f}");
        // Whole-sentence walker path: space-joined multi-syllable roman
        // (`tl_display_to_poj_display` splits on ` `/`-` per token).
        assert_eq!(
            recase_tl_as_poj_display("goo sann"),
            "go\u{0358} sa\u{207f}"
        );
        assert_eq!(recase_tl_as_poj_display("tai-oo"), "tai-o\u{0358}");
        // Tone-marked TL → POJ: the tone sits between `o` and the
        // U+0358 dot (`kòo` 顧 → `kò͘`). Exact, not just "contains".
        assert_eq!(recase_tl_as_poj_display("kòo"), "k\u{f2}\u{0358}");
    }

    #[test]
    fn recase_tl_as_poj_display_preserves_letter_case() {
        // Lowercase (normal typing) stays lowercase.
        assert_eq!(recase_tl_as_poj_display("goo"), "go\u{0358}");
        // Sentence-start capital (the `Hittui → Hit` segment class).
        assert_eq!(recase_tl_as_poj_display("Goo"), "Go\u{0358}");
        // CapsLock — the Codex pre-impl BLOCK: `tl_display_to_poj_display`
        // only title-cases, so without the per-token re-raise an
        // all-caps candidate would collapse to `Go͘`. Pin
        // the all-caps form survives.
        assert_eq!(recase_tl_as_poj_display("OO"), "O\u{0358}");
        // `nn` under CapsLock: the base letters go all-caps and the POJ
        // nasal marker follows its letter (`match_case` writes `ᴺ` after a
        // capital, as the preedit's `convert_syllable` does); the §53
        // switch pass later in step 5 owns the marker's final case.
        assert_eq!(recase_tl_as_poj_display("SANN"), "SA\u{1d3a}");
    }

    #[test]
    fn poj_render_rewrites_roman_but_preserves_canonical_tl() {
        // R2 (Codex pre-impl 2026-06-03 SHOULD): the POJ presentation pass
        // (`assemble_candidates` Step 5) rewrites the DISPLAY `roman`
        // TL→POJ, but the identity sidechannel `canonical_tl` must stay
        // canonical TL so the NextWord association learns the same TL a
        // normal candidate commit records. Mirror the exact mutation the
        // production loop applies (`cand.roman = recase_tl_as_poj_display`).
        let mut cand = RawCandidate {
            rowid: NO_ROWID,
            consumed_span: (0, 6),
            syllable_count: 1,
            display_text: "鵝".to_string(),
            roman: "goo".to_string(),
            hanji: Some("鵝".to_string()),
            canonical_tl: "goo".to_string(),
            score: 0.0,
            frequency: 0,
            walker_cost: WALKER_COST_UNPRICED,
            bitmask: 0,
            user_weight: 0.0,
            context_rank: ranking::CONTEXT_RANK_NONE,
            coverage_kind: COVERAGE_KIND_FULL,
            is_custom: false,
        };
        cand.roman = recase_tl_as_poj_display(&cand.roman);
        assert_eq!(cand.roman, "go\u{0358}", "display roman renders to POJ");
        assert_eq!(
            cand.canonical_tl, "goo",
            "identity sidechannel stays canonical TL through POJ render"
        );
    }

    #[test]
    fn recase_roman_mirrors_raw_segment_case_tone_aware() {
        let tl = phonetics::InputMode::Tl;
        // The motivating bug: dict roman is canonical lowercase; the
        // user's raw for each span drives the displayed case so a
        // continuous segment reads `Hit` / `tui`, not `Tui`.
        assert_eq!(recase_roman("hit", "Hit", tl), "Hit");
        assert_eq!(recase_roman("tui", "tui", tl), "tui");
        // Tone-diacritic roman, toneless ASCII raw (length mismatch is a
        // non-issue — raw only derives the case intent).
        assert_eq!(recase_roman("tâi", "Tai", tl), "Tâi");
        assert_eq!(recase_roman("tâi", "tai", tl), "tâi");
    }

    #[test]
    fn recase_roman_keeps_custom_entry_capitals() {
        // User report 2026-09-19: custom entry `Keng-lâm Su-īⁿ` typed as
        // `klsi` came out `keng-lâm su-īⁿ`, and `Klsi` as `Keng-lâm su-īⁿ`.
        // Raw case only raises; the stored capitals stay.
        let poj = phonetics::InputMode::Poj;
        assert_eq!(
            recase_roman("Keng-lâm Su-īⁿ", "Klsi", poj),
            "Keng-lâm Su-īⁿ"
        );
        // POJ display round-trip must not lower the stored `Su` either.
        assert_eq!(recase_tl_as_poj_display("Keng-lâm Su-īⁿ"), "Keng-lâm Su-īⁿ");
    }

    #[test]
    fn recase_tl_as_poj_display_keeps_caps_lock_and_mixed_case_per_token() {
        // Retro Codex review of #89 (2026-09-22): the stored `ⁿ` is an
        // alphabetic lowercase char, so a whole-string case read called the
        // Caps Lock custom entry "title case" and lowered every token.
        // trace: rewriter → "Keng-Lâm Su-Īⁿ", per-token match_case against
        // "KENG-LÂM SU-Īⁿ" re-raises every letter; the stored `ⁿ` is a
        // lowercase source letter, so it stays as stored (raise-only) and
        // the §53 switch pass later in step 5 writes `ᴺ` after the capital.
        assert_eq!(recase_tl_as_poj_display("KENG-LÂM SU-Īⁿ"), "KENG-LÂM SU-Īⁿ");
        // Mixed case inside one entry: each token keeps its own capitals.
        assert_eq!(recase_tl_as_poj_display("KENG-lâm"), "KENG-lâm");
        // A TL row whose token shrinks (`NN` → `ⁿ`) still aligns per token.
        assert_eq!(
            recase_tl_as_poj_display("KENG-LÂM SU-ĪNN"),
            "KENG-LÂM SU-Īᴺ"
        );
        // Negative control: lowercase stays lowercase.
        assert_eq!(
            recase_tl_as_poj_display("keng-lâm su-īnn"),
            "keng-lâm su-īⁿ"
        );
    }
}
