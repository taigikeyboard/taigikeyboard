//! Generated protobuf bindings for the Taigi engine wire types.
//!
//! Hand-written shell crate: every type is `include!`'d from the prost-build
//! output (`$OUT_DIR/taigi.engine.rs`), regenerated each build by `build.rs`
//! from `engine/protos/proto/{envelope,phonetics,composing,lexicon,nextword,
//! case}.proto`. Platform-side bindings (iOS `.pb.swift`, Android `.java`)
//! are NOT generated here — see `engine/scripts/gen-platform-protos.sh`.
//!
//! `clippy::all` + `clippy::pedantic` are silenced because the included
//! prost output lints noisily and is regenerated on every build.

#![allow(clippy::all, clippy::pedantic)]

pub mod engine {
    include!(concat!(env!("OUT_DIR"), "/taigi.engine.rs"));
}

impl engine::AppConfig {
    /// Whether candidate cells render romanization only (Candidate Display = Romanization Only).
    ///
    /// The single normalisation point for `candidate_display_mode`: the
    /// proto3 default `0`, an unknown value from a newer platform, and
    /// `SIDE_BY_SIDE` all answer `false` (legacy behaviour), so the two
    /// engine readers can never drift on the fallback.
    pub fn is_roman_only_display(&self) -> bool {
        // prost's accessor already maps an unknown value to `Unspecified`.
        self.candidate_display_mode() == engine::CandidateDisplayMode::RomanOnly
    }

    /// Whether every candidate cell shows ONE script (Romanization Only, or Hanji with Romanization's
    /// split cells) — the displays under which a cell that reads like an
    /// earlier one is collapsed into it (§42 / §44), so the collapsed row's
    /// identity has to survive on the survivor. For Hanji with Romanization the collapse is the
    /// platform's (§42 split) and the engine relies on it keeping the FIRST
    /// roman cell — the slot-0 §34 literal — as the survivor. Pairing (and the
    /// same fallbacks as [`Self::is_roman_only_display`]) answer `false`.
    pub fn is_single_script_display(&self) -> bool {
        matches!(
            self.candidate_display_mode(),
            engine::CandidateDisplayMode::RomanOnly | engine::CandidateDisplayMode::Combined
        )
    }

    /// Whether the keyboard is on the TPS (Bopomofo) layout: `input_mode` is
    /// `"tps"` (either spelling `phonetics::api::parse_input_mode` accepts).
    /// The engine ships inside each app, so this is the only TPS wire; the
    /// two readers below apply the TPS fold to the stored swap / separator.
    pub fn is_tps_layout(&self) -> bool {
        matches!(self.input_mode.as_str(), "tps" | "TPS")
    }

    /// Whether the composition renders Hanji first: the swap, or the TPS
    /// layout (which shows Hanji / Bopomofo, never the romanization).
    pub fn renders_hanji_first(&self) -> bool {
        self.is_hanji_first || self.is_tps_layout()
    }

    /// What replaces the dictionary's syllable hyphen in the rendered
    /// romanization (Syllable Separator, §49): `None` keeps the romanization
    /// as it is — Hyphen, the proto default, an unknown value, and always the
    /// TPS layout, whose platform re-splits the candidate `roman` on `-` to
    /// render Bopomofo; `Some("")` drops it (None); `Some(" ")` writes a space
    /// (Space). Fed to `phonetics::api::syllable_joiner_display`.
    pub fn rendered_syllable_joiner(&self) -> Option<&'static str> {
        if self.is_tps_layout() {
            return None;
        }
        // prost's accessor already maps an unknown value to `Unspecified`.
        match self.syllable_separator() {
            engine::SyllableSeparator::None => Some(""),
            engine::SyllableSeparator::Space => Some(" "),
            engine::SyllableSeparator::Unspecified | engine::SyllableSeparator::Hyphen => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::engine::{AppConfig, SyllableSeparator};

    fn config(input_mode: &str, swapped: bool) -> AppConfig {
        AppConfig {
            input_mode: input_mode.to_owned(),
            is_hanji_first: swapped,
            ..AppConfig::default()
        }
    }

    fn separated(input_mode: &str, separator: SyllableSeparator) -> AppConfig {
        AppConfig {
            syllable_separator: separator as i32,
            ..config(input_mode, false)
        }
    }

    #[test]
    fn tps_layout_is_the_tps_input_mode_only() {
        assert!(config("tps", false).is_tps_layout());
        assert!(config("TPS", false).is_tps_layout());
        for mode in ["tl", "poj", "english", "", "Tps"] {
            assert!(!config(mode, false).is_tps_layout(), "{mode:?}");
        }
    }

    #[test]
    fn tps_layout_renders_hanji_first_whatever_the_stored_swap() {
        assert!(config("tps", false).renders_hanji_first());
        assert!(config("tps", true).renders_hanji_first());
        assert!(config("tl", true).renders_hanji_first());
        assert!(!config("tl", false).renders_hanji_first());
        assert!(!config("poj", false).renders_hanji_first());
    }

    #[test]
    fn syllable_joiner_follows_the_separator_off_the_tps_layout() {
        for mode in ["tl", "poj"] {
            assert_eq!(
                separated(mode, SyllableSeparator::None).rendered_syllable_joiner(),
                Some("")
            );
            assert_eq!(
                separated(mode, SyllableSeparator::Space).rendered_syllable_joiner(),
                Some(" ")
            );
            assert_eq!(
                separated(mode, SyllableSeparator::Hyphen).rendered_syllable_joiner(),
                None
            );
            assert_eq!(
                config(mode, false).rendered_syllable_joiner(),
                None,
                "proto default"
            );
        }
        let mut unknown = config("tl", false);
        unknown.syllable_separator = 99;
        assert_eq!(unknown.rendered_syllable_joiner(), None, "unknown value");
    }

    #[test]
    fn tps_layout_never_rewrites_the_syllable_hyphen() {
        for separator in [SyllableSeparator::None, SyllableSeparator::Space] {
            assert_eq!(separated("tps", separator).rendered_syllable_joiner(), None);
        }
    }
}
