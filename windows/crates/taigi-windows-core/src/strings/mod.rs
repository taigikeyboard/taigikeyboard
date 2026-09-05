//! UI strings: the display-language roster, the resolver, and the positional
//! formatter the generated accessors call.
//!
//! `generated.rs` is written by the repo-root `make i18n` from `i18n/*.json`;
//! this file is the hand-written half. Resolution mirrors
//! `macos/Sources/TaigiInputMethodCore/Strings/StringResolver.swift:25-46`:
//! active language → Hanji → the key's own raw name.

// UI 字串 — 顯示語言名冊、解析器、以及產生程式碼呼叫的位置參數格式化器。

// Generator output, gated byte-for-byte by tools/i18n/check.py — rustfmt must not touch it.
#[rustfmt::skip]
mod generated;

pub use generated::StringKey;

/// The languages the UI can be shown in. `System` is a selection policy, not a
/// language: it resolves to one of the others through [`DisplayLanguage::resolve_automatic`]
/// and is never handed to a [`StringResolver`].
///
/// MIRROR: `tools/i18n/i18n_lib.py` `RUST_LANGUAGE_VARIANTS` and
/// `macos/Sources/TaigiInputMethodCore/Strings/DisplayLanguage.swift:19`.
// 介面語言;System 是「跟隨系統」的選擇策略,不是語言本身。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DisplayLanguage {
    System,
    Hanji,
    Tailo,
    Poj,
    Japanese,
    English,
}

impl DisplayLanguage {
    /// The persisted tag of a fresh install.
    pub const DEFAULT_TAG: &'static str = "system";

    /// The picker roster, in picker order — `System` first, then the production
    /// languages in the order iOS / Android / macOS list them.
    /// CROSS-PLATFORM INVARIANT (INVARIANT_DISPLAY_LANGUAGE_PRODUCTION_ROSTER) —
    /// mirrors `DisplayLanguage.swift:61-65` (`productionLanguages`).
    pub const PICKER: [DisplayLanguage; 6] = [
        DisplayLanguage::System,
        DisplayLanguage::Hanji,
        DisplayLanguage::English,
        DisplayLanguage::Japanese,
        DisplayLanguage::Tailo,
        DisplayLanguage::Poj,
    ];

    /// The persisted spelling. Two differ from the variant name (`ja`, `en`),
    /// which is why the mapping is written out.
    pub fn tag(self) -> &'static str {
        match self {
            DisplayLanguage::System => Self::DEFAULT_TAG,
            DisplayLanguage::Hanji => "hanji",
            DisplayLanguage::Tailo => "tailo",
            DisplayLanguage::Poj => "poj",
            DisplayLanguage::Japanese => "ja",
            DisplayLanguage::English => "en",
        }
    }

    /// A stored tag → language. An unknown or removed tag reads as Hanji, the
    /// base language every key is authored in (`DisplayLanguage.swift:94-96`).
    pub fn from_tag(tag: &str) -> Self {
        match tag {
            "system" => DisplayLanguage::System,
            "hanji" => DisplayLanguage::Hanji,
            "tailo" => DisplayLanguage::Tailo,
            "poj" => DisplayLanguage::Poj,
            "ja" => DisplayLanguage::Japanese,
            "en" => DisplayLanguage::English,
            _ => DisplayLanguage::Hanji,
        }
    }

    /// What `System` means on a machine whose UI language is `system_locale`
    /// (a BCP-47 tag such as `ja-JP`, `en-US`, `zh-TW`): Japanese and English
    /// by subtag, everything else the base language
    /// (`DisplayLanguage.swift:74-82`).
    pub fn resolve_automatic(system_locale: &str) -> Self {
        let language = system_locale
            .split(['-', '_'])
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        match language.as_str() {
            "ja" => DisplayLanguage::Japanese,
            "en" => DisplayLanguage::English,
            _ => DisplayLanguage::Hanji,
        }
    }

    /// The language strings are actually drawn in: `self`, unless `self` is
    /// `System`, which follows the machine (`DisplayLanguage.swift:88`).
    pub fn effective(self, system_locale: &str) -> Self {
        match self {
            DisplayLanguage::System => Self::resolve_automatic(system_locale),
            concrete => concrete,
        }
    }

    /// How the picker labels a concrete language: its own name in its own
    /// script (an endonym, not a translation). `System` has no endonym — the
    /// picker uses the `settingsDisplayLanguageAutomatic` key for it.
    pub fn endonym(self) -> Option<&'static str> {
        match self {
            DisplayLanguage::System => None,
            DisplayLanguage::Hanji => Some("台漢"),
            DisplayLanguage::Tailo => Some("Tâi-lô"),
            DisplayLanguage::Poj => Some("Pe̍h-ōe-jī"),
            DisplayLanguage::Japanese => Some("日本語"),
            DisplayLanguage::English => Some("English"),
        }
    }
}

/// Resolves keys for one concrete language.
///
/// Holds a language rather than reading a setting so every string one screen
/// draws comes from a single language; a resolver is rebuilt when the setting
/// changes, not consulted per key (`StringResolver.swift:11-21`).
// 針對一個具體語言解析字串;不會持有 System。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StringResolver {
    pub language: DisplayLanguage,
}

impl StringResolver {
    /// `System` is not a language strings exist in; a caller must resolve it
    /// through [`DisplayLanguage::effective`] first. Debug builds trap, release
    /// builds degrade to the base language (`StringResolver.swift:21`).
    pub fn new(language: DisplayLanguage) -> Self {
        debug_assert!(
            language != DisplayLanguage::System,
            "resolve DisplayLanguage::System through `effective` before building a resolver"
        );
        let language = if language == DisplayLanguage::System {
            DisplayLanguage::Hanji
        } else {
            language
        };
        Self { language }
    }

