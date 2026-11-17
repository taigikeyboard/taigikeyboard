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
    /// identity has to survive on the survivor. For Hanji with Romanization the collapse is
    /// [`split_single_script_cells`], which keeps the FIRST roman cell — the
    /// slot-0 §34 literal — as the survivor. Pairing (and the
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

    /// Whether candidate lists go out as §42 split cells
    /// ([`split_single_script_cells`]): Hanji with Romanization, off the TPS
    /// layout (TPS is Hanji-first by construction and ignores Candidate Display).
    pub fn splits_single_script_cells(&self) -> bool {
        self.candidate_display_mode() == engine::CandidateDisplayMode::Combined
            && !self.is_tps_layout()
    }

    /// Whether the composition renders Hanji first: the swap, or the TPS
    /// layout (which shows Hanji / Bopomofo, never the romanization).
    pub fn renders_hanji_first(&self) -> bool {
        self.is_hanji_first || self.is_tps_layout()
    }

    /// Whether each candidate cell commits plain Hanji alone (without bracketed
    /// phonetic annotation): TPS layout or TPS-typed buffer with brackets off,
    /// or Hanji-first paired display (`SideBySide`) with brackets off.
    /// `RomanOnly` and `Combined` (`is_single_script_display()`) answer `false`
    /// on roman layouts because `RomanOnly` commits romanization and `Combined`
    /// splits Hanji and romanization into separate cells.
    pub fn commits_plain_hanji(&self, is_tps_buffer: bool) -> bool {
        !self.output_both_scripts
            && ((is_tps_buffer || self.is_tps_layout())
                || (self.renders_hanji_first() && !self.is_single_script_display()))
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

/// The §42 split (Candidate Display = Hanji with Romanization): each row
/// becomes a Hanji cell (when `hanji_of` is non-empty) then a romanization
/// cell (when `roman_of` is non-empty), each a clone of the row marked by
/// `mark` with its [`engine::CellScript`]. Each script dedupes on the text its
/// cell shows, first-seen wins: a one-script cell carries nothing that could
/// tell it from an earlier cell reading the same (USER 2026-09-03: "the same
/// Hanji or romanization must not appear twice"), so 重/tîng and 重/tāng draw
/// ONE 重 cell and keep both roman cells, and the §34 literal leading the list
/// absorbs a later row's same roman. The Continuous list
/// (`composing::requests`) and NextWord predictions (`nextword::filter`)
/// differ only in the projections.
pub fn split_single_script_cells<T: Clone>(
    rows: Vec<T>,
    hanji_of: impl Fn(&T) -> &str,
    roman_of: impl Fn(&T) -> &str,
    mark: impl Fn(&mut T, engine::CellScript),
) -> Vec<T> {
    let mut cells = Vec::with_capacity(rows.len() * 2);
    let mut seen_hanji = std::collections::HashSet::new();
    let mut seen_roman = std::collections::HashSet::new();
    for row in rows {
        let hanji = hanji_of(&row);
        if !hanji.is_empty() && seen_hanji.insert(hanji.to_owned()) {
            let mut cell = row.clone();
            mark(&mut cell, engine::CellScript::Hanji);
            cells.push(cell);
        }
        let roman = roman_of(&row);
        if !roman.is_empty() && seen_roman.insert(roman.to_owned()) {
            let mut cell = row;
            mark(&mut cell, engine::CellScript::Roman);
            cells.push(cell);
        }
    }
    cells
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

    #[test]
    fn commits_plain_hanji_requires_brackets_off_and_paired_hanji_or_tps() {
        use super::engine::CandidateDisplayMode;
        let mk =
            |mode: &str, swapped: bool, brackets: bool, disp: CandidateDisplayMode| AppConfig {
                input_mode: mode.to_owned(),
                is_hanji_first: swapped,
                output_both_scripts: brackets,
                candidate_display_mode: disp as i32,
                ..AppConfig::default()
            };
        assert!(mk("tl", true, false, CandidateDisplayMode::SideBySide).commits_plain_hanji(false));
        assert!(!mk("tl", true, true, CandidateDisplayMode::SideBySide).commits_plain_hanji(false));
        assert!(
            !mk("tl", false, false, CandidateDisplayMode::SideBySide).commits_plain_hanji(false)
        );
        assert!(!mk("tl", true, false, CandidateDisplayMode::Combined).commits_plain_hanji(false));
        assert!(!mk("tl", true, false, CandidateDisplayMode::RomanOnly).commits_plain_hanji(false));
        assert!(mk("tps", false, false, CandidateDisplayMode::Combined).commits_plain_hanji(false));
        assert!(mk("tl", false, false, CandidateDisplayMode::SideBySide).commits_plain_hanji(true));
        assert!(!mk("tps", false, true, CandidateDisplayMode::SideBySide).commits_plain_hanji(true));
    }

    #[test]
    fn split_lists_hanji_then_roman_cells_deduped_per_script_first_seen() {
        use super::engine::CellScript;
        // trace: literal tâi (hanji-less) → one roman cell and absorbs 台's
        // roman; 重 tîng / 重 tāng draw ONE 重 cell and keep both roman cells;
        // 食 / 𤆬 keep both hanji cells and share one tsia̍h; a repeated 食
        // (another span) adds nothing.
        let rows = vec![
            ("", "tâi", 0),
            ("台", "tâi", 1),
            ("重", "tîng", 2),
            ("重", "tāng", 3),
            ("食", "tsia̍h", 4),
            ("𤆬", "tsia̍h", 5),
            ("食", "tsia̍h", 6),
        ];
        let cells = super::split_single_script_cells(
            rows.into_iter()
                .map(|(hanji, roman, id)| (hanji, roman, id, CellScript::Paired))
                .collect(),
            |row| row.0,
            |row| row.1,
            |row, script| row.3 = script,
        );
        let shown: Vec<(usize, CellScript)> = cells.iter().map(|c| (c.2, c.3)).collect();
        assert_eq!(
            shown,
            vec![
                (0, CellScript::Roman),
                (1, CellScript::Hanji),
                (2, CellScript::Hanji),
                (2, CellScript::Roman),
                (3, CellScript::Roman),
                (4, CellScript::Hanji),
                (4, CellScript::Roman),
                (5, CellScript::Hanji),
            ]
        );
    }
}
