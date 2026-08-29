//! Turns a user intent into an engine round-trip, mirrors what came back, and
//! hands the engine's effects to the executor for the client that asked.
//! Port of `ComposingManager.swift`.
//!
//! The engine owns the composition (phase, raw buffer, candidate index); this
//! owns only a mirror of the last answer, which the controller reads to
//! decide whether a key belongs to the composition or to the host.

// 中文: 組字管理者 — 意圖 → 引擎往返 → 鏡像 → 依序執行效果;候選兩段式提取;送出結果從效果讀。

use std::collections::HashSet;
use std::sync::Arc;

use super::document_text::{alternate_text, document_text, CandidateCellContent, CandidateScript};
use super::learner::NextWordLearner;
use super::outcomes::{CandidateCommitOutcome, CandidateFetchOutcome};
use super::stores::{Clock, CustomDictionarySource, FrequencySource};
use crate::engine::{
    self, CommitContinuousArgs, ComposingTransition, ContinuousCandidate, CustomEntry, Effect,
    FetchArgs, FrequencyRow,
};
use crate::settings::{EngineSettings, SettingsProvider};

/// Writes the engine's document effects into the client that is currently
/// focused. Learning handshakes never reach it — the manager routes them to
/// the learner, because they write to a database rather than a document.
pub trait ComposingEffectExecutor {
    fn execute(&mut self, effect: &Effect);
}

pub struct ComposingManager {
    is_composing: bool,
    /// What the user typed, with numeric tones. Once a candidate has been
    /// nailed this is only the pending tail (`transition.rs:585`).
    raw_input: String,
    /// The composition as the preedit renders it — the whole thing, nailed
    /// prefix included. Mirrored because only the engine knows how segments
    /// join, and because the candidate window anchors to what is on screen.
    display_text: String,
    settings: Arc<dyn SettingsProvider>,
    frequency: Box<dyn FrequencySource>,
    custom_dictionary: Box<dyn CustomDictionarySource>,
    learner: NextWordLearner,
    clock: Box<dyn Clock>,
    /// Unique across everything that talks to the engine in this process:
    /// the engine keeps one composition per process and drops it whenever the
    /// generation it is handed changes.
    current_generation: u64,
}

impl ComposingManager {
    /// `starting_generation` defaults to 1 in production because 0 is the
    /// generation an unset proto field carries. Every store is explicit —
    /// a defaulted parameter is how a test would silently teach the user's
    /// own database from a fixture.
    pub fn new(
        settings: Arc<dyn SettingsProvider>,
        frequency: Box<dyn FrequencySource>,
        custom_dictionary: Box<dyn CustomDictionarySource>,
        learner: NextWordLearner,
        clock: Box<dyn Clock>,
        starting_generation: u64,
    ) -> Self {
        Self {
            is_composing: false,
            raw_input: String::new(),
            display_text: String::new(),
            settings,
            frequency,
            custom_dictionary,
            learner,
            clock,
            current_generation: starting_generation,
        }
    }

    pub fn is_composing(&self) -> bool {
        self.is_composing
    }

    pub fn raw_input(&self) -> &str {
        &self.raw_input
    }

    pub fn display_text(&self) -> &str {
        &self.display_text
    }

    pub fn generation(&self) -> u64 {
        self.current_generation
    }

    fn current_settings(&self) -> EngineSettings {
        self.settings.current().engine_settings()
    }

    // MARK: - Session lifecycle

    /// Abandons any composition without touching a document, so the next
    /// session starts from an idle engine. Sends no `Reset`: the engine drops
    /// its state as soon as it sees the new generation (`handle.rs:61-66`).
    /// Does NOT clear the preedit on screen — that is the outgoing session's
    /// job while it still has its context.
    pub fn start_new_session(&mut self) {
        self.current_generation = self.current_generation.wrapping_add(1);
        self.clear_mirror();
        // The next-word context goes with the composition: a session change is
        // usually a change of application, and carrying the context across
        // would learn the last word typed in a chat window as the predecessor
        // of the first word typed in a terminal.
        self.learner
            .forget_context(&self.current_settings(), self.current_generation);
    }

