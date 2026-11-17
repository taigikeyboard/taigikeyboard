//! The cells the window shows for a fetched list, each knowing which of its
//! candidate's scripts it commits. The engine lists one row per CELL: under
//! Hanji with Romanization a candidate arrives as a Hanji row then a
//! romanization row, already deduped per script (`CandidateMessage.cell_script`,
//! invariants §42), so a window index is a row index.

use protos::engine::CellScript;

use super::cell_content::{CandidateCellContent, CandidateScript};
use super::manager::ComposingManager;
use crate::engine::ContinuousCandidate;
use crate::settings::EngineSettings;

/// One cell of the window: the script its own commit (Enter, a slot key)
/// writes — Output the Other Script writes the flip — and what it draws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PresentedCandidate {
    pub script: CandidateScript,
    pub cell: CandidateCellContent,
}

/// The cell for each row of `candidates` under `settings`, in display order.
/// A paired row is [`CandidateCellContent::cell`] committing the lead; a §42
/// split row shows its own script alone — the Hanji row commits the lead
/// (Hanji leads under Hanji with Romanization), the romanization row the other
/// script, or the lead for a candidate with no Hanji.
pub(crate) fn presentation(
    candidates: &[ContinuousCandidate],
    settings: &EngineSettings,
) -> Vec<PresentedCandidate> {
    candidates
        .iter()
        .map(|candidate| match candidate.cell_script {
            CellScript::Paired => PresentedCandidate {
                script: CandidateScript::Primary,
                cell: CandidateCellContent::cell(candidate, settings),
            },
            CellScript::Hanji => PresentedCandidate {
                script: CandidateScript::Primary,
                cell: CandidateCellContent::new(
                    candidate.nonempty_hanji().unwrap_or_default(),
                    None,
                ),
            },
            CellScript::Roman => PresentedCandidate {
                script: if candidate.nonempty_hanji().is_some() {
                    CandidateScript::Alternate
                } else {
                    CandidateScript::Primary
                },
                cell: CandidateCellContent::new(candidate.roman.clone(), None),
            },
        })
        .collect()
}

/// Whether `candidates` leads with the §34 literal — the WYSIWYG romanization
/// the engine prepends at index 0 while Show Typed Text First is on
/// (`engine/composing/src/requests.rs::handle_fetch_at_pos`). That cell takes no slot key:
/// it is what the user is already typing, not an offer to pick (USER
/// 2026-09-09), so the keys start on the cell after it
/// (`CandidateIndexLabel::candidate_index_for_key_slot`).
///
/// Read off the setting plus the shape of the leading candidate rather than
/// re-derived: the literal is roman-only by construction (`requests.rs::literal_roman_candidate`
/// `hanji: None`). Under TPS the setting reads off
/// (`SettingsDocument::engine_settings`), so a hanji-less TPS lead keeps its
/// key. A hanji-bearing lead means the prepend did not happen, whatever the
/// setting says, and every cell keeps its key.
pub(crate) fn leads_with_literal_roman(
    candidates: &[ContinuousCandidate],
    settings: &EngineSettings,
) -> bool {
    settings.is_literal_roman_candidate_enabled
        && candidates
            .first()
            .is_some_and(|candidate| candidate.nonempty_hanji().is_none())
}

/// The candidates the engine offered for one context and the cells shown
/// for them, written together so neither can outlive the other: `set` /
/// `clear` are the only writes, and every window index (highlighted, slot,
/// the UI-less element's selection) comes back through `resolve`.
#[derive(Debug, Default, PartialEq)]
pub struct CandidateSource {
    candidates: Vec<ContinuousCandidate>,
    presented: Vec<PresentedCandidate>,
    leads_with_literal_roman: bool,
}

impl CandidateSource {
    /// A fresh list, presented under the settings in force right now.
    pub fn set(&mut self, candidates: Vec<ContinuousCandidate>, manager: &ComposingManager) {
        let (presented, leads_with_literal_roman) = manager.presentation(&candidates);
        self.presented = presented;
        self.leads_with_literal_roman = leads_with_literal_roman;
        self.candidates = candidates;
    }

