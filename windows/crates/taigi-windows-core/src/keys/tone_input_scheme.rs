//! Which keys type a tone — and, by the same choice, which keys pick a
//! candidate. Port of `ToneInputScheme.swift`.

use super::slot_key_set::CandidateSlotKeySet;
use crate::settings::SettingChoice;
use crate::strings::StringKey;

/// Which keys type a tone — the half of the key contract that decides, in
/// turn, which keys pick a candidate.
///
/// Standard and Telex are the same eight letters and the same nine digits,
/// swapped (USER 2026-09-08). Under `Standard` a digit is the TL/POJ tone
/// marker (`tai5`), so the letters no syllable spells are free to pick
/// candidates; under `Telex` those letters carry the tones, so the digits
/// are free to pick. Neither half is chosen on its own: the slot key set is
/// derived here (`slot_key_set`) rather than stored, so the two can never
/// disagree about who owns `v` or `3`.
///
/// The letter table is the kahiok scheme (madmaxieee/taigi-telex) minus its
/// `c` → `tsh` key, because POJ spells `ch` / `chh` with `c`. Tone 6 has no
/// free letter and is not offered. The semantics — which tone each key
/// writes, how `z` resolves by input mode, what a repeated key does — live in
/// the engine (`engine/composing/src/telex.rs`); this side only decides which
/// keys reach it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ToneInputScheme {
    /// Digits type tones; `q w d f z x v y ;` pick candidates. The shipped
    /// default, and today's behaviour before the scheme existed.
    Standard,
    /// Letters type tones; `1`…`9` pick candidates.
    Telex,
}

impl ToneInputScheme {
    /// Lower-case Telex keys: `v y d w x q` for tones 2 3 5 7 8 9, `z` for
    /// the affricate initial (`ts` / `ch`; `zh` then spells `tsh` / `chh`),
    /// `f` for the hyphen. Mirrors the engine's `composing::telex::TELEX_KEYS`
    /// — the two lists must name the same keys, or a key classified as Telex
    /// here would be ignored there. CROSS-PLATFORM INVARIANT — mirrors
    /// `ToneInputScheme.swift` `telexKeys`.
    pub const TELEX_KEYS: &'static str = "vydwxqzf";

    /// The keys that pick a candidate under this scheme.
    pub fn slot_key_set(self) -> CandidateSlotKeySet {
        match self {
            Self::Standard => CandidateSlotKeySet::BareKeys,
            Self::Telex => CandidateSlotKeySet::Digits,
        }
    }

    /// The picker row's i18n key (`GeneralSettingsView.swift`).
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

    /// Whether `character` may start a composition on its own. Only `z` /
    /// `Z`: it types an initial, so it begins a syllable the way any letter
    /// does. A tone letter or `f` has nothing to attach to when idle, and
    /// passes to the host like an idle digit.
    pub fn starts_composition(character: char) -> bool {
        matches!(character, 'z' | 'Z')
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
        // trace: ToneInputScheme.swift `slotKeySet` — standard → bareKeys,
        // telex → digits; the default is Standard.
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
    fn the_telex_keys_are_the_eight_letters_in_either_case() {
        for letter in "vydwxqzf".chars() {
            assert!(ToneInputScheme::is_telex_key(letter), "{letter}");
            assert!(
                ToneInputScheme::is_telex_key(letter.to_ascii_uppercase()),
                "{letter}"
            );
        }
        for other in ['a', 't', 'c', '3', '-', ';', 'ｖ'] {
            assert!(!ToneInputScheme::is_telex_key(other), "{other}");
        }
        assert!(ToneInputScheme::starts_composition('z'));
        assert!(ToneInputScheme::starts_composition('Z'));
        assert!(!ToneInputScheme::starts_composition('v'));
        assert!(!ToneInputScheme::starts_composition('f'));
    }

    #[test]
    fn raw_values_round_trip_and_are_the_mac_spellings() {
        // trace: ToneInputScheme.swift `String` raw values — the settings
        // file speaks one vocabulary on both desktops.
        assert_eq!(ToneInputScheme::Standard.raw(), "standard");
        assert_eq!(ToneInputScheme::Telex.raw(), "telex");
        for scheme in ToneInputScheme::ALL {
            assert_eq!(ToneInputScheme::from_raw(scheme.raw()), Some(*scheme));
        }
        assert_eq!(ToneInputScheme::from_raw("bareKeys"), None);
    }
}
