//! Whether the keys reaching this input method compose Taigi or go straight
//! to the document as English — the Windows CJK 中/英 mode, toggled by a
//! Shift tap (`ShiftTapTracker`).

// 中文: 中/英 模式 —— 台語組字 或 英文直通。Windows 慣例,用 Shift 短按切換。

use crate::strings::StringKey;

/// The mode the Shift tap switches between. Deliberately NOT called an input
/// mode: [`crate::settings::InputMode`] is the TL / POJ romanization choice,
/// and the two are independent — English mode suspends composing whichever
/// romanization is selected, and the romanization is still there on the way
/// back.
///
/// This is a transient mode, never persisted: a fresh activation of the text
/// service starts in [`LanguageMode::Taigi`], the way Windows CJK IMEs start
/// in 中文 (新酷音 `chewing_ime.py:183`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LanguageMode {
    #[default]
    Taigi,
    English,
}

impl LanguageMode {
    /// The other mode — what a Shift tap switches to.
    pub fn toggled(self) -> Self {
        match self {
            Self::Taigi => Self::English,
            Self::English => Self::Taigi,
        }
    }

    /// True while keys belong to the document rather than to a composition.
    pub fn is_english(self) -> bool {
        matches!(self, Self::English)
    }

    /// The name the mode flash shows after a switch, as the romanization and
    /// candidate-display switches show theirs (`session.rs` `perform_global`).
    pub fn flash_label_key(self) -> StringKey {
        match self {
            Self::Taigi => StringKey::SettingsTaigiMode,
            Self::English => StringKey::SettingsEnglishMode,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_service_starts_in_taigi() {
        assert_eq!(LanguageMode::default(), LanguageMode::Taigi);
    }

    #[test]
    fn toggling_twice_returns_to_the_starting_mode() {
        let mode = LanguageMode::Taigi;
        assert_eq!(mode.toggled(), LanguageMode::English);
        assert_eq!(mode.toggled().toggled(), mode);
    }

    #[test]
    fn only_english_suspends_composing() {
        assert!(!LanguageMode::Taigi.is_english());
        assert!(LanguageMode::English.is_english());
    }

    #[test]
    fn each_mode_flashes_its_own_name() {
        // Named, not merely different: swapping the two arms is the one bug a
        // two-way mapping can have, and inequality would not see it.
        assert_eq!(
            LanguageMode::Taigi.flash_label_key(),
            StringKey::SettingsTaigiMode
        );
        assert_eq!(
            LanguageMode::English.flash_label_key(),
            StringKey::SettingsEnglishMode
        );
    }
}
