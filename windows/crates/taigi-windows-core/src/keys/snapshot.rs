//! The parts of a key event a composing decision is made from.

// 中文: 一個按鍵事件中,分類器需要的欄位快照。

/// The four chording modifiers. Caps Lock, Num Lock and the extended-key
/// flag are deliberately not represented: they say how a key was reached,
/// not which key it is, so the shell drops them before building a snapshot
/// (`ComposingKeyChord.swift:22-27`).
///
/// Windows' modifier set differs from the Mac's, and the mapping is by ROLE:
/// `control` and `alt` and `win` are the chords the host owns (the Mac's
/// ⌘/⌃/⌥), `shift` is the one that only changes how a character is typed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct KeyModifiers {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
    pub win: bool,
}

impl KeyModifiers {
    pub const NONE: Self = Self {
        shift: false,
        control: false,
        alt: false,
        win: false,
    };
    pub const SHIFT: Self = Self {
        shift: true,
        ..Self::NONE
    };
    pub const CONTROL: Self = Self {
        control: true,
        ..Self::NONE
    };
    pub const ALT: Self = Self {
        alt: true,
        ..Self::NONE
    };
    pub const WIN: Self = Self {
        win: true,
        ..Self::NONE
    };

    /// Whether any of the modifiers the host owns is held — the Mac's
    /// `hostChords` (`ComposingKeyIntent.swift:141`).
    pub fn has_host_chord(self) -> bool {
        self.control || self.alt || self.win
    }

    pub fn is_empty(self) -> bool {
        self == Self::NONE
    }

    pub const fn with(self, other: Self) -> Self {
        Self {
            shift: self.shift || other.shift,
            control: self.control || other.control,
            alt: self.alt || other.alt,
            win: self.win || other.win,
        }
    }
}

/// A key the platform names that this input method binds to candidate
/// navigation. Its own type rather than a virtual-key code so the policy
/// stays in [`super::ComposingKeyIntent`] and the shell only extracts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NavigationKey {
    LeftArrow,
    RightArrow,
    UpArrow,
    DownArrow,
    PageUp,
    PageDown,
}

/// The parts of a key event a composing decision is made from — a value
/// rather than the event, so the classification can be tested without one
/// (`ComposingKeyIntent.swift:39-91`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeyEventSnapshot {
    /// What the key typed with the modifiers held. `None` for a key that
    /// types nothing (an arrow, a function key).
    pub characters: Option<String>,
    /// What the same key would have typed with no modifiers held. Carried
    /// because Control rewrites the characters of the digits it is chorded
    /// with — Ctrl+2 … Ctrl+8 arrive as NUL, ESC, FS, GS, RS, US, DEL — so
    /// `characters` would read Ctrl+3 as an Escape.
    pub characters_ignoring_modifiers: Option<String>,
    /// The virtual-key code, for the shifted-digit chord alone: Shift+3
    /// types `#` on a US layout and even `characters_ignoring_modifiers`
    /// keeps Shift, so the number row's codes are what still say which key
    /// was pressed. `None` for a snapshot built without an event.
    pub key_code: Option<u16>,
    pub modifiers: KeyModifiers,
    /// Whether the platform has a name for this key rather than a character
    /// (arrows, function keys, Home/End, Insert, forward Delete, …).
    pub is_named_special_key: bool,
    /// The navigation key this event is, if it is one of the six.
    pub navigation_key: Option<NavigationKey>,
}

impl KeyEventSnapshot {
    /// A key that types `characters` — the common case, with
    /// `characters_ignoring_modifiers` the same string.
    pub fn text(characters: &str, modifiers: KeyModifiers) -> Self {
        Self {
            characters: Some(characters.to_owned()),
            characters_ignoring_modifiers: Some(characters.to_owned()),
            modifiers,
            ..Self::default()
        }
    }

    /// A key that types `characters` under `modifiers` but `unmodified`
    /// with none held (`Ctrl+3` arrives as `"\u{1B}"` / `"3"`).
    pub fn chord(characters: Option<&str>, unmodified: &str, modifiers: KeyModifiers) -> Self {
        Self {
            characters: characters.map(str::to_owned),
            characters_ignoring_modifiers: Some(unmodified.to_owned()),
            modifiers,
            ..Self::default()
        }
    }

    /// One of the six navigation keys.
    pub fn navigation(key: NavigationKey, modifiers: KeyModifiers) -> Self {
        Self {
            modifiers,
            is_named_special_key: true,
            navigation_key: Some(key),
            ..Self::default()
        }
    }

    /// A key the platform names that is not navigation (F5, Home, …).
    pub fn named_special(modifiers: KeyModifiers) -> Self {
        Self {
            modifiers,
            is_named_special_key: true,
            ..Self::default()
        }
    }

    pub fn with_key_code(mut self, key_code: u16) -> Self {
        self.key_code = Some(key_code);
        self
    }

    /// The unmodified characters, falling back to the modified ones for a
    /// snapshot that carries only one (`charactersIgnoringModifiers ?? characters`).
    pub fn unmodified_characters(&self) -> Option<&str> {
        self.characters_ignoring_modifiers
            .as_deref()
            .or(self.characters.as_deref())
    }
}