    /// Active language → Hanji → the key's own raw name. The last fallback is
    /// what a missing translation looks like on screen: a visible
    /// `i18n_desktop_generalTab`, never a blank.
    pub fn resolve(&self, key: StringKey) -> &'static str {
        generated::lookup(self.language, key)
            .or_else(|| generated::lookup(DisplayLanguage::Hanji, key))
            .unwrap_or_else(|| key.as_str())
    }

    /// The template behind a format key, filled with `args` by position.
    pub fn format(&self, key: StringKey, args: &[&dyn std::fmt::Display]) -> String {
        format_positional(self.resolve(key), args)
    }

    /// A literal template (the generated plural accessors pick one arm at
    /// runtime) filled with `args` by position.
    pub fn format_template(&self, template: &str, args: &[&dyn std::fmt::Display]) -> String {
        format_positional(template, args)
    }
}

/// Substitutes `{N}` slots (0-based) with `args[N]` and un-doubles `{{` /
/// `}}`. A slot with no argument, or a brace that opens no slot, is kept
/// verbatim — a malformed template renders visibly rather than panicking in
/// the middle of drawing a settings pane.
///
/// This is the only formatter the generated code calls; the `{N}` contract is
/// owned by `tools/i18n/i18n_lib.py` (`_lower_atom`, `target == "rust"`).
// 以位置參數填入 {N} 槽位;`{{`/`}}` 還原成單一大括號;格式錯誤照字面輸出不 panic。
pub fn format_positional(template: &str, args: &[&dyn std::fmt::Display]) -> String {
    let mut out = String::with_capacity(template.len() + 16);
    let mut chars = template.char_indices().peekable();
    while let Some((index, c)) = chars.next() {
        match c {
            '{' => {
                if chars.peek().map(|(_, next)| *next) == Some('{') {
                    chars.next();
                    out.push('{');
                    continue;
                }
                let rest = &template[index + 1..];
                let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
                if !digits.is_empty() && rest[digits.len()..].starts_with('}') {
                    match digits.parse::<usize>().ok().and_then(|n| args.get(n)) {
                        Some(arg) => {
                            out.push_str(&arg.to_string());
                            for _ in 0..=digits.len() {
                                chars.next();
                            }
                            continue;
                        }
                        None => out.push('{'),
                    }
                } else {
                    out.push('{');
                }
            }
            '}' => {
                if chars.peek().map(|(_, next)| *next) == Some('}') {
                    chars.next();
                }
                out.push('}');
            }
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positional_slots_fill_by_index_and_braces_undouble() {
        assert_eq!(
            format_positional("{1}: {0} 次 {{x}}", &[&3, &"why"]),
            "why: 3 次 {x}"
        );
    }

    #[test]
    fn missing_argument_and_stray_braces_render_verbatim() {
        assert_eq!(format_positional("{0} {5} {a} }", &[&"ok"]), "ok {5} {a} }");
    }

    #[test]
    fn resolve_falls_back_active_then_hanji_then_raw_name() {
        // trace: every production language is authored for every key, so the
        // active-language arm answers; the raw-name arm is reachable only for
        // a language map that lacks the key, which the generator forbids —
        // pinned through `as_str` so the fallback text is at least visible.
        let english = StringResolver::new(DisplayLanguage::English);
        assert_eq!(english.resolve(StringKey::CommonCancel), "Cancel");
        let hanji = StringResolver::new(DisplayLanguage::Hanji);
        assert_eq!(hanji.resolve(StringKey::CommonCancel), "取消");
        assert_eq!(StringKey::CommonCancel.as_str(), "i18n_common_cancel");
    }

    #[test]
    fn format_key_substitutes_in_base_text_order() {
        let english = StringResolver::new(DisplayLanguage::English);
        let rendered = english.desktop_update_current_version_label("3.6.6");
        assert!(rendered.contains("3.6.6"), "{rendered}");
        assert!(!rendered.contains("{0}"), "{rendered}");
    }

    #[test]
    fn automatic_resolution_by_locale_subtag() {
        assert_eq!(
            DisplayLanguage::resolve_automatic("ja-JP"),
            DisplayLanguage::Japanese
        );
        assert_eq!(
            DisplayLanguage::resolve_automatic("en_US"),
            DisplayLanguage::English
        );
        assert_eq!(
            DisplayLanguage::resolve_automatic("zh-TW"),
            DisplayLanguage::Hanji
        );
        assert_eq!(
            DisplayLanguage::resolve_automatic(""),
            DisplayLanguage::Hanji
        );
        assert_eq!(
            DisplayLanguage::Poj.effective("en-US"),
            DisplayLanguage::Poj,
            "a concrete choice never follows the machine"
        );
    }

    #[test]
    fn tags_round_trip_and_unknown_reads_as_hanji() {
        for language in DisplayLanguage::PICKER {
            assert_eq!(DisplayLanguage::from_tag(language.tag()), language);
        }
        assert_eq!(DisplayLanguage::from_tag("klingon"), DisplayLanguage::Hanji);
        assert_eq!(DisplayLanguage::DEFAULT_TAG, DisplayLanguage::System.tag());
    }

    #[test]
    fn every_key_has_every_production_language() {
        // The generator enforces completeness at the CLI boundary; this pins the
        // same property on the compiled tables so a hand-edited generated file
        // cannot silently fall back to Hanji for one language.
        for language in DisplayLanguage::PICKER.into_iter().skip(1) {
            let resolver = StringResolver::new(language);
            assert_ne!(
                resolver.resolve(StringKey::DesktopGeneralTab),
                StringKey::DesktopGeneralTab.as_str(),
                "{language:?} lacks DesktopGeneralTab"
            );
        }
    }
}
