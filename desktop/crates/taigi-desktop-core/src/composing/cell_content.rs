//! What one candidate's cell shows, and which of its scripts a commit asks
//! for; the document text a commit writes is the engine's
//! (`composing::commit_text`). macOS keeps a Swift twin of the cell value:
//! `CandidateCellContent.swift`.

use crate::engine::{self, ContinuousCandidate};
use crate::settings::{CandidateDisplayMode, EngineSettings, InputMode};

/// Which of a candidate's two scripts a commit writes. RELATIVE to the
/// output settings, never absolute: `Primary` is what Enter writes,
/// `Alternate` the other one (what Output the Other Script writes). Sent as the engine's
/// `CommitScript` LEAD / OTHER.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandidateScript {
    Primary,
    Alternate,
}

impl CandidateScript {
    /// The other script — what Output the Other Script writes relative to a
    /// cell's own.
    pub fn flipped(self) -> Self {
        match self {
            Self::Primary => Self::Alternate,
            Self::Alternate => Self::Primary,
        }
    }
}

/// One candidate as the window renders it — both scripts, in the order the
/// user's swap setting puts them. A Taigi candidate is a `(Hanji, romanization)`
/// pair (Core Principle #7), and showing only one makes several read
/// identically. Display only; what a commit writes is the engine's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CandidateCellContent {
    /// The script this cell leads with.
    pub text: String,
    /// The other script, or `None` for a candidate that has only one. An
    /// empty annotation is normalized to `None` so the cell reserves no
    /// width for it.
    pub annotation: Option<String>,
}

impl CandidateCellContent {
    pub fn new(text: impl Into<String>, annotation: Option<String>) -> Self {
        Self {
            text: text.into(),
            annotation: annotation.filter(|annotation| !annotation.is_empty()),
        }
    }

    /// CROSS-PLATFORM INVARIANT — mirrors
    /// `ios/.../TaigiAutocompleteService.swift` `buildContinuousSuggestions` (primary =
    /// romanization, secondary = Hanji) and the swap flip in
    /// `ios/.../CandidateCellHelper.swift` `suggestionToHandle`; arm order is the
    /// same on every platform. Serves Pairing and Romanization Only; Combined's split cells
    /// are built by [`super::presentation`].
    ///
    /// Under TPS a cell is one script whatever Candidate Display says: the
    /// Hanji, or for a candidate with none its reading in TPS — as on mobile
    /// (`ios/.../CandidateCellHelper.swift`), and what the engine's commit
    /// writes for it.
    pub fn cell(candidate: &ContinuousCandidate, settings: &EngineSettings) -> Self {
        if settings.input_mode == InputMode::Tps {
            let text = match candidate.nonempty_hanji() {
                Some(hanji) => hanji.to_owned(),
                None => engine::tl_display_to_tps(&candidate.roman, engine::TPS_OR_MAPS_TO_ER)
                    .unwrap_or_else(|| candidate.roman.clone()),
            };
            return Self::new(text, None);
        }
        match candidate.nonempty_hanji() {
            None => Self::new(candidate.roman.clone(), None),
            Some(_) if settings.candidate_display_mode == CandidateDisplayMode::RomanOnly => {
                Self::new(candidate.roman.clone(), None)
            }
            Some(hanji) => {
                if settings.is_hanji_first {
                    Self::new(hanji, Some(candidate.roman.clone()))
                } else {
                    Self::new(candidate.roman.clone(), Some(hanji.to_owned()))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::test_support::candidate;

    fn settings(swapped: bool) -> EngineSettings {
        EngineSettings {
            is_hanji_first: swapped,
            ..EngineSettings::default()
        }
    }

    #[test]
    fn the_cell_leads_with_the_script_the_swap_names() {
        // trace: `cell` — roman-first annotates the hanji, hanji-first the
        // roman, verbatim.
        let c = candidate("kau--lâng", Some("交--人"), 0);
        assert_eq!(
            CandidateCellContent::cell(&c, &settings(false)),
            CandidateCellContent::new("kau--lâng", Some("交--人".to_owned()))
        );
        assert_eq!(
            CandidateCellContent::cell(&c, &settings(true)),
            CandidateCellContent::new("交--人", Some("kau--lâng".to_owned()))
        );
    }

    #[test]
    fn a_one_script_candidate_shows_its_romanization_with_no_annotation() {
        for c in [candidate("guá", None, 0), candidate("guá", Some(""), 0)] {
            for swapped in [false, true] {
                let cell = CandidateCellContent::cell(&c, &settings(swapped));
                assert_eq!(cell.text, "guá");
                assert_eq!(cell.annotation, None, "swapped={swapped}");
            }
        }
    }

    #[test]
    fn roman_only_mode_shows_the_romanization_alone() {
        // trace: hanji present, mode=RomanOnly → cell = roman with no
        // annotation whichever way the swap points; Side-by-side keeps it.
        let c = candidate("tâi-gí", Some("台語"), 0);
        for swapped in [false, true] {
            let settings = EngineSettings {
                is_hanji_first: swapped,
                candidate_display_mode: CandidateDisplayMode::RomanOnly,
                ..EngineSettings::default()
            };
            let cell = CandidateCellContent::cell(&c, &settings);
            assert_eq!(cell.text, "tâi-gí");
            assert_eq!(cell.annotation, None);
        }
        let cell = CandidateCellContent::cell(&c, &settings(false));
        assert_eq!(cell.annotation.as_deref(), Some("台語"));
    }

    #[test]
    fn under_tps_a_cell_is_the_hanji_alone_or_the_reading_in_tps() {
        // trace: `cell` TPS arm — Hanji with no annotation, whatever the
        // swap and display mode; a Hanji-less `ka` is ㄍㄚ (tl_display_to_tps).
        let tps = |display| EngineSettings {
            input_mode: InputMode::Tps,
            candidate_display_mode: display,
            ..EngineSettings::default()
        };
        let hanji = candidate("ka", Some("家"), 0);
        for display in [
            CandidateDisplayMode::SideBySide,
            CandidateDisplayMode::RomanOnly,
            CandidateDisplayMode::Combined,
        ] {
            assert_eq!(
                CandidateCellContent::cell(&hanji, &tps(display)),
                CandidateCellContent::new("家", None)
            );
        }
        let bare = candidate("ka", None, 0);
        assert_eq!(
            CandidateCellContent::cell(&bare, &tps(CandidateDisplayMode::SideBySide)),
            CandidateCellContent::new("ㄍㄚ", None)
        );
    }
}
