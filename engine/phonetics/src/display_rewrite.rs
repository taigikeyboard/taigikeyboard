//! POJ ↔ TL display-level rewrite (iOS / Android `pojDisplayToTLDisplay` /
//! `tlDisplayToPOJDisplay`) and the canonical TL commit form built on it.

use std::borrow::Cow;

use crate::api::{capitalize_first, InputMode, System};
use crate::case_transform::{full_uppercase_tone_string, raise_case, LetterCase};
use crate::khinsiann::{is_roman_syllable_char, render_hanji_khinsiann};
use crate::poj::to_poj;
use crate::syllable::{
    is_stop_tone, normalize_to_tl, normalize_to_tl_keep_tl_finals, split_initial_final,
    strip_tone_mark,
};
use crate::tl::to_tl;

pub fn poj_display_to_tl_display(text: &str) -> String {
    rewrite_display(text, System::Tl, false, DisplayCase::Keep)
}

/// [`poj_display_to_tl_display`] that keeps the TL special finals `eng` /
/// `ek` literal (as [`canonical_tl_form`] does in TL mode): for a spelling
/// read as TL, where `téng` is TL's own final, not POJ's `eng` (TL `ing`).
pub(crate) fn tl_display_keeping_tl_finals(text: &str) -> String {
    rewrite_display(text, System::Tl, true, DisplayCase::Keep)
}

pub fn tl_display_to_poj_display(text: &str) -> String {
    rewrite_display(text, System::Poj, false, DisplayCase::Keep)
}

/// A dictionary or learned word's Hanji as the candidate strip shows it: the
/// khinsiann marks when `khinsiann` is on ([`render_hanji_khinsiann`]), its TL
/// runs in POJ when `poj_roman` is on ([`tl_roman_in_hanji_to_poj`]). Neither
/// changes the syllable count, so the order does not matter. Borrows when
/// nothing changes.
pub fn render_hanji_display<'a>(
    hanji: &'a str,
    canonical_tl: &str,
    khinsiann: bool,
    poj_roman: bool,
) -> Cow<'a, str> {
    let marked = render_hanji_khinsiann(hanji, canonical_tl, khinsiann);
    if !poj_roman {
        return marked;
    }
    match marked {
        Cow::Borrowed(h) => tl_roman_in_hanji_to_poj(h),
        Cow::Owned(h) => match tl_roman_in_hanji_to_poj(&h) {
            Cow::Owned(rendered) => Cow::Owned(rendered),
            Cow::Borrowed(_) => Cow::Owned(h),
        },
    }
}

/// The Hanji spelling as POJ mode shows it: every romanization run inside a
/// Han-Lo spelling (`親tsiânn`, `lóo力`) rewritten TL → POJ (`親chiâⁿ`,
/// `ló͘力`), Hanji, kana, separators and other characters as they are. The
/// dictionary stores one spelling per word, its romanization in TL; this is
/// presentation only — identity keys keep the TL spelling (Core Principle 6).
///
/// A run that only changes case (`OK` → `Ok`) or does not parse as a
/// syllable (`iPhone`) stays as written. Borrows when nothing changes.
pub fn tl_roman_in_hanji_to_poj(hanji: &str) -> Cow<'_, str> {
    // Built only once a run changes: `hanji[..copied_to]` is already in it.
    let mut out: Option<String> = None;
    let mut copied_to = 0;
    let mut run_start: Option<usize> = None;
    let run_ends = hanji
        .char_indices()
        .map(|(idx, c)| (idx, is_roman_syllable_char(c)))
        .chain(std::iter::once((hanji.len(), false)));
    for (idx, in_run) in run_ends {
        if in_run {
            run_start.get_or_insert(idx);
            continue;
        }
        let Some(start) = run_start.take() else {
            continue;
        };
        let run = &hanji[start..idx];
        let rewritten = rewrite_token(run, System::Poj, false, DisplayCase::Keep);
        let case_only = rewritten
            .chars()
            .flat_map(char::to_lowercase)
            .eq(run.chars().flat_map(char::to_lowercase));
        if case_only {
            continue;
        }
        let buf = out.get_or_insert_with(|| String::with_capacity(hanji.len() + 8));
        buf.push_str(&hanji[copied_to..start]);
        buf.push_str(&rewritten);
        copied_to = idx;
    }
    match out {
        None => Cow::Borrowed(hanji),
        Some(mut buf) => {
            buf.push_str(&hanji[copied_to..]);
            Cow::Owned(buf)
        }
    }
}

