//! Compute the display form for a raw composition buffer.
//!
//! TPS inputs are already display-ready (return as-is). POJ/TL inputs go
//! through the full `phonetics::api::normalize_tone` chain (POJ doubletap
//! preprocessing → tone-mark application → nasal-marker case adjustment) per
//! plan §3.2a. The platform `RustEngineBridge.normalizeTone` call sites are
//! replaced by this in-process call.
//!
//! This is the rendering primitive behind [`crate::api::Phase::raw_input`] —
//! see `docs/engine/continuous-input-ranking.md` §10.2 / §10.3 clarification β
//! for the `rawInput` contract. User-typed hyphens are preserved as conversion
//! boundaries (the tone-mark chain splits on `-`); the engine does NOT validate
//! whether each chunk is a real syllable, and does not insert hyphens on its
//! own. Engine-side syllabifier-driven auto-hyphenation is out of scope for
//! v3.5.8 Item 2 (see §10.2 amendment 2026-05-13).

use protos::engine::AppConfig;

pub(crate) fn derived_display(raw: &str, config: &AppConfig) -> String {
    if raw.is_empty() {
        return String::new();
    }
    if phonetics::api::contains_tps(raw) {
        return strip_tps_separator_markers(raw);
    }
    phonetics::api::normalize_tone(raw, config)
}

/// §41 — drop the TPS separator markers from `raw`.
///
/// The keyboard's ASCII space in a TPS buffer is the tone-1 /
/// syllable-boundary MARKER, not text the user typed: it is appended so the
/// next dual-form consonant stays an onset (`phonetics::tps_adjust`), the
/// shadow records it as a lattice barrier
/// (`shadow::build_separator_shadow`), and §41 reads that barrier to pin the
/// unmarked tone. It therefore stays in the raw buffer and must be absent
/// from everything the raw buffer FEEDS the outside world: the pre-edit, the
/// text a commit writes, and the romanization handed to NextWord. Every TPS
/// glyph, tone mark included, passes through untouched; only `U+0020` goes,
/// and only for a buffer that actually carries TPS (a TL / POJ space is a
/// literal word boundary and survives).
pub(crate) fn strip_tps_separator_markers(raw: &str) -> String {
    if phonetics::api::contains_tps(raw) {
        return raw.replace(' ', "");
    }
    raw.to_string()
}

