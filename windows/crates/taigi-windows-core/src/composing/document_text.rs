//! What one candidate writes into the document, and what its cell shows.
//! Port of `CandidateDocumentText.swift`, `CandidateCellContent.swift`,
//! `CandidateScript.swift`.

// 中文: 候選送出時寫入文件的字串,與候選格顯示的兩種文字;身分鍵另有其人(漢字, canonical TL)。

use crate::engine::ContinuousCandidate;
use crate::settings::{CandidateDisplayMode, EngineSettings};

/// Which of a candidate's two scripts a commit writes. RELATIVE to the
/// output settings, never absolute: `Primary` is what Enter writes,
/// `Alternate` the other one (what Space writes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandidateScript {
    Primary,
    Alternate,
}

/// One candidate as the window renders it — both scripts, in the order the
/// user's swap setting puts them. A Taigi candidate is a `(漢字, 羅馬字)`
/// pair (Core Principle #7), and showing only one makes several read
/// identically. Display only; what a commit writes is [`document_text`].
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
    /// `ios/.../TaigiAutocompleteService.swift:150-162` (primary =
    /// romanization, secondary = Hanji) and the swap flip. Arm order is the
    /// same on every platform: hanji-less → roman-only mode → swap. A
    /// roman-only cell carries no annotation, which is what makes Space
    /// answer `Ignored` on it (`alternate_text`).
    pub fn cell(candidate: &ContinuousCandidate, settings: &EngineSettings) -> Self {
        match candidate.hanji.as_deref().filter(|hanji| !hanji.is_empty()) {
            None => Self::new(candidate.roman.clone(), None),
            Some(_) if settings.candidate_display_mode == CandidateDisplayMode::RomanOnly => {
                Self::new(candidate.roman.clone(), None)
            }
            Some(hanji) => {
                if settings.is_translate_swapped {
                    Self::new(hanji, Some(candidate.roman.clone()))
                } else {
                    Self::new(candidate.roman.clone(), Some(hanji.to_owned()))
                }
            }
        }
    }
}

/// What committing `candidate` writes into the document under `settings`.
/// CROSS-PLATFORM INVARIANT — mirrors
/// `ios/.../ActionHandler+Suggestions.swift:218`; the TPS branch is absent
/// because the desktop ships TL and POJ only.
pub fn document_text(candidate: &ContinuousCandidate, settings: &EngineSettings) -> String {
    // A romanization-only candidate is marked by an absent hanji, but a
    // producer that emitted "" means the same thing — rendering "guá ()"
    // for it would be a visible defect.
    let Some(hanji) = candidate.hanji.as_deref().filter(|hanji| !hanji.is_empty()) else {
        return candidate.roman.clone();
    };
    if settings.is_output_both_scripts {
        if settings.is_translate_swapped {
            format!("{hanji} ({})", candidate.roman)
        } else {
            format!("{} ({hanji})", candidate.roman)
        }
    } else if settings.is_translate_swapped {
        hanji.to_owned()
    } else {
        candidate.roman.clone()
    }
}

/// The script `document_text` does NOT lead with, or `None` when the
/// candidate has only one. What Space commits — read off the cell rather than
/// resolved again, so Space writes exactly the script the user can already see
/// under the primary one, and `is_output_both_scripts` is not consulted.
pub fn alternate_text(
    candidate: &ContinuousCandidate,
    settings: &EngineSettings,
) -> Option<String> {
    CandidateCellContent::cell(candidate, settings).annotation
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::CandidateMode;

    fn candidate(roman: &str, hanji: Option<&str>) -> ContinuousCandidate {
        ContinuousCandidate {
            consumed_span_start: 0,
            consumed_span_end: 0,
            syllable_count: 1,
            display_text: hanji.unwrap_or(roman).to_owned(),
            score: 0.0,
            form: 1,
            mode: CandidateMode::Unspecified,
            roman: roman.to_owned(),
            hanji: hanji.map(str::to_owned),
            canonical_tl: roman.to_owned(),
        }
    }

    fn settings(swapped: bool, both: bool) -> EngineSettings {
        EngineSettings {
            is_translate_swapped: swapped,
            is_output_both_scripts: both,
            ..EngineSettings::default()
        }
    }

    #[test]
    fn document_text_follows_the_output_settings() {
        // trace: CandidateDocumentTextTests.swift:9-47.
        let c = candidate("tâi-gí", Some("台語"));
        assert_eq!(document_text(&c, &settings(false, false)), "tâi-gí");
        assert_eq!(document_text(&c, &settings(true, false)), "台語");
        assert_eq!(document_text(&c, &settings(false, true)), "tâi-gí (台語)");
        assert_eq!(document_text(&c, &settings(true, true)), "台語 (tâi-gí)");
    }

    #[test]
    fn roman_only_and_empty_hanji_write_the_romanization_everywhere() {
        for c in [candidate("guá", None), candidate("guá", Some(""))] {
            for (swapped, both) in [(false, false), (true, false), (false, true), (true, true)] {
                assert_eq!(document_text(&c, &settings(swapped, both)), "guá");
                assert_eq!(alternate_text(&c, &settings(swapped, both)), None);
            }
        }
    }

    #[test]
    fn roman_only_mode_shows_and_writes_the_romanization_alone() {
        // trace: hanji present, mode=RomanOnly → cell = roman with no
        // annotation whichever way the swap points, so `alternate_text` is
        // `None` (Space → `Ignored`). Enter writes the roman through the
        // unchanged `document_text` arms because the snapshot's pair is
        // DERIVED false under roman-only (`SettingsDocument::engine_settings`).
        let c = candidate("tâi-gí", Some("台語"));
        for swapped in [false, true] {
            let settings = EngineSettings {
                is_translate_swapped: swapped,
                candidate_display_mode: CandidateDisplayMode::RomanOnly,
                ..EngineSettings::default()
            };
            let cell = CandidateCellContent::cell(&c, &settings);
            assert_eq!(cell.text, "tâi-gí");
            assert_eq!(cell.annotation, None);
            assert_eq!(alternate_text(&c, &settings), None);
        }
        let derived = EngineSettings {
            candidate_display_mode: CandidateDisplayMode::RomanOnly,
            ..EngineSettings::default()
        };
        assert_eq!(document_text(&c, &derived), "tâi-gí");
        // Side-by-side is untouched by the new arm.
        let cell = CandidateCellContent::cell(&c, &settings(false, false));
        assert_eq!(cell.annotation.as_deref(), Some("台語"));
    }

    #[test]
    fn alternate_is_whichever_script_the_primary_is_not_and_ignores_brackets() {
        // trace: CandidateDocumentTextTests.swift:85-160.
        let c = candidate("tâi-gí", Some("台語"));
        assert_eq!(
            alternate_text(&c, &settings(false, false)).as_deref(),
            Some("台語")
        );
        assert_eq!(
            alternate_text(&c, &settings(true, false)).as_deref(),
            Some("tâi-gí")
        );
        assert_eq!(
            alternate_text(&c, &settings(false, true)).as_deref(),
            Some("台語")
        );
        let cell = CandidateCellContent::cell(&c, &settings(true, false));
        assert_eq!(cell.text, "台語");
        assert_eq!(cell.annotation.as_deref(), Some("tâi-gí"));
        let hyphenated = candidate("kau--lâng", Some("交--人"));
        assert_eq!(
            alternate_text(&hyphenated, &settings(false, false)).as_deref(),
            Some("交--人"),
            "verbatim"
        );
    }
}
