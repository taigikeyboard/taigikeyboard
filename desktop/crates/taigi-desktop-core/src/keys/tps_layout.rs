//! The physical-keyboard TPS layout: which glyph a key types under TPS
//! (`docs/architecture/desktop-tps-roadmap.md` § D2). Every symbol Mandarin
//! Zhuyin shares with TPS keeps its system (Dachen) key, so a Zhuyin typist's
//! hands already know the base layer; the TPS-only glyphs take the keys
//! Dachen leaves free and the Shift layer, where Shift is the key's sister
//! sound (voiced, syllabic or nasalized).
//!
//! Keyed on the character a key types on a US layout — the same assumption
//! the slot keys make. A letter is read lowercased with the Shift modifier
//! choosing the layer, so Caps Lock never selects it; a digit or punctuation
//! key is read as typed, because Windows and Linux hand over the shifted
//! character (`!`, `^`, `)`) rather than the key's base one.

use super::chord::is_keypad_key_code;
use super::snapshot::{KeyEventSnapshot, KeyModifiers};

/// Every key the layout assigns, as `(typed character, glyph)`. A shifted
/// letter is spelled as its capital; a shifted digit or punctuation key as the
/// character Shift makes it type. A key or Shift layer missing here is not a
/// TPS key and types as it would in any other mode.
const KEYS: &[(char, &str)] = &[
    // Initials.
    ('1', "ㄅ"),
    ('!', "ㆠ"),
    ('q', "ㄆ"),
    ('a', "ㄇ"),
    ('2', "ㄉ"),
    ('w', "ㄊ"),
    ('s', "ㄋ"),
    ('x', "ㄌ"),
    ('e', "ㄍ"),
    ('E', "ㆣ"),
    ('d', "ㄎ"),
    ('D', "ㄫ"),
    ('c', "ㄏ"),
    ('r', "ㄐ"),
    ('R', "ㆢ"),
    ('f', "ㄑ"),
    ('v', "ㄒ"),
    ('y', "ㄗ"),
    ('Y', "ㆡ"),
    ('h', "ㄘ"),
    ('n', "ㄙ"),
    // Vowels and nasalized vowels. ㄛ ㄜ ㄝ keep their Dachen keys (TL `o`
    // is ㄛ, ㄜ is `er` / `or`, `engine/phonetics/src/tps.rs`); ㆨ takes the
    // Dachen ㄩ key, so ㄧ ㄨ ㆨ stand in one column, and ㆦ the ㄡ key, the
    // sound it is closest to — both free under TPS.
    ('8', "ㄚ"),
    ('*', "ㆩ"),
    ('i', "ㄛ"),
    ('k', "ㄜ"),
    ('o', "ㆤ"),
    ('O', "ㆥ"),
    ('m', "ㆨ"),
    ('.', "ㆦ"),
    ('>', "ㆧ"),
    (',', "ㄝ"),
    ('u', "ㄧ"),
    ('U', "ㆪ"),
    ('j', "ㄨ"),
    ('J', "ㆫ"),
    ('9', "ㄞ"),
    ('(', "ㆮ"),
    ('l', "ㄠ"),
    ('L', "ㆯ"),
    // Nasal finals, each with its sister on Shift: ㄣ ㆬ (n / m), ㄥ ㆭ
    // (ng), ㄢ ㆰ (an / am), ㆲ ㆱ (ong / om) — ㆲ, the common one, bare on
    // the free Dachen ㄦ key.
    ('0', "ㄢ"),
    (')', "ㆰ"),
    (';', "ㄤ"),
    ('-', "ㆲ"),
    ('_', "ㆱ"),
    ('p', "ㄣ"),
    ('P', "ㆬ"),
    ('/', "ㄥ"),
    ('?', "ㆭ"),
    // Stop codas in p t k order down one column, ㆷ on `z`.
    ('t', "ㆴ"),
    ('g', "ㆵ"),
    ('b', "ㆻ"),
    ('z', "ㆷ"),
    // Tone marks 2, 3, 5, 6, 7, 8 and 9; tones 1 and 4 are Space. Tone 8 is
    // U+02D9, what mobile types; the engine folds it for lookup. `3` is tone
    // 3 (˪); Dachen's ˇ, tone 6, sits on Shift+`3`.
    ('4', "\u{02cb}"),
    ('3', "\u{02ea}"),
    ('6', "\u{02ca}"),
    ('#', "\u{02c7}"),
    ('5', "\u{02eb}"),
    ('7', "\u{02d9}"),
    ('^', "\u{02c6}"),
    // The hyphen (`--` writes the neutral tone), on the free `=` since ㆲ
    // took `-`.
    ('=', "-"),
];