    /// A character reached the host without going through a composition.
    /// Forwarded so the engine can end the context on sentence-end
    /// punctuation; letters (they start compositions) and whitespace (never
    /// punctuation, and a round-trip per space bar) are excluded.
    pub fn note_character_typed_outside_composition(&self, character: &str) {
        if character.is_empty()
            || character
                .chars()
                .any(|c| c.is_alphabetic() || c.is_whitespace())
        {
            return;
        }
        self.learner.word_selected(
            character,
            "",
            &self.current_settings(),
            self.current_generation,
        );
    }

    /// Appends one typed character. The engine starts a composition when idle
    /// (`transition.rs:55`), so there is no separate "begin" call.
    pub fn append(&mut self, character: &str, executor: &mut dyn ComposingEffectExecutor) {
        log::debug!("append");
        let settings = self.current_settings();
        let transition = engine::append(character, &settings, self.current_generation);
        self.apply(transition, executor);
        self.promote_to_continuous(&settings, executor);
    }

    /// Drops the last character of the raw buffer. Ends the composition when
    /// that empties it.
    pub fn delete_backward(&mut self, executor: &mut dyn ComposingEffectExecutor) {
        log::debug!("deleteBackward");
        let transition = engine::delete_backward(&self.current_settings(), self.current_generation);
        self.apply(transition, executor);
    }

    /// Commits the composition as rendered (`CommitRaw`). Answers the text
    /// the commit wrote, or `None` for a commit that never reached the engine
    /// or wrote nothing — what auto-space earns its trailing space from.
    pub fn commit_composition(
        &mut self,
        executor: &mut dyn ComposingEffectExecutor,
    ) -> Option<String> {
        log::debug!("commitComposition");
        let transition = engine::commit_raw(&self.current_settings(), self.current_generation);
        let committed = Self::committed_text(transition.as_ref());
        self.apply(transition, executor);
        committed
    }

    /// Commits the composition and appends `text` after it in the same engine
    /// step, so one keystroke reaches the host as one document mutation.
    pub fn commit_composition_then_insert(
        &mut self,
        text: &str,
        executor: &mut dyn ComposingEffectExecutor,
    ) -> Option<String> {
        log::debug!("commitCompositionThenInsert");
        let settings = self.current_settings();
        let transition =
            engine::commit_preedit_then_insert_external(text, &settings, self.current_generation);
        let committed = Self::committed_text(transition.as_ref());
        self.apply(transition, executor);
        // The one commit path the engine does not describe to the learner: it
        // emits `NextWordClearForNewComposing` and no `NextWordWordSelected`
        // (`transition.rs:769-780`). If the context were left alone, the NEXT
        // commit would pair itself with whatever was committed BEFORE this one.
        // Dropping the context under-learns one pair rather than learning a
        // wrong one.
        self.learner
            .forget_context(&settings, self.current_generation);
        committed
    }

    /// Abandons the composition. Nothing reaches the document.
    pub fn cancel_composition(&mut self, executor: &mut dyn ComposingEffectExecutor) {
        log::debug!("cancelComposition");
        let transition = engine::reset(self.current_generation);
        self.apply(transition, executor);
    }

    // MARK: - Candidates