/// A Han-Lo spelling's romanized words raised to `letter_case`, the typed case
/// of its span: the runs [`tl_roman_in_hanji_to_poj`] reads, and of those only
/// the ones that parse as a Taigi syllable (TL or POJ) — `iPhone`, `macOS` and
/// every Hanji, kana or separator stay as written.
///
/// - `CapsLocked` raises every Taigi run: `我tsia̍h` → `我TSIA̍H`,
///   `iPhone tsia̍h` → `iPhone TSIA̍H`.
/// - `Uppercased` capitalizes the spelling's first letter only when it opens a
///   Taigi run: `tsia̍h來` → `Tsia̍h來`; a leading Hanji (`我tsia̍h`) or
///   non-Taigi word (`iPhone來`) stops it, so no later word is capitalized.
/// - `Lowercased` changes nothing.
///
/// Presentation only, run on the shown spelling (after the POJ rewrite, which
/// reads lowercase TL). Borrows when nothing changes.
pub fn raise_taigi_roman_case_in_hanji(hanji: &str, letter_case: LetterCase) -> Cow<'_, str> {
    if letter_case == LetterCase::Lowercased || !hanji.chars().any(char::is_lowercase) {
        return Cow::Borrowed(hanji);
    }
    // The first letter (a Hanji counts) decides capitalize-first; past it,
    // only Caps Lock raises anything.
    let first_letter = hanji.char_indices().find(|(_, c)| c.is_alphabetic());
    let mut out: Option<String> = None;
    let mut copied_to = 0;
    let mut run_start: Option<usize> = None;
    let run_ends = hanji
        .char_indices()
        .map(|(idx, c)| (idx, is_roman_syllable_char(c)))
        .chain(std::iter::once((hanji.len(), false)));
    for (idx, in_run) in run_ends {
        if in_run {
            run_start.get_or_insert(idx);
            continue;
        }
        let Some(start) = run_start.take() else {
            continue;
        };
        let run = &hanji[start..idx];
        let raised = match letter_case {
            LetterCase::CapsLocked => full_uppercase_tone_string(run),
            LetterCase::Uppercased if first_letter.map(|(at, _)| at) == Some(start) => {
                raise_case(run, LetterCase::Uppercased)
            }
            _ => continue,
        };
        if raised == run || !parses_as_taigi_syllable(run) {
            continue;
        }
        let buf = out.get_or_insert_with(|| String::with_capacity(hanji.len() + 8));
        buf.push_str(&hanji[copied_to..start]);
        buf.push_str(&raised);
        copied_to = idx;
    }
    match out {
        None => Cow::Borrowed(hanji),
        Some(mut buf) => {
            buf.push_str(&hanji[copied_to..]);
            Cow::Owned(buf)
        }
    }
}

/// Whether `token` (one romanization run, tone mark and all) splits into a TL
/// initial and final once POJ-shaped spellings fold onto TL — the parse
/// [`rewrite_token`] converts, and leaves a token as written without.
fn parses_as_taigi_syllable(token: &str) -> bool {
    let (bare, _) = strip_tone_mark(token);
    !bare.is_empty() && split_initial_final(&normalize_to_tl(&bare.to_lowercase())).is_some()
}

