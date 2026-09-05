//! The cells the window shows for a fetched list, each knowing which
//! candidate it stands for and which of that candidate's scripts it commits.
//! Under 漢羅濫 one candidate is TWO adjacent one-script cells (hanji, then
//! roman), so a window index is a CELL index and never indexes the fetched
//! list directly. Port of macOS `ComposingManager.presentation(for:)`
//! (invariants §42).

use std::collections::HashSet;

use super::document_text::{CandidateCellContent, CandidateScript};
use super::manager::ComposingManager;
use crate::engine::ContinuousCandidate;
use crate::settings::{CandidateDisplayMode, EngineSettings};

/// One cell of the window: the candidate it was built from and the script
/// its own commit (Enter, a slot key) writes; Space writes the flip.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PresentedCandidate {
    pub candidate_index: usize,
    pub script: CandidateScript,
    pub cell: CandidateCellContent,
}

/// The cells for `candidates` under `settings`, in display order: one
/// `Primary` cell per candidate, except 漢羅濫 splits a hanji-bearing
/// candidate into a hanji cell then an `Alternate` roman cell.
///
/// Both scripts dedupe on the TEXT THE CELL SHOWS, first-seen wins: a
/// one-script cell carries nothing that could tell it from an earlier cell
/// reading the same, so a second one is a defect, not a second offer
/// (USER 2026-09-03 「相同的漢字 or 羅馬字不能重複出現」). The scripts keep separate
/// keys. Hanji cells were exempt until 2026-09-03 on Core Principle #7
/// grounds: 重 tîng / 重 tāng ARE two words, but under 漢羅濫 they draw two
/// identical 重 cells, and the losing reading stays reachable through its own
/// roman cell. 漢羅並排 is untouched — its annotation tells the pair apart.
pub(crate) fn presentation(
    candidates: &[ContinuousCandidate],
    settings: &EngineSettings,
) -> Vec<PresentedCandidate> {
    if settings.candidate_display_mode != CandidateDisplayMode::Combined {
        return candidates
            .iter()
            .enumerate()
            .map(|(candidate_index, candidate)| PresentedCandidate {
                candidate_index,
                script: CandidateScript::Primary,
                cell: CandidateCellContent::cell(candidate, settings),
            })
            .collect();
    }
    let mut presented = Vec::with_capacity(candidates.len() * 2);
    let mut seen_hanji: HashSet<&str> = HashSet::new();
    let mut seen_roman: HashSet<&str> = HashSet::new();
    for (candidate_index, candidate) in candidates.iter().enumerate() {
        let hanji = candidate.nonempty_hanji();
        if let Some(hanji) = hanji {
            if seen_hanji.insert(hanji) {
                presented.push(PresentedCandidate {
                    candidate_index,
                    script: CandidateScript::Primary,
                    cell: CandidateCellContent::new(hanji, None),
                });
            }
        }
        let roman_is_new = seen_roman.insert(candidate.roman.as_str());
        if roman_is_new {
            presented.push(PresentedCandidate {
                candidate_index,
                script: if hanji.is_some() {
                    CandidateScript::Alternate
                } else {
                    CandidateScript::Primary
                },
                cell: CandidateCellContent::new(candidate.roman.clone(), None),
            });
        }
    }
    presented
}

/// The candidates the engine offered for one context and the cells shown
/// for them, written together so neither can outlive the other: `set` /
/// `clear` are the only writes, and every window index (highlighted, slot,
/// the UI-less element's selection) comes back through `resolve`.
#[derive(Debug, Default, PartialEq)]
pub struct CandidateSource {
    candidates: Vec<ContinuousCandidate>,
    presented: Vec<PresentedCandidate>,
}

impl CandidateSource {
    /// A fresh list, presented under the settings in force right now.
    pub fn set(&mut self, candidates: Vec<ContinuousCandidate>, manager: &ComposingManager) {
        self.presented = manager.presentation(&candidates);
        self.candidates = candidates;
    }