    /// Reads the candidates for the composition as it stands, ranked against
    /// what the user has committed before. Two fetches: the first is neutral
    /// and discovers the keys; the second re-ranks with the counts those keys
    /// carry. Both go out under the same generation and settings snapshot.
    /// Every shortfall degrades to the neutral ranking — except a second
    /// fetch that succeeds and reports the engine idle, which is newer than
    /// the first and wins.
    pub fn fetch_candidates(&mut self) -> CandidateFetchOutcome {
        let settings = self.current_settings();
        let generation = self.current_generation;
        // Resolved once from this one snapshot and handed to both phases.
        let enabled_sources_bitmask = engine::enabled_sources_bitmask(&settings.dictionary_sources);
        let custom_entries = self.custom_dictionary_matches(&settings);

        let Some(neutral) = engine::fetch_at_pos(
            &settings,
            generation,
            &FetchArgs {
                enabled_sources_bitmask,
                custom_entries: &custom_entries,
                ..FetchArgs::default()
            },
        ) else {
            return CandidateFetchOutcome::Unavailable;
        };
        let Some(neutral_candidates) = neutral.candidates else {
            self.mirror(&neutral.transition);
            return CandidateFetchOutcome::NotComposing;
        };
        let rows = if neutral_candidates.is_empty() {
            None
        } else {
            self.frequency_rows(&neutral_candidates)
                .filter(|rows| !rows.is_empty())
        };
        let Some(rows) = rows else {
            self.mirror(&neutral.transition);
            return CandidateFetchOutcome::Found(neutral_candidates);
        };

        let Some(boosted) = engine::fetch_at_pos(
            &settings,
            generation,
            &FetchArgs {
                frequency_rows: &rows,
                now_ms: self.clock.now_ms(),
                enabled_sources_bitmask,
                custom_entries: &custom_entries,
            },
        ) else {
            self.mirror(&neutral.transition);
            return CandidateFetchOutcome::Found(neutral_candidates);
        };
        self.mirror(&boosted.transition);
        match boosted.candidates {
            Some(candidates) => CandidateFetchOutcome::Found(candidates),
            None => CandidateFetchOutcome::NotComposing,
        }
    }

    /// The learned rows for the candidates on offer, deduped by the key the
    /// engine ranks on.
    fn frequency_rows(&self, candidates: &[ContinuousCandidate]) -> Option<Vec<FrequencyRow>> {
        let mut seen = HashSet::new();
        let keys: Vec<String> = candidates
            .iter()
            .map(|candidate| candidate.display_text.clone())
            .filter(|key| seen.insert(key.clone()))
            .collect();
        self.frequency.rows_for_words(&keys)
    }

    /// The user's own dictionary's matches for what is being typed. With the
    /// setting off nothing is read at all — the gate is on the lookup.
    fn custom_dictionary_matches(&self, settings: &EngineSettings) -> Vec<CustomEntry> {
        if !settings.is_custom_dict_enabled || self.raw_input.is_empty() {
            return Vec::new();
        }
        let Some(key) = engine::derive_custom_query_key(&self.raw_input, settings.input_mode)
        else {
            return Vec::new();
        };
        self.custom_dictionary
            .rows_matching(&key.family, &key.form, &key.key)
    }

    /// What committing `candidate` would write into the document, under the
    /// settings in force right now — so the window labels a cell with the
    /// string that cell produces.
    pub fn document_text(&self, candidate: &ContinuousCandidate) -> String {
        document_text(candidate, &self.current_settings())
    }

    /// How the window renders `candidate` — both scripts, under the settings
    /// in force right now.
    pub fn cell_content(&self, candidate: &ContinuousCandidate) -> CandidateCellContent {
        CandidateCellContent::cell(candidate, &self.current_settings())
    }

    /// Commits `candidate`, which must come from the `fetch_candidates` call
    /// that produced the list the user is looking at. `script` picks WHICH of
    /// the candidate's two renderings the document gets; `Alternate` on a
    /// single-script candidate answers `Ignored` without reaching the engine.
    /// Returns the outcome and the text written, if any.
    pub fn commit_candidate(
        &mut self,
        candidate: &ContinuousCandidate,
        script: CandidateScript,
        executor: &mut dyn ComposingEffectExecutor,
    ) -> (CandidateCommitOutcome, Option<String>) {
        let settings = self.current_settings();
        log::debug!(
            "commitCandidate consumedBytes={}",
            candidate.consumed_span_end
        );
        let text = match script {
            CandidateScript::Primary => document_text(candidate, &settings),
            CandidateScript::Alternate => match alternate_text(candidate, &settings) {
                Some(alternate) => alternate,
                None => return (CandidateCommitOutcome::Ignored, None),
            },
        };
        let Some(transition) = engine::commit_continuous(
            &CommitContinuousArgs {
                document_text: &text,
                canonical_text: &candidate.display_text,
                association_tl: &candidate.canonical_tl,
                consumed_bytes: candidate.consumed_span_end,
                syllable_count: candidate.syllable_count,
            },
            &settings,
            self.current_generation,
        ) else {
            return (CandidateCommitOutcome::Unavailable, None);
        };
        let outcome = CandidateCommitOutcome::from_transition(&transition);
        let committed = Self::committed_text(Some(&transition));
        self.apply(Some(transition), executor);
        self.record_usage(candidate, outcome, &settings);
        (outcome, committed)
    }

