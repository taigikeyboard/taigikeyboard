//! What one candidate writes into the document, and what its cell shows.
//! Port of `CandidateDocumentText.swift`, `CandidateCellContent.swift`,
//! `CandidateScript.swift`.

// 候選送出時寫入文件的字串,與候選格顯示的兩種文字;身分鍵另有其人(漢字, canonical TL)。

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

impl CandidateScript {
    /// The other script — what Space writes relative to a cell's own.
    pub fn flipped(self) -> Self {
        match self {
            Self::Primary => Self::Alternate,
            Self::Alternate => Self::Primary,
        }
    }
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
    /// romanization, secondary = Hanji) and the swap flip; arm order is the
    /// same on every platform. Serves 並排 and 羅馬字; 合用's split cells
    /// are built by [`super::presentation`].
    pub fn cell(candidate: &ContinuousCandidate, settings: &EngineSettings) -> Self {
        match candidate.nonempty_hanji() {
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

/// One commit's document string, and whether writing it puts romanization
/// in the document — the single input the auto-space gate reads
/// (`policies::is_gate_active`). Port of `ResolvedCommit` (Swift).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedCommit {
    pub text: String,
    pub wrote_romanization: bool,
}

/// What committing `candidate` writes into the document under `settings` —
/// the string alone, for the window's cell label.
/// CROSS-PLATFORM INVARIANT — mirrors
/// `ios/.../ActionHandler+Suggestions.swift:218`; the TPS branch is absent
/// because the desktop ships TL and POJ only.
pub fn document_text(candidate: &ContinuousCandidate, settings: &EngineSettings) -> String {
    resolved_commit(candidate, settings).text
}

/// [`document_text`] with the auto-space verdict resolved by the SAME arm
/// that picks the string.
///
/// Auto-space is a property of ROMANIZATION (`guá beh khì` needs the gaps,
/// 我欲去 does not), so its gate has to answer for the string this commit
/// actually writes. Deriving the verdict from the output mode instead is
/// only ever an approximation, and it is wrong for a candidate with no
/// Hanji: the 字面羅馬字 candidate (§34), an out-of-vocabulary name, a
/// romanization-only custom entry all write their romanization whatever the
/// mode leads with.
/// CROSS-PLATFORM INVARIANT — mirrors `macos/.../CandidateDocumentText.swift`
/// `resolved(for:settings:)`. Drift changes which commits earn a space.
pub fn resolved_commit(
    candidate: &ContinuousCandidate,
    settings: &EngineSettings,
) -> ResolvedCommit {
    // An empty-string hanji folds to roman-only — "guá ()" would be a
    // visible defect — and romanization is what the document gets whichever
    // script the mode leads with.
    let Some(hanji) = candidate.nonempty_hanji() else {
        return ResolvedCommit {
            text: candidate.roman.clone(),
            wrote_romanization: true,
        };
    };
    if settings.is_output_both_scripts {
        // 括號標註 writes the pair, so the romanization IS in the document
        // whichever half leads.
        let text = if settings.is_translate_swapped {
            format!("{hanji} ({})", candidate.roman)
        } else {
            format!("{} ({hanji})", candidate.roman)
        };
        ResolvedCommit {
            text,
            wrote_romanization: true,
        }
    } else if settings.is_translate_swapped {
        ResolvedCommit {
            text: hanji.to_owned(),
            wrote_romanization: false,
        }
    } else {
        ResolvedCommit {
            text: candidate.roman.clone(),
            wrote_romanization: true,
        }
    }
}

/// What Space commits — the script `document_text` does NOT lead with, or
/// `None` when the candidate has one script or the mode shows no hanji.
/// Read off the settings, never the cell: under 合用 the flip in
/// `CandidateSource::resolve` turns this into "the other cell's script".
/// `is_output_both_scripts` is not consulted — Space writes one script, so
/// its verdict is simply which script that is.
pub fn resolved_alternate(
    candidate: &ContinuousCandidate,
    settings: &EngineSettings,
) -> Option<ResolvedCommit> {
    let hanji = candidate.nonempty_hanji()?;
    if !settings.candidate_display_mode.shows_hanji() {
        return None;
    }
    Some(if settings.is_translate_swapped {
        ResolvedCommit {
            text: candidate.roman.clone(),
            wrote_romanization: true,
        }
    } else {
        ResolvedCommit {
            text: hanji.to_owned(),
            wrote_romanization: false,
        }
    })
}

/// The alternate's text alone, for tests and callers that only render.
#[cfg(test)]
fn alternate_text(candidate: &ContinuousCandidate, settings: &EngineSettings) -> Option<String> {
    resolved_alternate(candidate, settings).map(|resolved| resolved.text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::test_support::candidate;

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
        let c = candidate("tâi-gí", Some("台語"), 0);
        assert_eq!(document_text(&c, &settings(false, false)), "tâi-gí");
        assert_eq!(document_text(&c, &settings(true, false)), "台語");
        assert_eq!(document_text(&c, &settings(false, true)), "tâi-gí (台語)");
        assert_eq!(document_text(&c, &settings(true, true)), "台語 (tâi-gí)");
    }

    #[test]
    fn roman_only_and_empty_hanji_write_the_romanization_everywhere() {
        for c in [candidate("guá", None, 0), candidate("guá", Some(""), 0)] {
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
        let c = candidate("tâi-gí", Some("台語"), 0);
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
        // Side-by-side is untouched by the roman-only arm.
        let cell = CandidateCellContent::cell(&c, &settings(false, false));
        assert_eq!(cell.annotation.as_deref(), Some("台語"));
    }

    #[test]
    fn alternate_text_follows_the_display_mode_not_the_cell() {
        // trace: hanji present → SideBySide both swaps = the annotation the
        // cell carries today; Combined (derived swap = true) = roman; RomanOnly
        // = None; hanji-less = None; 括號標註 ignored (one script).
        let c = candidate("tâi-gí", Some("台語"), 0);
        for both in [false, true] {
            let side_by_side = |swapped| EngineSettings {
                is_translate_swapped: swapped,
                is_output_both_scripts: both,
                ..EngineSettings::default()
            };
            assert_eq!(
                alternate_text(&c, &side_by_side(false)),
                CandidateCellContent::cell(&c, &side_by_side(false)).annotation
            );
            assert_eq!(
                alternate_text(&c, &side_by_side(true)),
                CandidateCellContent::cell(&c, &side_by_side(true)).annotation
            );
            let combined = EngineSettings {
                is_translate_swapped: true,
                is_output_both_scripts: both,
                candidate_display_mode: CandidateDisplayMode::Combined,
                ..EngineSettings::default()
            };
            assert_eq!(alternate_text(&c, &combined).as_deref(), Some("tâi-gí"));
            let roman_only = EngineSettings {
                is_output_both_scripts: both,
                candidate_display_mode: CandidateDisplayMode::RomanOnly,
                ..EngineSettings::default()
            };
            assert_eq!(alternate_text(&c, &roman_only), None);
            assert_eq!(alternate_text(&candidate("guá", None, 0), &combined), None);
            assert_eq!(
                alternate_text(&candidate("guá", Some(""), 0), &combined),
                None
            );
        }
    }

    #[test]
    fn the_verdict_follows_the_string_the_arm_picked() {
        // trace: CandidateDocumentTextTests.swift
        // `testResolved_saysWhetherTheStringItPickedCarriesRomanization`.
        let c = candidate("tâi-gí", Some("台語"), 0);
        assert!(resolved_commit(&c, &settings(false, false)).wrote_romanization);
        assert!(
            !resolved_commit(&c, &settings(true, false)).wrote_romanization,
            "a pure 漢字 commit earns no space"
        );
        for swapped in [false, true] {
            assert!(
                resolved_commit(&c, &settings(swapped, true)).wrote_romanization,
                "括號標註 writes the pair either way round (swapped={swapped})"
            );
        }
    }

    #[test]
    fn a_candidate_with_no_hanji_always_carries_romanization() {
        // trace: `resolved_commit` — the hanji-absent arm. Romanization
        // under EVERY mode, including the two the old mode proxy called a
        // hanji commit (漢字優先 and 漢羅濫).
        for c in [candidate("taigi", None, 0), candidate("taigi", Some(""), 0)] {
            for (swapped, both) in [(false, false), (true, false), (false, true), (true, true)] {
                let resolved = resolved_commit(&c, &settings(swapped, both));
                assert_eq!(resolved.text, "taigi");
                assert!(resolved.wrote_romanization, "swapped={swapped} both={both}");
            }
        }
    }

    #[test]
    fn the_alternate_verdict_inverts_the_mode_and_ignores_brackets() {
        let c = candidate("tâi-gí", Some("台語"), 0);
        for both in [false, true] {
            let swapped = resolved_alternate(&c, &settings(true, both)).unwrap();
            assert_eq!(swapped.text, "tâi-gí");
            assert!(swapped.wrote_romanization, "both={both}");
            let roman_first = resolved_alternate(&c, &settings(false, both)).unwrap();
            assert_eq!(roman_first.text, "台語");
            assert!(
                !roman_first.wrote_romanization,
                "Space wrote the hanji, not the pair (both={both})"
            );
        }
    }

    #[test]
    fn alternate_is_whichever_script_the_primary_is_not_and_ignores_brackets() {
        // trace: CandidateDocumentTextTests.swift:85-160.
        let c = candidate("tâi-gí", Some("台語"), 0);
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
        let hyphenated = candidate("kau--lâng", Some("交--人"), 0);
        assert_eq!(
            alternate_text(&hyphenated, &settings(false, false)).as_deref(),
            Some("交--人"),
            "verbatim"
        );
    }
}