    /// The same list presented again under the settings in force right
    /// now (the 漢羅 flip re-renders in place).
    pub fn refresh_presentation(&mut self, manager: &ComposingManager) {
        self.presented = manager.presentation(&self.candidates);
    }

    pub fn clear(&mut self) {
        self.candidates.clear();
        self.presented.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.presented.is_empty()
    }

    /// What the window draws, display order.
    pub fn cells(&self) -> Vec<CandidateCellContent> {
        self.presented
            .iter()
            .map(|presented| presented.cell.clone())
            .collect()
    }

    /// The candidate behind cell `cell_index` and the script its commit
    /// writes: the cell's own, or the other one when `flip` (Space). `None`
    /// past the list — nothing to commit.
    pub fn resolve(
        &self,
        cell_index: usize,
        flip: bool,
    ) -> Option<(&ContinuousCandidate, CandidateScript)> {
        let presented = self.presented.get(cell_index)?;
        let candidate = self.candidates.get(presented.candidate_index)?;
        let script = if flip {
            presented.script.flipped()
        } else {
            presented.script
        };
        Some((candidate, script))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composing::{NextWordLearner, NoStores, SystemClock};
    use crate::engine::test_support::candidate;
    use crate::settings::{keys, SettingsDocument, StaticSettingsProvider};
    use std::sync::Arc;

    /// The DERIVED snapshot for a mode (`SettingsDocument::engine_settings`):
    /// the swap forced on under Combined, off under RomanOnly.
    fn settings(mode: CandidateDisplayMode, stored_swap: bool) -> EngineSettings {
        EngineSettings {
            is_translate_swapped: mode.effective_translate_swapped(stored_swap),
            candidate_display_mode: mode,
            ..EngineSettings::default()
        }
    }

    fn texts(presented: &[PresentedCandidate]) -> Vec<(usize, CandidateScript, &str)> {
        presented
            .iter()
            .map(|p| (p.candidate_index, p.script, p.cell.text.as_str()))
            .collect()
    }

    #[test]
    fn side_by_side_and_roman_only_are_one_primary_cell_per_candidate_as_today() {
        let list = [candidate("tâi", Some("台"), 3), candidate("tâi", None, 3)];
        for mode in [
            CandidateDisplayMode::SideBySide,
            CandidateDisplayMode::RomanOnly,
        ] {
            for swap in [false, true] {
                let settings = settings(mode, swap);
                let presented = presentation(&list, &settings);
                assert_eq!(presented.len(), 2, "{mode:?} swap={swap}");
                for (index, (presented, candidate)) in presented.iter().zip(&list).enumerate() {
                    assert_eq!(presented.candidate_index, index);
                    assert_eq!(presented.script, CandidateScript::Primary);
                    assert_eq!(
                        presented.cell,
                        CandidateCellContent::cell(candidate, &settings),
                        "{mode:?} swap={swap}"
                    );
                }
            }
        }
    }

    #[test]
    fn combined_splits_a_hanji_candidate_into_hanji_then_roman_cells() {
        // trace: 台語/tâi-gí → [台語 (0, Primary), tâi-gí (0, Alternate)];
        // hanji-less guá → one Primary roman cell.
        let list = [
            candidate("tâi-gí", Some("台語"), 5),
            candidate("guá", None, 3),
            candidate("guá", Some(""), 3),
        ];
        let presented = presentation(&list, &settings(CandidateDisplayMode::Combined, false));
        assert_eq!(
            texts(&presented),
            vec![
                (0, CandidateScript::Primary, "台語"),
                (0, CandidateScript::Alternate, "tâi-gí"),
                (1, CandidateScript::Primary, "guá"),
            ],
            "the empty-hanji duplicate's roman cell is absorbed by guá's"
        );
        assert!(presented.iter().all(|p| p.cell.annotation.is_none()));
    }

    #[test]
    fn combined_dedupes_both_scripts_on_the_text_the_cell_shows() {
        // trace: literal tâi (slot 0, hanji-less) absorbs 台's roman; 重 tîng /
        // 重 tāng draw ONE 重 cell (first-seen) and keep both roman cells; 食 /
        // 𤆬 keep both hanji cells and share one tsia̍h; the repeated 食 over a
        // different span adds nothing — a cell reading the same is not listed.
        let list = [
            candidate("tâi", None, 3),
            candidate("tâi", Some("台"), 3),
            candidate("tîng", Some("重"), 5),
            candidate("tāng", Some("重"), 5),
            candidate("tsia̍h", Some("食"), 5),
            candidate("tsia̍h", Some("𤆬"), 5),
            candidate("tsia̍h", Some("食"), 6),
        ];
        let presented = presentation(&list, &settings(CandidateDisplayMode::Combined, false));
        assert_eq!(
            texts(&presented),
            vec![
                (0, CandidateScript::Primary, "tâi"),
                (1, CandidateScript::Primary, "台"),
                (2, CandidateScript::Primary, "重"),
                (2, CandidateScript::Alternate, "tîng"),
                (3, CandidateScript::Alternate, "tāng"),
                (4, CandidateScript::Primary, "食"),
                (4, CandidateScript::Alternate, "tsia̍h"),
                (5, CandidateScript::Primary, "𤆬"),
            ]
        );
    }

    fn manager(mode: CandidateDisplayMode) -> ComposingManager {
        let mut document = SettingsDocument::default();
        document.set_choice(&keys::CANDIDATE_DISPLAY_MODE, mode);
        ComposingManager::new(
            Arc::new(StaticSettingsProvider::new(document)),
            Box::new(NoStores),
            Box::new(NoStores),
            NextWordLearner::new(Box::new(NoStores), Box::new(SystemClock)),
            Box::new(SystemClock),
            1,
        )
    }

    #[test]
    fn source_resolves_cell_indices_to_candidate_and_script_with_the_flip() {
        // trace: Combined → cells [台語, tâi-gí, guá]; Enter on cell 1 = (台語,
        // Alternate) = roman; Space on cell 0 = flip → Alternate = roman;
        // Space on cell 1 = flip → Primary = hanji; cell 3 past the list.
        let manager = manager(CandidateDisplayMode::Combined);
        let list = vec![
            candidate("tâi-gí", Some("台語"), 5),
            candidate("guá", None, 3),
        ];
        let mut source = CandidateSource::default();
        assert!(source.is_empty());
        source.set(list.clone(), &manager);
        assert!(!source.is_empty());
        assert_eq!(
            source.resolve(0, false),
            Some((&list[0], CandidateScript::Primary))
        );
        assert_eq!(
            source.resolve(1, false),
            Some((&list[0], CandidateScript::Alternate))
        );
        assert_eq!(
            source.resolve(0, true),
            Some((&list[0], CandidateScript::Alternate))
        );
        assert_eq!(
            source.resolve(1, true),
            Some((&list[0], CandidateScript::Primary))
        );
        assert_eq!(
            source.resolve(2, false),
            Some((&list[1], CandidateScript::Primary))
        );
        assert_eq!(source.resolve(3, false), None);
        source.clear();
        assert!(source.is_empty());
        assert!(source.cells().is_empty());
        assert_eq!(source.resolve(0, false), None);
    }

    #[test]
    fn refresh_presentation_re_renders_the_same_candidates_under_new_settings() {
        // trace: the list set under SideBySide is one cell per candidate;
        // re-presented by a Combined manager it is the split — candidates
        // untouched (the swap arm's in-place update).
        let list = vec![candidate("tâi-gí", Some("台語"), 5)];
        let mut source = CandidateSource::default();
        source.set(list.clone(), &manager(CandidateDisplayMode::SideBySide));
        assert_eq!(source.cells().len(), 1);
        source.refresh_presentation(&manager(CandidateDisplayMode::Combined));
        assert_eq!(source.cells().len(), 2);
        assert_eq!(
            source.resolve(1, false),
            Some((&list[0], CandidateScript::Alternate))
        );
    }
}