/// The glyph `event` types under TPS, or `None` for a key the layout does not
/// assign — including any key chorded with Control, Alt or the Windows /
/// Command key, which stays the host's or a shortcut's, and every keypad key,
/// which types the same character as its main-block twin: the digits pick
/// candidates (`CandidateSlotKeySet::TpsDigits`), the rest are plain text.
pub fn tps_glyph_for_event(event: &KeyEventSnapshot) -> Option<&'static str> {
    if event.key_code.is_some_and(is_keypad_key_code) {
        return None;
    }
    let shift = match event.modifiers {
        KeyModifiers::NONE => false,
        KeyModifiers::SHIFT => true,
        _ => return None,
    };
    let mut characters = event.characters.as_deref()?.chars();
    let typed = characters.next()?;
    if characters.next().is_some() {
        return None;
    }
    let key = if typed.is_ascii_alphabetic() {
        if shift {
            typed.to_ascii_uppercase()
        } else {
            typed.to_ascii_lowercase()
        }
    } else {
        typed
    };
    glyph_for_key(key)
}

/// The glyph the layout assigns to `key`, spelled as in [`KEYS`] — a shifted
/// letter as its capital, a shifted digit or punctuation key as the
/// character Shift makes it type.
pub(super) fn glyph_for_key(key: char) -> Option<&'static str> {
    KEYS.iter()
        .find(|(assigned, _)| *assigned == key)
        .map(|(_, glyph)| *glyph)
}

/// Whether `glyph` is one the layout types — what a click on the on-screen
/// panel may hand the engine (`ComposingKeyIntent::tps_keyboard_press`).
pub(super) fn is_layout_glyph(glyph: &str) -> bool {
    KEYS.iter().any(|(_, assigned)| *assigned == glyph)
}