/// Where a caret sitting at raw byte offset `caret` lands inside `display`
/// (the [`derived_display`] of the same `raw`), as a UTF-16 offset — the unit
/// both desktop hosts take (`NSRange` for `setMarkedText`, `cch` for
/// `ITfRange::ShiftEnd`).
///
/// Deriving the display of `raw[..caret]` alone and taking its length is NOT
/// this: `ng|5` renders `n̂g`, whose prefix `ng` is two code units but whose
/// caret belongs after the `g` at three; `ka2|i` stays literal `ka2i`, whose
/// prefix `ká` is two units while the caret sits after the `2` at three. So
/// the raw and display strings are walked in lockstep, relying on what the
/// derivation chain does to a character and nothing else:
///
/// - it inserts combining marks (tone marks, the POJ dot) after a letter —
///   a matched letter consumes the nonzero-CCC marks that follow it, so the
///   caret never lands between a letter and its marks;
/// - it drops a raw character: the trailing tone digit and a TPS separator
///   marker have no display counterpart, so a raw character that does not
///   match the next display letter is one the chain dropped and maps right
///   after the display character before it;
/// - it folds a POJ double tap, `oo` → `o\u{0358}` / `nn` → `ⁿ` (`ᴺ` after a
///   capital vowel). Both taps
///   are the same letter, so the second one cannot be told from a later
///   `o` / `n` by matching alone (`hoo|on` would land after the third `o`);
///   the fold is recognised by its signature instead — the matched `o` carries
///   the dot, or the matched letter IS a nasal marker — together with the next raw
///   character being that same letter, and the second tap is consumed there;
/// - it changes case (`taI2` → `tái`, `Tai5` → `Tâi`) — compared
///   case-insensitively.
///
/// Folded positions (`ho|o`, `tin|n`, the digit) share a display offset with
/// their neighbour; the caret takes no visible step there. `caret` past the
/// end clamps to the end; off a char boundary it lands after the character
/// holding that byte.
///
/// The "never adds a base letter" premise is pinned where it can break, in
/// `phonetics::api` (`normalize_tone_never_adds_a_base_letter`).
pub(crate) fn display_caret_utf16(raw: &str, display: &str, caret: usize) -> usize {
    if caret >= raw.len() {
        return display.encode_utf16().count();
    }
    let mut display_chars = display.chars().peekable();
    let mut display_utf16 = 0;
    let mut raw_chars = raw.char_indices().peekable();
    while let Some((raw_offset, raw_char)) = raw_chars.next() {
        if raw_offset >= caret {
            break;
        }
        display_utf16 += absorb_marks(&mut display_chars).utf16_len;
        let Some(&display_char) = display_chars.peek() else {
            continue;
        };
        if !same_base_scalar(raw_char, display_char) {
            continue;
        }
        display_chars.next();
        display_utf16 += display_char.len_utf16();
        let marks = absorb_marks(&mut display_chars);
        display_utf16 += marks.utf16_len;

        let is_fold_signature = (marks.has_poj_dot && raw_char.eq_ignore_ascii_case(&'o'))
            || (NASAL_MARKERS.contains(&display_char) && raw_char.eq_ignore_ascii_case(&'n'));
        let next_is_second_tap = raw_chars
            .peek()
            .is_some_and(|&(_, next)| next.eq_ignore_ascii_case(&raw_char));
        if is_fold_signature && next_is_second_tap {
            let (second_tap_offset, _) = raw_chars.next().unwrap();
            if second_tap_offset >= caret {
                break;
            }
        }
    }
    display_utf16
}

/// `ⁿ` and its capital `ᴺ` (what `adjust_nasal_marker_case` writes after an
/// upper-case vowel) — the display side of a folded `nn`.
const NASAL_MARKERS: [char; 2] = ['\u{207f}', '\u{1d3a}'];

struct AbsorbedMarks {
    utf16_len: usize,
    has_poj_dot: bool,
}