/// v3.5.9 B-4 — mode-aware canonicalizer for the `user_frequency.db`
/// commit key (`RawCandidate.display_text`). Folds Taigi-script
/// romanization onto canonical TL display form so a romanization-only
/// candidate (hanji-absent) keys identically across modes; downstream
/// presentation reverses via `recase_tl_as_poj_display`. Composes
/// [`poj_display_to_tl_display`] for **both** `Tl` and `Poj` modes
/// (PR #310 r3278520895 fix — a POJ-form custom entry typed in POJ
/// mode and later accessed in TL mode still needs the fold or
/// `user_frequency.db` keys split). `English` mode is intentionally
/// identity — English custom entries like `hello` must not be
/// reinterpreted as Taigi.
///
/// Idempotence on TL input: for already-TL display form the rewrite
/// chain (`strip_tone_mark` → `normalize_to_tl` → `split_initial_final`
/// → `to_tl`) returns the same TL string. Pass-through on non-Taigi:
/// `split_initial_final` fails the phonotactic split and the rewriter
/// returns the original token. Both cases mean `Tl` mode for ordinary
/// TL custom entries is observably identity at the byte level.
///
/// **Case is not identity.** A `TL` / `POJ` syllable comes out lowercase: an
/// auto-capitalized `Kóng` and a typed `kóng` are one word (Core Principle
/// 6), and the dictionary's readings are all lowercase. Text that does not
/// parse as a syllable (`iPhone`) and `English` keep their case: that is the
/// word the user wrote.
pub fn canonical_tl_form(text: &str, mode: InputMode) -> String {
    match mode {
        // TL mode: still fold POJ-shaped spellings onto canonical TL for the
        // cross-mode identity (`góa`→`guá`, B-4 / R2 / R5), but keep TL special
        // finals `eng`[ɛŋ] / `ek` literal so a hanji-absent `teng` candidate's
        // committed `display_text` is `teng`, not `tíng` (2026-06-05 TL-literal).
        InputMode::Tl => rewrite_display(text, System::Tl, true, DisplayCase::Drop),
        // POJ mode: input genuinely IS POJ, so `eng`→`ing` is a correct POJ→TL
        // conversion — use the full fold.
        InputMode::Poj => rewrite_display(text, System::Tl, false, DisplayCase::Drop),
        // v3.5.9 D / C-3b — TPS roman is Bopomofo, not POJ/TL Latin shape;
        // poj_display_to_tl_display would no-op on Bopomofo anyway (no Latin
        // patterns to substitute). Only the ㆳ glyph alias folds onto ㆪ, so a
        // raw commit typed with either glyph keys one user_frequency.db row.
        InputMode::Tps => text.chars().map(crate::tps::fold_tps_glyph_alias).collect(),
        InputMode::English => text.to_string(),
    }
}

/// `keep_tl_finals = true` swaps the spelling fold to
/// [`normalize_to_tl_keep_tl_finals`] (drops `eng→ing` / `ek→ik`) so a TL
/// special final survives — used only by `canonical_tl_form` for TL mode. The
/// public `poj_display_to_tl_display` / `tl_display_to_poj_display` (and POJ-mode
/// `canonical_tl_form`) pass `false` for the full POJ↔TL fold.
/// Whether a rewritten syllable keeps the capital its token started with.
#[derive(Clone, Copy, PartialEq, Eq)]
enum DisplayCase {
    /// Display conversions: `Góa` stays `Guá`.
    Keep,
    /// The identity fold: `Góa` keys `guá`. Only parsed syllables lose case;
    /// a token that is not Taigi (`iPhone`) is returned as typed.
    Drop,
}

fn rewrite_display(text: &str, target: System, keep_tl_finals: bool, case: DisplayCase) -> String {
    if text.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    let mut current = String::new();
    for ch in text.chars() {
        if ch == '-' || ch == ' ' {
            out.push_str(&rewrite_token(&current, target, keep_tl_finals, case));
            out.push(ch);
            current.clear();
        } else {
            current.push(ch);
        }
    }
    out.push_str(&rewrite_token(&current, target, keep_tl_finals, case));
    out
}