/// Every key the layout assigns, as typed (base and Shift layers together).
#[cfg(test)]
pub(super) fn assigned_keys() -> impl Iterator<Item = char> {
    KEYS.iter().map(|(key, _)| *key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn glyph(characters: &str, modifiers: KeyModifiers) -> Option<&'static str> {
        tps_glyph_for_event(&KeyEventSnapshot::text(characters, modifiers))
    }

    #[test]
    fn no_key_and_no_glyph_is_assigned_twice() {
        let keys: HashSet<char> = KEYS.iter().map(|(key, _)| *key).collect();
        let glyphs: HashSet<&str> = KEYS.iter().map(|(_, glyph)| *glyph).collect();
        assert_eq!(keys.len(), KEYS.len());
        assert_eq!(glyphs.len(), KEYS.len());
    }

    #[test]
    fn every_bare_letter_digit_and_layout_punctuation_key_types_a_glyph() {
        // trace: roadmap D2 — all 26 letters, all ten digits and `, ; / - . =`
        // carry a base glyph.
        for key in ('a'..='z').chain('0'..='9').chain(",;/-.=".chars()) {
            assert!(
                glyph(&key.to_string(), KeyModifiers::NONE).is_some(),
                "{key:?}"
            );
        }
    }

    #[test]
    fn shift_selects_the_tps_only_layer() {
        // trace: Shift+e types `E` → ㆣ; Shift+1 types `!` → ㆠ; Shift+p → ㆬ;
        // Shift+/ types `?` → ㆭ; Shift+- types `_` → ㆱ; Shift+. types `>`
        // → ㆧ; Shift+3 types `#` → ˇ.
        assert_eq!(glyph("e", KeyModifiers::NONE), Some("ㄍ"));
        assert_eq!(glyph("E", KeyModifiers::SHIFT), Some("ㆣ"));
        assert_eq!(glyph("1", KeyModifiers::NONE), Some("ㄅ"));
        assert_eq!(glyph("!", KeyModifiers::SHIFT), Some("ㆠ"));
        assert_eq!(glyph("p", KeyModifiers::NONE), Some("ㄣ"));
        assert_eq!(glyph("P", KeyModifiers::SHIFT), Some("ㆬ"));
        assert_eq!(glyph("/", KeyModifiers::NONE), Some("ㄥ"));
        assert_eq!(glyph("?", KeyModifiers::SHIFT), Some("ㆭ"));
        assert_eq!(glyph("-", KeyModifiers::NONE), Some("ㆲ"));
        assert_eq!(glyph("_", KeyModifiers::SHIFT), Some("ㆱ"));
        assert_eq!(glyph(".", KeyModifiers::NONE), Some("ㆦ"));
        assert_eq!(glyph(">", KeyModifiers::SHIFT), Some("ㆧ"));
        assert_eq!(glyph("#", KeyModifiers::SHIFT), Some("\u{02c7}"));
        assert_eq!(glyph("^", KeyModifiers::SHIFT), Some("\u{02c6}"));
    }

    #[test]
    fn caps_lock_does_not_select_the_shift_layer() {
        // Caps Lock alone: the letter arrives uppercased with no Shift → base.
        assert_eq!(glyph("E", KeyModifiers::NONE), Some("ㄍ"));
        // Caps Lock + Shift: the letter arrives lowercased with Shift → Shift layer.
        assert_eq!(glyph("e", KeyModifiers::SHIFT), Some("ㆣ"));
    }

    #[test]
    fn an_unassigned_shift_cell_or_a_chord_is_not_a_tps_key() {
        // trace: Shift+q (`Q`), Shift+k (`K`), Shift+2 (`@`), Shift+a (`A`),
        // Shift+m (`M`), Shift+; (`:`) and Shift+= (`+`) have no TPS glyph;
        // nor has the bare `'`.
        for shifted in ["Q", "K", "@", "A", "M", ":", "+"] {
            assert_eq!(glyph(shifted, KeyModifiers::SHIFT), None, "{shifted:?}");
        }
        assert_eq!(glyph("'", KeyModifiers::NONE), None);
        assert_eq!(glyph("e", KeyModifiers::CONTROL), None);
        assert_eq!(glyph("e", KeyModifiers::ALT), None);
        assert_eq!(glyph("e", KeyModifiers::WIN), None);
        assert_eq!(glyph("[", KeyModifiers::NONE), None);
        assert_eq!(glyph("ab", KeyModifiers::NONE), None);
    }

    #[test]
    fn a_keypad_key_is_not_a_tps_key_though_its_main_block_twin_is() {
        // trace: number-row `1` (VK 0x31) → ㄅ; keypad `1` (VK_NUMPAD1 0x61),
        // `0` (0x60), `.` (VK_DECIMAL 0x6E), `-` (VK_SUBTRACT 0x6D), `/`
        // (VK_DIVIDE 0x6F), `*` (VK_MULTIPLY 0x6A) and the Mac keypad `=` (0x92)
        // type the same characters and are not glyph keys.
        let number_row = KeyEventSnapshot::text("1", KeyModifiers::NONE).with_key_code(0x31);
        assert_eq!(tps_glyph_for_event(&number_row), Some("ㄅ"));
        for (typed, code) in [
            ("1", 0x61),
            ("0", 0x60),
            (".", 0x6E),
            ("-", 0x6D),
            ("/", 0x6F),
            ("*", 0x6A),
            ("=", 0x92),
        ] {
            let keypad = KeyEventSnapshot::text(typed, KeyModifiers::NONE).with_key_code(code);
            assert_eq!(tps_glyph_for_event(&keypad), None, "{typed:?}");
        }
    }

    #[test]
    fn tones_and_the_hyphen_type_what_mobile_types() {
        // trace: tps.rs ZHUYIN_TONES — 2 ˋ U+02CB, 3 ˪ U+02EA, 5 ˊ U+02CA,
        // 7 ˫ U+02EB, 8 ˙ U+02D9 (encode-safe form) on bare keys; 6 ˇ U+02C7
        // and 9 ˆ U+02C6 on Shift (`shift_selects_the_tps_only_layer`).
        let tones = [
            ("4", "\u{02cb}"),
            ("3", "\u{02ea}"),
            ("6", "\u{02ca}"),
            ("5", "\u{02eb}"),
            ("7", "\u{02d9}"),
        ];
        for (key, mark) in tones {
            assert_eq!(glyph(key, KeyModifiers::NONE), Some(mark), "{key}");
        }
        assert_eq!(glyph("=", KeyModifiers::NONE), Some("-"));
    }

    #[test]
    fn the_stop_codas_run_p_t_k_down_one_column() {
        // trace: reporter layout (Discord 2026-10-19) — `t` ㆴ (p), `g` ㆵ (t),
        // `b` ㆻ (k); `z` ㆷ (h).
        for (key, coda) in [("t", "ㆴ"), ("g", "ㆵ"), ("b", "ㆻ"), ("z", "ㆷ")] {
            assert_eq!(glyph(key, KeyModifiers::NONE), Some(coda), "{key}");
        }
    }
}