    /// The same list presented again under the settings in force right
    /// now (the Hanji/romanization flip re-renders in place).
    pub(crate) fn refresh_presentation(&mut self, manager: &ComposingManager) {
        let (presented, leads_with_literal_roman) = manager.presentation(&self.candidates);
        self.presented = presented;
        self.leads_with_literal_roman = leads_with_literal_roman;
    }

    pub fn clear(&mut self) {
        self.candidates.clear();
        self.presented.clear();
        self.leads_with_literal_roman = false;
    }

    /// Whether the first cell is the §34 literal, which takes no slot key
    /// (`ComposingManager::leads_with_literal_roman_candidate`). A fact about
    /// THIS fetch under the settings snapshot that presented it — a stale copy
    /// would shift the keys off by one.
    pub fn leads_with_literal_roman(&self) -> bool {
        self.leads_with_literal_roman
    }

    pub fn is_empty(&self) -> bool {
        self.presented.is_empty()
    }

    /// How many cells the window draws — `cells().len()` without copying them.
    pub fn len(&self) -> usize {
        self.presented.len()
    }

    /// What the window draws, display order.
    pub fn cells(&self) -> Vec<CandidateCellContent> {
        self.presented
            .iter()
            .map(|presented| presented.cell.clone())
            .collect()
    }