fn rewrite_token(token: &str, target: System, keep_tl_finals: bool, case: DisplayCase) -> String {
    if token.is_empty() {
        return String::new();
    }
    let (bare, tone) = strip_tone_mark(token);
    if bare.is_empty() {
        return token.to_string();
    }
    let lowered = bare.to_lowercase();
    let normalized = if keep_tl_finals {
        normalize_to_tl_keep_tl_finals(&lowered)
    } else {
        normalize_to_tl(&lowered)
    };
    let Some((initial, final_str)) = split_initial_final(&normalized) else {
        return token.to_string();
    };
    let resolved_tone = if tone.is_empty() {
        if is_stop_tone(&final_str) { "4" } else { "1" }.to_string()
    } else {
        tone
    };
    let assembled = match target {
        System::Tl => to_tl(&initial, &final_str, &resolved_tone),
        System::Poj => to_poj(&initial, &final_str, &resolved_tone),
        System::Tps => return token.to_string(),
    };
    let first = token.chars().next().unwrap();
    if case == DisplayCase::Keep && first.is_uppercase() {
        capitalize_first(&assembled)
    } else {
        assembled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Expected POJ runs are the production `dictionary.csv` `poj` column of
    // the same rows (`lóo力` → `ló͘-la̍t`, `親tsiânn` → `chhin-chiâⁿ`, …).
    #[test]
    fn taigi_runs_in_han_lo_spellings_follow_the_typed_case() {
        use LetterCase::{CapsLocked, Lowercased, Uppercased};
        let cases = [
            // Caps Lock: every Taigi run, TL or POJ; Hanji stay.
            ("我tsia̍h", CapsLocked, "我TSIA̍H"),
            ("親tsiânn仔", CapsLocked, "親TSIÂNN仔"),
            ("親chi\u{e2}\u{207f}", CapsLocked, "親CHI\u{c2}\u{207f}"),
            ("恬tsih-tsih", CapsLocked, "恬TSIH-TSIH"),
            ("l\u{f3}\u{358}力", CapsLocked, "L\u{d3}\u{358}力"),
            // A non-Taigi word stays as written, the Taigi run beside it rises.
            ("iPhone殼", CapsLocked, "iPhone殼"),
            ("iPhone tsia̍h", CapsLocked, "iPhone TSIA̍H"),
            ("macOS", CapsLocked, "macOS"),
            // Capitalize-first: only a spelling that opens with a Taigi run.
            ("tsia̍h來", Uppercased, "Tsia̍h來"),
            ("我tsia̍h", Uppercased, "我tsia̍h"),
            ("iPhone tsia̍h", Uppercased, "iPhone tsia̍h"),
            ("啥mih", Uppercased, "啥mih"),
            // Lowercase and pure Hanji: nothing to do.
            ("tsia̍h來", Lowercased, "tsia̍h來"),
            ("我愛你", CapsLocked, "我愛你"),
        ];
        for (hanji, case, expected) in cases {
            let raised = raise_taigi_roman_case_in_hanji(hanji, case);
            assert_eq!(raised, expected, "{hanji} {case:?}");
            if hanji == expected {
                assert!(matches!(raised, Cow::Borrowed(_)), "{hanji} borrows");
            }
        }
    }

    #[test]
    fn tl_runs_in_han_lo_spellings_render_as_poj() {
        let cases = [
            ("親tsiânn", "親chi\u{e2}\u{207f}"),
            ("l\u{f3}o力", "l\u{f3}\u{358}力"),
            ("Ph\u{f3}o-meh狗", "Ph\u{f3}\u{358}-meh狗"),
            ("恬tsih-tsih", "恬chih-chih"),
            ("ing暗", "eng暗"),
            ("tso\u{30d}h--日", "cho\u{30d}h--日"),
            (
                "重hi\u{30d}nnh-hi\u{30d}nnh",
                "重hi\u{30d}\u{207f}h-hi\u{30d}\u{207f}h",
            ),
            ("oo-m\u{fa} 卵", "o\u{358}-m\u{fa} 卵"),
        ];
        for (hanji, expected) in cases {
            assert_eq!(tl_roman_in_hanji_to_poj(hanji), expected, "{hanji}");
        }
    }

    #[test]
    fn spellings_without_a_changed_run_are_borrowed() {
        // `OK` parses (`ok`) but only its case would change; `iPhone` does not
        // parse; `kah` is spelled alike in TL and POJ.
        for hanji in ["親情", "OK球", "iPhone殼", "kah意", ""] {
            assert!(
                matches!(tl_roman_in_hanji_to_poj(hanji), Cow::Borrowed(h) if h == hanji),
                "{hanji}"
            );
        }
    }
}