    /// The text `transition` wrote to the document, read off the effects —
    /// the mirror carries the display rendering, which the output settings
    /// can make differ from the document string.
    fn committed_text(transition: Option<&ComposingTransition>) -> Option<String> {
        transition?
            .effects
            .iter()
            .filter_map(|effect| match effect {
                Effect::CommitTextReplacingPreedit(text) => Some(text.clone()),
                _ => None,
            })
            .next_back()
    }

    /// Counts a candidate the engine confirmed it took. Gated on the
    /// effect-backed outcome: `Ignored` can follow a composition the engine
    /// reset out from under the commit. Identity = `(display text, canonical
    /// TL)` (Core Principle #7), never the document rendering.
    fn record_usage(
        &self,
        candidate: &ContinuousCandidate,
        outcome: CandidateCommitOutcome,
        settings: &EngineSettings,
    ) {
        if !settings.is_frequency_recording_enabled {
            return;
        }
        if matches!(
            outcome,
            CandidateCommitOutcome::Nailed | CandidateCommitOutcome::Finalized
        ) {
            self.frequency
                .record(&candidate.display_text, &candidate.canonical_tl);
        }
    }

    /// Promotes the composition into the continuous phase, on the same call
    /// stack as the character that triggered it: the engine no-ops on an
    /// empty or already-continuous buffer (`transition.rs:496-502`).
    fn promote_to_continuous(
        &mut self,
        settings: &EngineSettings,
        executor: &mut dyn ComposingEffectExecutor,
    ) {
        let transition = engine::enter_continuous(settings, self.current_generation);
        self.apply(transition, executor);
    }

    /// Mirror first, then run the effects in the order the engine listed
    /// them. `None` is a round-trip that never reached the engine: nothing to
    /// mirror and nothing to perform.
    fn apply(
        &mut self,
        transition: Option<ComposingTransition>,
        executor: &mut dyn ComposingEffectExecutor,
    ) {
        let Some(transition) = transition else { return };
        self.mirror(&transition);
        let settings = self.current_settings();
        for effect in &transition.effects {
            match effect {
                // Learning handshakes are not document effects; routed here so
                // the executor keeps its one job and the generation stays out
                // of the effect path.
                Effect::NextWordWordSelected { text, roman, .. } => {
                    self.learner
                        .word_selected(text, roman, &settings, self.current_generation);
                }
                Effect::NextWordUpdateLastSelectedWord { text, roman } => {
                    self.learner
                        .segment_nailed(text, roman, &settings, self.current_generation);
                }
                // Hides predictions while keeping the context. The desktop
                // shows no predictions, so there is nothing to hide and the
                // context is exactly what must survive.
                Effect::NextWordClearForNewComposing => {}
                Effect::UpdatePreedit(_)
                | Effect::ClearPreeditWithoutCommit
                | Effect::CommitTextReplacingPreedit(_)
                | Effect::DeleteBackwardFromDocument
                | Effect::ResetAutocomplete
                | Effect::PerformAutocomplete
                | Effect::ResetAutocompleteContext => executor.execute(effect),
            }
        }
    }

    /// Updates the mirror from an engine answer, without performing anything.
    /// The read paths use this directly: a fetch response still carries the
    /// authoritative composition state.
    fn mirror(&mut self, transition: &ComposingTransition) {
        self.is_composing = transition.is_composing;
        self.raw_input = transition.raw_input.clone();
        self.display_text = transition.display_text.clone();
    }

    fn clear_mirror(&mut self) {
        self.is_composing = false;
        self.raw_input.clear();
        self.display_text.clear();
    }
}