    /// The candidate behind cell `cell_index` and the script its commit
    /// writes: the cell's own, or the other one when `flip` (Output the Other
    /// Script, ⇧ + slot key). `None`
    /// past the list — nothing to commit.
    pub fn resolve(
        &self,
        cell_index: usize,
        flip: bool,
    ) -> Option<(&ContinuousCandidate, CandidateScript)> {
        let presented = self.presented.get(cell_index)?;
        let candidate = self.candidates.get(cell_index)?;
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
    use crate::composing::{EngineNextWord, SystemClock};
    use crate::engine::test_support::{candidate, tps_candidate};
    use crate::settings::{
        keys, CandidateDisplayMode, InputMode, SettingChoice, SettingsDocument,
        StaticSettingsProvider,
    };
    use std::sync::Arc;

    /// The DERIVED snapshot for a mode (`SettingsDocument::engine_settings`):
    /// the swap forced on under Combined, off under RomanOnly.
    fn settings(mode: CandidateDisplayMode, stored_swap: bool) -> EngineSettings {
        EngineSettings {
            is_hanji_first: mode.effective_hanji_first(stored_swap),
            candidate_display_mode: mode,
            ..EngineSettings::default()
        }
    }

    /// A row the engine sent as the §42 split cell `script`.
    fn split(roman: &str, hanji: Option<&str>, script: CellScript) -> ContinuousCandidate {
        ContinuousCandidate {
            cell_script: script,
            ..candidate(roman, hanji, 5)
        }
    }

    fn texts(presented: &[PresentedCandidate]) -> Vec<(CandidateScript, &str)> {
        presented
            .iter()
            .map(|p| (p.script, p.cell.text.as_str()))
            .collect()
    }

    #[test]
    fn a_paired_row_is_one_primary_cell_as_the_display_renders_it() {
        let list = [candidate("tâi", Some("台"), 3), candidate("tâi", None, 3)];
        for mode in [
            CandidateDisplayMode::SideBySide,
            CandidateDisplayMode::RomanOnly,
        ] {
            for swap in [false, true] {
                let settings = settings(mode, swap);
                let presented = presentation(&list, &settings);
                assert_eq!(presented.len(), 2, "{mode:?} swap={swap}");
                for (presented, candidate) in presented.iter().zip(&list) {
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
    fn a_split_row_shows_its_own_script_and_the_roman_row_commits_the_other() {
        // trace: 台語 Hanji row → (Primary, 台語); its roman row → (Alternate,
        // tâi-gí); a hanji-less guá roman row → (Primary, guá) — one script,
        // nothing to flip to. No annotation on any split cell, whatever the swap.
        let list = [
            split("tâi-gí", Some("台語"), CellScript::Hanji),
            split("tâi-gí", Some("台語"), CellScript::Roman),
            split("guá", None, CellScript::Roman),
        ];
        for stored_swap in [false, true] {
            let raw = EngineSettings {
                is_hanji_first: stored_swap,
                candidate_display_mode: CandidateDisplayMode::Combined,
                ..EngineSettings::default()
            };
            let presented = presentation(&list, &raw);
            assert_eq!(
                texts(&presented),
                vec![
                    (CandidateScript::Primary, "台語"),
                    (CandidateScript::Alternate, "tâi-gí"),
                    (CandidateScript::Primary, "guá"),
                ],
                "swap={stored_swap}"
            );
            assert!(presented.iter().all(|p| p.cell.annotation.is_none()));
        }
    }

    #[test]
    fn an_empty_list_presents_nothing() {
        // trace: `presentation` maps each row to its cell — no rows, no cells.
        for &mode in CandidateDisplayMode::ALL {
            assert!(
                presentation(&[], &settings(mode, false)).is_empty(),
                "{mode:?}"
            );
        }
    }

    fn manager(mode: CandidateDisplayMode) -> ComposingManager {
        let mut document = SettingsDocument::default();
        document.set_choice(&keys::CANDIDATE_DISPLAY_MODE, mode);
        ComposingManager::new(
            Arc::new(StaticSettingsProvider::new(document)),
            Box::new(EngineNextWord),
            Arc::new(SystemClock),
            1,
        )
    }

    /// A manager whose Show Typed Text First setting is `enabled`.
    fn manager_with_literal(enabled: bool) -> ComposingManager {
        let mut document = SettingsDocument::default();
        document.set_bool(&keys::IS_LITERAL_ROMAN_CANDIDATE_ENABLED, enabled);
        ComposingManager::new(
            Arc::new(StaticSettingsProvider::new(document)),
            Box::new(EngineNextWord),
            Arc::new(SystemClock),
            1,
        )
    }

    /// The settings snapshot with Show Typed Text First `enabled`.
    fn literal_settings(enabled: bool) -> EngineSettings {
        EngineSettings {
            is_literal_roman_candidate_enabled: enabled,
            ..EngineSettings::default()
        }
    }

    #[test]
    fn the_lead_cell_is_unkeyed_only_for_a_roman_only_lead_under_the_setting() {
        // trace: the §34 literal is roman-only (hanji None) and leads while the
        // setting is on — that list keys from its second cell. A hanji-bearing
        // lead means the engine did not prepend; an empty list has no lead.
        let literal_led = vec![
            candidate("tâi", None, 0),
            candidate("tâi-gí", Some("台語"), 5),
        ];
        let dict_led = vec![candidate("tâi-gí", Some("台語"), 5)];

        let on = literal_settings(true);
        assert!(leads_with_literal_roman(&literal_led, &on));
        assert!(!leads_with_literal_roman(&dict_led, &on));
        assert!(!leads_with_literal_roman(&[], &on));

        assert!(!leads_with_literal_roman(
            &literal_led,
            &literal_settings(false)
        ));
    }

    #[test]
    fn the_source_carries_the_unkeyed_lead_with_the_list_it_presented() {
        // trace: set → true for a literal-led list; clear → false; a re-present
        // under the same settings keeps it (the Hanji/romanization flip re-renders in place).
        let manager = manager_with_literal(true);
        let mut source = CandidateSource::default();
        assert!(!source.leads_with_literal_roman());

        source.set(
            vec![
                candidate("tâi", None, 0),
                candidate("tâi-gí", Some("台語"), 5),
            ],
            &manager,
        );
        assert!(source.leads_with_literal_roman());

        source.refresh_presentation(&manager);
        assert!(source.leads_with_literal_roman());

        source.clear();
        assert!(!source.leads_with_literal_roman());

        source.set(vec![candidate("tâi-gí", Some("台語"), 5)], &manager);
        assert!(!source.leads_with_literal_roman());
    }

    #[test]
    fn source_resolves_cell_indices_to_candidate_and_script_with_the_flip() {
        // trace: split rows [台語 Hanji, tâi-gí Roman, guá Roman] → Enter on
        // cell 1 = (row 1, Alternate) = roman; the other script on cell 0 = flip
        // → Alternate = roman; on cell 1 = flip → Primary = hanji; cell 3 past
        // the list.
        let manager = manager(CandidateDisplayMode::Combined);
        let list = vec![
            split("tâi-gí", Some("台語"), CellScript::Hanji),
            split("tâi-gí", Some("台語"), CellScript::Roman),
            split("guá", None, CellScript::Roman),
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
            Some((&list[1], CandidateScript::Alternate))
        );
        assert_eq!(
            source.resolve(0, true),
            Some((&list[0], CandidateScript::Alternate))
        );
        assert_eq!(
            source.resolve(1, true),
            Some((&list[1], CandidateScript::Primary))
        );
        assert_eq!(
            source.resolve(2, false),
            Some((&list[2], CandidateScript::Primary))
        );
        assert_eq!(source.resolve(3, false), None);
        source.clear();
        assert!(source.is_empty());
        assert!(source.cells().is_empty());
        assert_eq!(source.resolve(0, false), None);
    }

    #[test]
    fn refresh_presentation_re_renders_the_same_candidates_under_new_settings() {
        // trace: a paired 台語 set under roman-first leads with tâi-gí;
        // re-presented after the swap it leads with 台語 — candidates untouched
        // (the swap arm's in-place update).
        let list = vec![candidate("tâi-gí", Some("台語"), 5)];
        let swapped = |is_hanji_first| {
            let mut document = SettingsDocument::default();
            document.set_bool(&keys::IS_HANJI_FIRST, is_hanji_first);
            ComposingManager::new(
                Arc::new(StaticSettingsProvider::new(document)),
                Box::new(EngineNextWord),
                Arc::new(SystemClock),
                1,
            )
        };
        let mut source = CandidateSource::default();
        source.set(list.clone(), &swapped(false));
        assert_eq!(source.cells()[0].text, "tâi-gí");
        source.refresh_presentation(&swapped(true));
        assert_eq!(source.cells()[0].text, "台語");
        assert_eq!(
            source.resolve(0, false),
            Some((&list[0], CandidateScript::Primary))
        );
    }

    #[test]
    fn under_tps_a_paired_row_is_its_tps_text_and_a_hanji_less_lead_keeps_its_key() {
        // trace: under TPS the engine never splits, so rows arrive PAIRED; the
        // cell is the engine's `tps_text`. `engine_settings` reads Show Typed
        // Text First off under TPS, so `leads_with_literal_roman` is false
        // though the stored setting is on and the lead has no Hanji.
        let candidates = vec![
            tps_candidate("ka", None, 0, "ㄍㄚ"),
            tps_candidate("ka", Some("家"), 1, "家"),
        ];
        let mut document = SettingsDocument::default();
        document.set_choice(&keys::INPUT_MODE, InputMode::Tps);
        document.set_choice(
            &keys::CANDIDATE_DISPLAY_MODE,
            CandidateDisplayMode::Combined,
        );
        document.set_bool(&keys::IS_LITERAL_ROMAN_CANDIDATE_ENABLED, true);
        let tps = document.engine_settings();
        let presented = presentation(&candidates, &tps);
        assert_eq!(
            texts(&presented),
            [
                (CandidateScript::Primary, "ㄍㄚ"),
                (CandidateScript::Primary, "家")
            ]
        );
        assert!(!leads_with_literal_roman(&candidates, &tps));
    }
}