/// Consumes the run of combining marks (nonzero canonical combining class)
/// at the front of `chars`.
fn absorb_marks(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> AbsorbedMarks {
    let mut absorbed = AbsorbedMarks {
        utf16_len: 0,
        has_poj_dot: false,
    };
    while let Some(&c) = chars.peek() {
        if unicode_normalization::char::canonical_combining_class(c) == 0 {
            break;
        }
        absorbed.utf16_len += c.len_utf16();
        absorbed.has_poj_dot |= c == '\u{0358}';
        chars.next();
    }
    absorbed
}

/// `raw_char` and `display_char` share their first compatibility-decomposed
/// scalar, case aside: `a` ↔ `â`, `I` ↔ `i`, `n` ↔ `ⁿ`.
fn same_base_scalar(raw_char: char, display_char: char) -> bool {
    use unicode_normalization::UnicodeNormalization;
    let raw_base = raw_char.nfkd().next().unwrap_or(raw_char);
    let display_base = display_char.nfkd().next().unwrap_or(display_char);
    raw_base.to_lowercase().eq(display_base.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{NailedSegment, Phase};

    fn config_tl() -> AppConfig {
        AppConfig {
            tone_mode: String::new(),
            input_mode: "tl".to_string(),
            oo_doubletap_enabled: false,
            nn_doubletap_enabled: false,
            is_translate_swapped: false,
            is_association_recording_enabled: false,
            platform_id: 0,
            output_both_scripts: false,
            candidate_display_mode: 0,
        }
    }

    fn config_poj() -> AppConfig {
        AppConfig {
            input_mode: "poj".to_string(),
            ..config_tl()
        }
    }

    fn config_poj_doubletap() -> AppConfig {
        AppConfig {
            oo_doubletap_enabled: true,
            nn_doubletap_enabled: true,
            ..config_poj()
        }
    }

    /// Every char boundary of `raw` → the display offset the caret gets there.
    fn caret_map(raw: &str, config: &AppConfig) -> (String, Vec<usize>) {
        let display = derived_display(raw, config);
        let boundaries = raw
            .char_indices()
            .map(|(offset, _)| offset)
            .chain(std::iter::once(raw.len()))
            .map(|offset| display_caret_utf16(raw, &display, offset))
            .collect();
        (display, boundaries)
    }

    #[test]
    fn display_caret_tone_digit_folds_into_the_marked_vowel() {
        // trace: raw t|a|i|5, display "tâi" (t, â, i: 3 units). t→1; a matches
        // â→2; i→3; 5 unmatched (dropped) → stays 3; end → 3.
        let (display, boundaries) = caret_map("tai5", &config_tl());
        assert_eq!(display, "t\u{e2}i");
        assert_eq!(boundaries, vec![0, 1, 2, 3, 3]);
    }

    #[test]
    fn display_caret_syllabic_nasal_keeps_the_caret_after_the_marked_letter() {
        // Codex counterexample `ng|5`: display is n + U+0302 + g (no
        // precomposed form). trace: n matches n→1, then the mark is
        // absorbed→2; g→3; 5 dropped→3. Prefix-derivation would have said 2.
        let (display, boundaries) = caret_map("ng5", &config_tl());
        assert_eq!(display, "n\u{302}g");
        assert_eq!(boundaries, vec![0, 2, 3, 3]);
    }

    #[test]
    fn display_caret_literal_buffer_is_identity() {
        // Codex counterexample `ka2|i`: a digit mid-buffer is not a syllable
        // end, so the display stays literal `ka2i` and the caret after the
        // `2` is 3 — prefix-derivation (`ká` → 2) would have put it before it.
        let (display, boundaries) = caret_map("ka2i", &config_tl());
        assert_eq!(display, "ka2i");
        assert_eq!(boundaries, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn display_caret_hyphen_and_case_are_one_to_one() {
        // trace: "Tai5-gI2" → "Tâi-gí". T~T→1, a~â→2, i→3, 5 dropped→3,
        // -→4, g→5, I~í→6, 2 dropped→6.
        let (display, boundaries) = caret_map("Tai5-gI2", &config_tl());
        assert_eq!(display, "T\u{e2}i-g\u{ed}");
        assert_eq!(boundaries, vec![0, 1, 2, 3, 3, 4, 5, 6, 6]);
    }

    #[test]
    fn display_caret_poj_double_o_shares_one_offset_across_the_fold() {
        // POJ doubletap: "hoon" → "ho͘n" (h, o, U+0358, n: 4 units). trace:
        // h→1; first o matches o→2 then absorbs U+0358→3; the dot + next raw
        // `o` is the fold signature → second o consumed, stays 3; n→4. So `h|oo`=1, `ho|o`=3, `hoo|`=3,
        // and the caret before the trailing n is 3, not the end.
        let (display, boundaries) = caret_map("hoon", &config_poj_doubletap());
        assert_eq!(display, "ho\u{358}n");
        assert_eq!(boundaries, vec![0, 1, 3, 3, 4]);
    }

    #[test]
    fn display_caret_poj_nasal_double_n_matches_the_first_tap() {
        // POJ doubletap: "tinn" → "tiⁿ" (U+207F, NFKD `n`). trace: t→1, i→2,
        // first n matches ⁿ→3, the fold signature consumes the second n→3.
        let (display, boundaries) = caret_map("tinn", &config_poj_doubletap());
        assert_eq!(display, "ti\u{207f}");
        assert_eq!(boundaries, vec![0, 1, 2, 3, 3]);
    }

    #[test]
    fn display_caret_poj_tone_on_double_o_never_splits_the_cluster() {
        // "koo2" → k + ó + U+0358 (POJ puts the mark between o and the dot;
        // NFC keeps ó precomposed). trace: k→1; o matches ó→2, absorbs
        // U+0358→3; o dropped→3; 2 dropped→3.
        let (display, boundaries) = caret_map("koo2", &config_poj_doubletap());
        assert_eq!(display, "k\u{f3}\u{358}");
        assert_eq!(boundaries, vec![0, 1, 3, 3, 3]);
    }

    #[test]
    fn display_caret_tps_separator_marker_takes_no_step() {
        // TPS: the U+0020 marker is dropped from the display. trace over
        // "ㄉㄞ ㄍ" (raw bytes 3+3+1+3): ㄉ→1, ㄞ→2, ' ' dropped→2, ㄍ→3.
        let (display, boundaries) = caret_map("\u{3109}\u{311e} \u{310d}", &config_tl());
        assert_eq!(display, "\u{3109}\u{311e}\u{310d}");
        assert_eq!(boundaries, vec![0, 1, 2, 2, 3]);
    }

    #[test]
    fn display_caret_past_the_end_clamps_and_off_boundary_lands_after_the_char() {
        assert_eq!(display_caret_utf16("tai5", "t\u{e2}i", 99), 3);
        // Byte 1 sits inside the 3-byte ㄉ: the walk consumes ㄉ (boundary 0
        // is below the caret) and stops at ㄞ (boundary 3 is not), so the
        // caret lands after ㄉ.
        assert_eq!(
            display_caret_utf16("\u{3109}\u{311e}", "\u{3109}\u{311e}", 1),
            1
        );
        assert_eq!(display_caret_utf16("", "", 0), 0);
    }

    #[test]
    fn display_caret_folded_double_o_does_not_steal_a_later_o() {
        // Codex post-impl counterexample: "hooon" folds only the first pair →
        // "ho͘on". trace: h→1; o matches o, absorbs the dot→3, the dot + next
        // raw `o` is the fold signature → second tap consumed, still 3;
        // third o matches the display o→4; n→5. So `hoo|on` is 3 and typing
        // `k` there gives `ho͘kon` with the caret where the host showed it.
        let (display, boundaries) = caret_map("hooon", &config_poj_doubletap());
        assert_eq!(display, "ho\u{358}on");
        assert_eq!(boundaries, vec![0, 1, 3, 3, 4, 5]);
    }

    #[test]
    fn display_caret_typed_poj_dot_is_not_a_fold() {
        // The user typed o͘ themselves (custom key): raw "ho\u{358}o" is
        // already the display. trace: h→1; o matches o, absorbs the typed
        // dot→3, but the next raw char is the dot, not `o` → no fold; the raw
        // dot matches no letter → dropped, 3; the second o matches→4.
        let raw = "ho\u{358}o";
        let (display, boundaries) = caret_map(raw, &config_poj_doubletap());
        assert_eq!(display, raw);
        assert_eq!(boundaries, vec![0, 1, 3, 3, 4]);
    }

    #[test]
    fn display_caret_capital_nasal_marker_is_a_fold_too() {
        // Codex re-review: after a capital vowel the chain writes ᴺ (U+1D3A),
        // whose NFKD is `N`. "TINNN" → "TIᴺN". trace: T→1, I→2, N matches
        // ᴺ→3 and the next raw N is the second tap → consumed, 3; N→4.
        let (display, boundaries) = caret_map("TINNN", &config_poj_doubletap());
        assert_eq!(display, "TI\u{1d3a}N");
        assert_eq!(boundaries, vec![0, 1, 2, 3, 3, 4]);
    }

    #[test]
    fn display_caret_folded_nasal_does_not_steal_a_later_n() {
        // "tinnn" → "tiⁿn" (the pair folds, the third n stays). trace: t→1,
        // i→2, n matches ⁿ→3 and the next raw n is the second tap → consumed,
        // 3; third n matches n→4.
        let (display, boundaries) = caret_map("tinnn", &config_poj_doubletap());
        assert_eq!(display, "ti\u{207f}n");
        assert_eq!(boundaries, vec![0, 1, 2, 3, 3, 4]);
    }

    #[test]
    fn raw_input_idle_is_empty() {
        assert_eq!(Phase::Idle.raw_input(&config_tl()), "");
    }

    #[test]
    fn raw_input_composing_empty_is_empty() {
        let phase = Phase::Composing {
            raw: String::new(),
            caret: 0,
        };
        assert_eq!(phase.raw_input(&config_tl()), "");
    }

    #[test]
    fn raw_input_composing_tl_is_literal_no_poj_spelling_fold() {
        // TL composing display is literal (2026-06-05): the tone mark lands
        // on the typed letters with NO spelling fold. POJ-style `goa2` keeps
        // `goa` (mark on `a` per TL rule → `goá`), NOT canonicalized to `guá`.
        // `ai3`→`ài`, `li2`→`lí`; hyphens preserved.
        let phase = Phase::Composing {
            raw: "goa2-ai3-li2".to_string(),
            caret: 0,
        };
        assert_eq!(phase.raw_input(&config_tl()), "go\u{e1}-\u{e0}i-l\u{ed}");
    }

    #[test]
    fn raw_input_composing_tl_preserves_special_final_eng() {
        // Regression for the `téng` bug: the TL special nasal final `eng`
        // [ɛŋ] must NOT be folded to `ing` [iŋ]. `teng2` → `téng`, NOT `tíng`.
        let phase = Phase::Composing {
            raw: "teng2".to_string(),
            caret: 0,
        };
        assert_eq!(phase.raw_input(&config_tl()), "t\u{e9}ng");
    }

    #[test]
    fn raw_input_composing_poj_is_literal_no_spelling_fold() {
        // POJ composing display is ALSO literal (2026-06-05): the tone mark
        // lands on the typed letters, NO spelling conversion. A user typing TL
        // spelling in POJ mode keeps it: `ting2` → `tíng` (NOT `téng`). POJ-
        // spelled input is likewise verbatim with POJ tone placement:
        // `goa2` → `góa` (mark on `o`), `teng2` → `téng`.
        assert_eq!(
            Phase::Composing {
                raw: "ting2".to_string(),
                caret: 0,
            }
            .raw_input(&config_poj()),
            "t\u{ed}ng"
        );
        assert_eq!(
            Phase::Composing {
                raw: "goa2".to_string(),
                caret: 0,
            }
            .raw_input(&config_poj()),
            "g\u{f3}a"
        );
        assert_eq!(
            Phase::Composing {
                raw: "teng2".to_string(),
                caret: 0,
            }
            .raw_input(&config_poj()),
            "t\u{e9}ng"
        );
    }

    #[test]
    fn raw_input_composing_poj_applies_both_doubletap_affordances() {
        // The preedit is what the user sees AND what a commit writes to the
        // document, so pin the fix on the real display path, not just on
        // `normalize_tone` in isolation: with both toggles on, typing
        // `h o o n n` must read `ho͘ⁿ`, not `ho͘nn`.
        let config = AppConfig {
            oo_doubletap_enabled: true,
            nn_doubletap_enabled: true,
            ..config_poj()
        };
        assert_eq!(
            Phase::Composing {
                raw: "hoonn".to_string(),
                caret: 0,
            }
            .raw_input(&config),
            "ho\u{0358}\u{207f}"
        );
        assert_eq!(
            Phase::Composing {
                raw: "hoonnh".to_string(),
                caret: 0,
            }
            .raw_input(&config),
            "ho\u{0358}\u{207f}h"
        );
        // Canonical spelling unchanged.
        assert_eq!(
            Phase::Composing {
                raw: "honn".to_string(),
                caret: 0,
            }
            .raw_input(&config),
            "ho\u{207f}"
        );
    }

    #[test]
    fn raw_input_composing_tl_preserves_unhyphenated_input_verbatim() {
        // §10.2 amendment 2026-05-13 — engine does NOT auto-insert syllable
        // boundaries; if the user typed no hyphens, derived display has no
        // boundary to convert and returns the raw single chunk.
        let phase = Phase::Composing {
            raw: "goa2ai3li2".to_string(),
            caret: 0,
        };
        assert_eq!(phase.raw_input(&config_tl()), "goa2ai3li2");
    }

    #[test]
    fn raw_input_composing_poj_uses_poj_diacritics() {
        let phase = Phase::Composing {
            raw: "goa2".to_string(),
            caret: 0,
        };
        assert_eq!(phase.raw_input(&config_poj()), "góa");
    }

    #[test]
    fn raw_input_continuous_returns_pending_tail_only() {
        // Mid-commit state: "tsua" already nailed, "li2" still pending.
        let phase = Phase::Continuous {
            raw: "li2".to_string(),
            caret: 0,
            nailed: vec![NailedSegment {
                display_text: "紙".to_string(),
                canonical_text: "紙".to_string(),
                raw_text: "tsua".to_string(),
                association_tl: "tsuá".to_string(),
                raw_span: (0, 4),
                syllable_count: 1,
            }],
        };
        assert_eq!(phase.raw_input(&config_tl()), "lí");
    }

    #[test]
    fn raw_input_continuous_empty_pending_is_empty() {
        let phase = Phase::Continuous {
            raw: String::new(),
            caret: 0,
            nailed: vec![NailedSegment {
                display_text: "紙".to_string(),
                canonical_text: "紙".to_string(),
                raw_text: "tsua".to_string(),
                association_tl: "tsuá".to_string(),
                raw_span: (0, 4),
                syllable_count: 1,
            }],
        };
        assert_eq!(phase.raw_input(&config_tl()), "");
    }

    #[test]
    fn raw_input_tps_glyphs_pass_through_verbatim() {
        let phase = Phase::Composing {
            raw: "ㄍㄨㄚˋ".to_string(),
            caret: 0,
        };
        assert_eq!(phase.raw_input(&config_tl()), "ㄍㄨㄚˋ");
    }

    // §41 — the separator marker is hidden at the display seam. Trailing
    // (`ㄒㄧ` + space, the reported case), interior (`ㄍㄠ`␣`ㄉㄞ`), and
    // repeated separators all render as the bare glyph run.
    #[test]
    fn raw_input_tps_hides_the_trailing_separator_marker() {
        let phase = Phase::Composing {
            raw: "ㄒㄧ ".to_string(),
            caret: 0,
        };
        assert_eq!(phase.raw_input(&config_tl()), "ㄒㄧ");
    }

    #[test]
    fn raw_input_tps_hides_an_interior_separator_marker() {
        let phase = Phase::Composing {
            raw: "ㄍㄠ ㄉㄞ".to_string(),
            caret: 0,
        };
        assert_eq!(phase.raw_input(&config_tl()), "ㄍㄠㄉㄞ");
    }

    #[test]
    fn raw_input_tps_hides_repeated_separator_markers() {
        let phase = Phase::Composing {
            raw: "ㄍㄠ  ㄉㄞ ".to_string(),
            caret: 0,
        };
        assert_eq!(phase.raw_input(&config_tl()), "ㄍㄠㄉㄞ");
    }

    #[test]
    fn raw_input_tps_keeps_tone_marks_while_hiding_the_separator() {
        let phase = Phase::Composing {
            raw: "ㄉㄞˊ ㆣㄧˋ".to_string(),
            caret: 0,
        };
        assert_eq!(phase.raw_input(&config_tl()), "ㄉㄞˊㆣㄧˋ");
    }

    // A TL/POJ space is a literal word boundary and must survive — the
    // hide rule is TPS-only, keyed on the buffer actually carrying TPS.
    #[test]
    fn raw_input_tl_keeps_a_literal_space() {
        let phase = Phase::Composing {
            raw: "tai uan".to_string(),
            caret: 0,
        };
        assert_eq!(phase.raw_input(&config_tl()), "tai uan");
    }
}
