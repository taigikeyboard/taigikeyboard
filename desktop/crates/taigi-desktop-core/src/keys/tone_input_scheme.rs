//! Which keys type a tone — and, by the same choice, which keys pick a
//! candidate. macOS keeps a Swift twin for the window labels:
//! `ToneInputScheme.swift`.

use super::slot_key_set::CandidateSlotKeySet;
use crate::settings::SettingChoice;
use crate::strings::StringKey;

/// Which keys type a tone — the half of the key contract that decides, in
/// turn, which keys pick a candidate.
///
/// Standard and Telex swap eight letters and the nine digits (USER
/// 2026-09-08). Under `Standard` a digit is the TL/POJ tone marker (`tai5`),
/// so the letters no syllable spells are free to pick candidates; under
/// `Telex` those letters carry the tones, so the digits are free to pick.
/// Neither half is chosen on its own: the slot key set is derived here
/// (`slot_key_set`) rather than stored, so the two can never disagree about
/// who owns `v` or `3`. Telex also claims `c` and `h`, which pick nothing
/// under Standard and stay plain letters there.
///
/// The letter table is the kahiok scheme (madmaxieee/taigi-telex) with `c` as
/// the unaspirated affricate rather than its `c` → `tsh`, which leaves `z` to
/// write tone 6 (USER 2026-11-05). The semantics — which tone each key writes,
/// how `c` resolves by input mode, what a repeated key does — live in the
/// engine (`engine/composing/src/telex.rs`); this side only decides which keys
/// reach it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ToneInputScheme {
    /// Digits type tones; `q w d f z x v y ;` pick candidates. The shipped
    /// default, and today's behaviour before the scheme existed.
    Standard,
    /// Letters type tones; `1`…`9` pick candidates.
    Telex,
}

impl ToneInputScheme {
    /// Lower-case Telex keys: `x v y d z w q` for tones 1/4 2/8 3 5 6 7 9 (`x` and `v`
    /// split their pair by the coda, engine-side), `c` for the affricate initial
    /// (`ts` / `ch`; `ch` then spells `tsh` / `chh`), `h` because only the engine
    /// can tell a slip after `tsh` / `chh`, `f` for the hyphen. Mirrors the engine's
    /// `composing::telex::TELEX_KEYS`
    /// — the two lists must name the same keys, or a key classified as Telex
    /// here would be ignored there. CROSS-PLATFORM INVARIANT — the one list for
    /// macOS, Windows and Linux.
    pub const TELEX_KEYS: &'static str = "vydwxqzfch";

    /// The keys that pick a candidate under this scheme.
    pub fn slot_key_set(self) -> CandidateSlotKeySet {
        match self {
            Self::Standard => CandidateSlotKeySet::BareKeys,
            Self::Telex => CandidateSlotKeySet::Digits,
        }
    }

    /// The picker row's i18n key. macOS keeps a Swift twin of the picker:
    /// `GeneralSettingsView.swift`.
    pub fn label_key(self) -> StringKey {
        match self {
            Self::Standard => StringKey::SettingsToneSchemeStandard,
            Self::Telex => StringKey::SettingsToneSchemeTelex,
        }
    }

    /// Whether `character` is a Telex key in either case. ASCII only: the
    /// engine reads the key as an ASCII letter, and a `v` from another
    /// script is document text.
    pub fn is_telex_key(character: char) -> bool {
        character.is_ascii() && Self::TELEX_KEYS.contains(character.to_ascii_lowercase())
    }

    /// Whether `character` goes to the engine when no composition is active.
    /// `c` and `h`, in either case, type an initial, so each begins a syllable
    /// the way any letter does. `f` types the `-` key there: the neutral-tone
    /// (khinsiann) marker, inserted as a literal while the engine stays idle,
    /// so an idle `f` can no longer type the letter itself. A tone letter has
    /// nothing to attach to when idle, and passes to the host like an idle
    /// digit.
    pub fn reaches_engine_when_idle(character: char) -> bool {
        matches!(character, 'c' | 'C' | 'h' | 'H' | 'f' | 'F')
    }
}

impl SettingChoice for ToneInputScheme {
    const ALL: &'static [Self] = &[Self::Standard, Self::Telex];
    const DEFAULT: Self = Self::Standard;
    fn raw(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Telex => "telex",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_slot_key_set_is_derived_from_the_scheme() {
        // trace: `slot_key_set` — Standard → `BareKeys`, Telex → `Digits`;
        // `DEFAULT` is Standard.
        assert_eq!(
            ToneInputScheme::Standard.slot_key_set(),
            CandidateSlotKeySet::BareKeys
        );
        assert_eq!(
            ToneInputScheme::Telex.slot_key_set(),
            CandidateSlotKeySet::Digits
        );
        assert_eq!(ToneInputScheme::DEFAULT, ToneInputScheme::Standard);
    }

    #[test]
    fn the_telex_keys_are_the_ten_letters_in_either_case() {
        for letter in "vydwxqzfch".chars() {
            assert!(ToneInputScheme::is_telex_key(letter), "{letter}");
            assert!(
                ToneInputScheme::is_telex_key(letter.to_ascii_uppercase()),
                "{letter}"
            );
        }
        for other in ['a', 't', 'g', '3', '-', ';', ' ', 'ｖ', 'ｚ'] {
            assert!(!ToneInputScheme::is_telex_key(other), "{other}");
        }
        // trace: `TELEX_KEYS` vs `BARE_KEY_ROW` — the eight Telex keys other
        // than `c` and `h` are the eight letters of the Standard slot row: what
        // one scheme types tones with, the other picks with.
        let slot_letters: std::collections::BTreeSet<char> = CandidateSlotKeySet::BARE_KEY_ROW
            .iter()
            .filter_map(|key| key.chars().next())
            .filter(char::is_ascii_alphabetic)
            .collect();
        assert_eq!(
            ToneInputScheme::TELEX_KEYS
                .chars()
                .filter(|letter| !matches!(letter, 'c' | 'h'))
                .collect::<std::collections::BTreeSet<_>>(),
            slot_letters
        );
        for letter in ['c', 'C', 'h', 'H', 'f', 'F'] {
            assert!(
                ToneInputScheme::reaches_engine_when_idle(letter),
                "{letter}"
            );
        }
        for letter in ['z', 'Z', 'v', 'x', 'y', 'd', 'w', 'q'] {
            assert!(
                !ToneInputScheme::reaches_engine_when_idle(letter),
                "{letter}"
            );
        }
    }

    #[test]
    fn raw_values_round_trip_and_are_the_mac_spellings() {
        // trace: `SettingChoice::raw` — the settings file speaks one
        // vocabulary on all three desktops (the spellings the Mac stores).
        assert_eq!(ToneInputScheme::Standard.raw(), "standard");
        assert_eq!(ToneInputScheme::Telex.raw(), "telex");
        for scheme in ToneInputScheme::ALL {
            assert_eq!(ToneInputScheme::from_raw(scheme.raw()), Some(*scheme));
        }
        assert_eq!(ToneInputScheme::from_raw("bareKeys"), None);
    }
}
