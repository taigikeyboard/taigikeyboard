//! POJ ↔ TL display-level rewrite (iOS / Android `pojDisplayToTLDisplay` /
//! `tlDisplayToPOJDisplay`) and the canonical TL commit form built on it.

use crate::api::{capitalize_first, InputMode, System};
use crate::poj::to_poj;
use crate::syllable::{
    is_stop_tone, normalize_to_tl, normalize_to_tl_keep_tl_finals, split_initial_final,
    strip_tone_mark,
};
use crate::tl::to_tl;

pub fn poj_display_to_tl_display(text: &str) -> String {
    rewrite_display(text, System::Tl, false, DisplayCase::Keep)
}

pub fn tl_display_to_poj_display(text: &str) -> String {
    rewrite_display(text, System::Poj, false, DisplayCase::Keep)
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
