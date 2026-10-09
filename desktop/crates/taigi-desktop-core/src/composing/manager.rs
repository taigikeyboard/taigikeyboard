//! Turns a user intent into an engine round-trip, mirrors what came back, and
//! hands the engine's effects to the executor for the client that asked.
//!
//! The engine owns the composition (phase, raw buffer, candidate index); this
//! owns only a mirror of the last answer, which the controller reads to
//! decide whether a key belongs to the composition or to the host.

use std::sync::Arc;

use super::cell_content::CandidateScript;
use super::clock::Clock;
use super::next_word::NextWordPort;
use super::outcomes::{CandidateCommitOutcome, CandidateFetchOutcome};
use super::presentation::{leads_with_literal_roman, presentation, PresentedCandidate};
use crate::engine::{self, CommitContinuousArgs, ComposingTransition, ContinuousCandidate, Effect};
use crate::keys::CaretDirection;
use crate::settings::{EngineSettings, InputMode, SettingsProvider};

/// Writes the engine's document effects into the client that is currently
/// focused. Learning handshakes never reach it — the manager reports them to
/// the next-word port, because they write to a database rather than a document.
pub trait ComposingEffectExecutor {
    fn execute(&mut self, effect: &Effect);
}

/// What became of one TPS key (`ComposingManager::tps_key`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TpsKeyOutcome {
    /// The engine typed it.
    Taken,
    /// The engine refused it — only a Space does: the syllable before the
    /// caret is already closed, or a tone mark or separator sits beside the
    /// caret, or nothing precedes it. `is_caret_at_end` tells a closed last
    /// syllable from a refusal inside the composition.
    Refused { is_caret_at_end: bool },
    /// The round trip failed; nothing changed.
    Failed,
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
    /// Where the caret sits in `display_text`, as a UTF-16 offset — the last
    /// engine answer's, which a list re-mirrors before it is shown. Under TPS
    /// the candidate window lists the word before it, so the macOS window
    /// anchors there (`taigi-macos-ffi` `RecordingSurface::finish`).
    display_caret_utf16: u32,
    /// The input mode the composition in flight began under; `None` while
    /// idle. A switch across TPS changes the raw buffer's alphabet, so the
    /// composition it leaves behind cannot take the new mode's keys
    /// (`is_left_by_mode_change`).
    composition_input_mode: Option<InputMode>,
    /// The settings the preedit on screen was last written under; `None`
    /// while idle. A TPS composition is listed, picked from and committed
    /// under them (`composition_settings`): the Hanji the preedit shows is
    /// the conversion those settings asked for.
    preedit_settings: Option<EngineSettings>,
    settings: Arc<dyn SettingsProvider>,
    /// Where the next-word handshakes go, stamped with `clock`.
    next_word: Box<dyn NextWordPort>,
    clock: Box<dyn Clock>,
    /// Unique across everything that talks to the engine in this process:
    /// the engine keeps one composition per process and drops it whenever the
    /// generation it is handed changes.
    current_generation: u64,
}

impl ComposingManager {
    /// `starting_generation` defaults to 1 in production because 0 is the
    /// generation an unset proto field carries.
    pub fn new(
        settings: Arc<dyn SettingsProvider>,
        next_word: Box<dyn NextWordPort>,
        clock: Box<dyn Clock>,
        starting_generation: u64,
    ) -> Self {
        Self {
            is_composing: false,
            raw_input: String::new(),
            display_text: String::new(),
            display_caret_utf16: 0,
            composition_input_mode: None,
            preedit_settings: None,
            settings,
            next_word,
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

    pub fn display_caret_utf16(&self) -> u32 {
        self.display_caret_utf16
    }

    pub fn generation(&self) -> u64 {
        self.current_generation
    }

    fn current_settings(&self) -> EngineSettings {
        self.settings.current().engine_settings()
    }

    /// Whether the composition in flight was typed on the other side of TPS
    /// from `mode`, the mode in force — glyphs under a romanization, or a
    /// romanization under TPS — after a switch that did not end it. The next
    /// key commits it first (`commit_composition_left_by_mode_change`).
    /// `mode` comes from the caller's own settings snapshot, the one the key
    /// is read under.
    pub fn is_left_by_mode_change(&self, mode: InputMode) -> bool {
        self.composition_input_mode
            .is_some_and(|typed_in| typed_in.crosses_tps(mode))
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
        self.next_word.forget_context(
            self.clock.now_ms(),
            &self.current_settings(),
            self.current_generation,
        );
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
        self.next_word.word_selected(
            character,
            "",
            &[],
            self.clock.now_ms(),
            &self.current_settings(),
            self.current_generation,
        );
    }

    /// Appends one typed character. The engine starts a composition when idle,
    /// so there is no separate "begin" call — one engine request per key.
    pub fn append(&mut self, character: &str, executor: &mut dyn ComposingEffectExecutor) {
        log::debug!("append");
        let settings = self.current_settings();
        let transition = engine::append(character, &settings, self.current_generation);
        self.apply(transition, &settings, executor);
    }

    /// Applies one Telex key — a tone letter, `z` or `f` — to the pending
    /// syllable. Shaped like `append` because it is the same step with the
    /// engine deciding what the key writes (`engine/composing/src/telex.rs`):
    /// an idle `z` starts a composition the way a letter does.
    pub fn telex_key(&mut self, key: &str, executor: &mut dyn ComposingEffectExecutor) {
        log::debug!("telexKey");
        let settings = self.current_settings();
        let transition = engine::telex_key(key, &settings, self.current_generation);
        self.apply(transition, &settings, executor);
    }

    /// Types one TPS key — a glyph, a tone mark, the hyphen, or `" "` for the
    /// Space separator — and answers what became of it, so the caller can
    /// give a refused Space its own meaning (`desktop-tps-roadmap.md` § D3).
    pub fn tps_key(
        &mut self,
        key: &str,
        executor: &mut dyn ComposingEffectExecutor,
    ) -> TpsKeyOutcome {
        log::debug!("tpsKey");
        let settings = self.current_settings();
        let Some(transition) = engine::tps_key(key, &settings, self.current_generation) else {
            return TpsKeyOutcome::Failed;
        };
        let outcome = if !transition.effects.is_empty() {
            TpsKeyOutcome::Taken
        } else {
            let display_length = transition.display_text.encode_utf16().count();
            TpsKeyOutcome::Refused {
                is_caret_at_end: transition.caret_utf16 as usize == display_length,
            }
        };
        self.apply(Some(transition), &settings, executor);
        outcome
    }

    /// Drops the last character of the raw buffer. Ends the composition when
    /// that empties it.
    pub fn delete_backward(&mut self, executor: &mut dyn ComposingEffectExecutor) {
        log::debug!("deleteBackward");
        let settings = self.current_settings();
        let transition = engine::delete_backward(&settings, self.current_generation);
        self.apply(transition, &settings, executor);
    }

    /// Steps the caret inside the pending tail. Not a buffer change: the
    /// engine asks for no fetch — the candidates on
    /// screen still describe the same text.
    pub fn move_caret(
        &mut self,
        direction: CaretDirection,
        executor: &mut dyn ComposingEffectExecutor,
    ) {
        log::debug!("moveCaret");
        let settings = self.current_settings();
        let transition = engine::move_caret(direction, &settings, self.current_generation);
        self.apply(transition, &settings, executor);
    }

    /// Commits the composition as rendered: `CommitRaw` for TL and POJ, and
    /// for a TPS composition `CommitAsShown` — the Hanji conversion on screen
    /// (`desktop-tps-hanji-conversion-roadmap.md` B2). Answers the text the
    /// commit wrote, or `None` for a commit that never reached the engine or
    /// wrote nothing — what auto-space earns its trailing space from.
    pub fn commit_composition(
        &mut self,
        executor: &mut dyn ComposingEffectExecutor,
    ) -> Option<String> {
        log::debug!("commitComposition");
        let settings = self.composition_settings();
        let commit = if self.is_tps_composition() {
            engine::commit_as_shown
        } else {
            engine::commit_raw
        };
        let transition = commit(&settings, self.current_generation);
        self.finish_commit(transition, &settings, executor)
    }

    /// Commits a TPS composition as typed (`CommitAsTyped`): the glyphs of
    /// the whole composition, picks included. Teaches nothing.
    pub fn commit_composition_as_typed(
        &mut self,
        executor: &mut dyn ComposingEffectExecutor,
    ) -> Option<String> {
        log::debug!("commitCompositionAsTyped");
        let settings = self.composition_settings();
        let transition = engine::commit_as_typed(&settings, self.current_generation);
        self.finish_commit(transition, &settings, executor)
    }

    /// Commits the composition and appends `text` after it in the same engine
    /// step, so one keystroke reaches the host as one document mutation. A
    /// TPS composition is written as shown, as `commit_composition` writes it.
    pub fn commit_composition_then_insert(
        &mut self,
        text: &str,
        executor: &mut dyn ComposingEffectExecutor,
    ) -> Option<String> {
        log::debug!("commitCompositionThenInsert");
        let settings = self.composition_settings();
        let transition =
            engine::commit_preedit_then_insert_external(text, &settings, self.current_generation);
        self.finish_commit(transition, &settings, executor)
    }

    /// Applies a commit's answer and returns the text it wrote. A commit that
    /// teaches next word nothing — `NextWordClearForNewComposing` with no
    /// `NextWordWordSelected`: commit-then-insert always, a TPS composition
    /// holding a word the user did not pick (H7), commit as typed — leaves the
    /// context of the commit BEFORE it, which the next commit would pair
    /// itself with; the context is forgotten, under-learning one pair rather
    /// than learning a wrong one. TL and POJ's `CommitRaw` always teaches.
    fn finish_commit(
        &mut self,
        transition: Option<ComposingTransition>,
        settings: &EngineSettings,
        executor: &mut dyn ComposingEffectExecutor,
    ) -> Option<String> {
        let committed = Self::committed_text(transition.as_ref());
        let teaches_nothing = transition.as_ref().is_some_and(|transition| {
            transition
                .effects
                .contains(&Effect::NextWordClearForNewComposing)
                && !transition
                    .effects
                    .iter()
                    .any(|effect| matches!(effect, Effect::NextWordWordSelected { .. }))
        });
        self.apply(transition, settings, executor);
        if teaches_nothing {
            self.next_word
                .forget_context(self.clock.now_ms(), settings, self.current_generation);
        }
        committed
    }

    /// Whether the composition in flight was begun under TPS — the one
    /// answer to "is this a TPS composition", whatever mode is in force now.
    pub fn is_tps_composition(&self) -> bool {
        self.composition_input_mode == Some(InputMode::Tps)
    }

    /// The settings a request that reads or ends the composition on screen
    /// goes out under — a fetch, a pick, a commit. A TPS composition takes
    /// the settings its preedit was last written under: a conversion is
    /// shown only to the config it was walked for, and the engine resolves a
    /// list's start from it, so after a switch left the composition behind
    /// (`is_left_by_mode_change`) or the dictionary switches changed, the
    /// list, the pick and the commit still describe the Hanji on screen (a
    /// pick under other settings could be resolved against another start and
    /// swallow the words before it). The next key redraws under the settings
    /// in force. Any other composition takes the settings in force.
    fn composition_settings(&self) -> EngineSettings {
        match &self.preedit_settings {
            Some(drawn) if self.is_tps_composition() => drawn.clone(),
            _ => self.current_settings(),
        }
    }

    /// Abandons the composition. Nothing reaches the document.
    pub fn cancel_composition(&mut self, executor: &mut dyn ComposingEffectExecutor) {
        log::debug!("cancelComposition");
        let transition = engine::reset(self.current_generation);
        self.apply(transition, &self.current_settings(), executor);
    }

    // MARK: - Candidates

    /// Reads the candidates for the composition as it stands, ranked against
    /// what the user has committed before. One fetch: the engine reads the
    /// user's own data — frequency, custom dictionary, learned phrases —
    /// and ranks with it itself (user-data-engine-roadmap P3b / P5).
    pub fn fetch_candidates(&mut self) -> CandidateFetchOutcome {
        let settings = self.composition_settings();
        let Some(fetched) =
            engine::fetch_at_pos(&settings, self.current_generation, self.clock.now_ms())
        else {
            return CandidateFetchOutcome::Unavailable;
        };
        self.mirror(&fetched.transition, settings.input_mode);
        match fetched.candidates {
            Some(candidates) => CandidateFetchOutcome::Found(candidates),
            None => CandidateFetchOutcome::NotComposing,
        }
    }

    /// The cells the window shows for `candidates`, under one snapshot of the
    /// settings in force right now — Combined splits a candidate into two, so the
    /// window's indices are cell indices (`CandidateSource::resolve`) — plus
    /// whether the first cell is the §34 literal, which takes no slot key.
    ///
    /// ONE snapshot for BOTH answers, so the cells and the key row can never
    /// be resolved against two different instants.
    pub fn presentation(
        &self,
        candidates: &[ContinuousCandidate],
    ) -> (Vec<PresentedCandidate>, bool) {
        let settings = self.current_settings();
        (
            presentation(candidates, &settings),
            leads_with_literal_roman(candidates, &settings),
        )
    }

    /// Commits `candidate`, which must come from the `fetch_candidates` call
    /// that produced the list the user is looking at. `script` picks WHICH of
    /// the candidate's two renderings the document gets; the engine resolves
    /// the text under the same settings snapshot, answers `Ignored` for a
    /// script the candidate does not have (Space on a one-script cell), and —
    /// with the user data open — counts the pick under its `(display text,
    /// canonical TL)` identity (Core Principle #7), never the rendering.
    pub fn commit_candidate(
        &mut self,
        candidate: &ContinuousCandidate,
        script: CandidateScript,
        executor: &mut dyn ComposingEffectExecutor,
    ) -> CandidateCommitOutcome {
        let settings = self.composition_settings();
        log::debug!(
            "commitCandidate consumedBytes={}",
            candidate.consumed_span_end
        );
        let Some(committed) = engine::commit_continuous(
            &CommitContinuousArgs {
                script: match script {
                    CandidateScript::Primary => engine::CommitScript::Lead,
                    CandidateScript::Alternate => engine::CommitScript::Other,
                },
                roman: &candidate.roman,
                canonical_text: &candidate.display_text,
                association_tl: &candidate.canonical_tl,
                hanji: candidate.hanji.as_deref(),
                consumed_bytes: candidate.consumed_span_end,
                syllable_count: candidate.syllable_count,
            },
            &settings,
            self.current_generation,
        ) else {
            return CandidateCommitOutcome::Unavailable;
        };
        let asks_refetch = committed
            .transition
            .effects
            .contains(&Effect::RefreshCandidates);
        let outcome = CandidateCommitOutcome::from_resolution(&committed.commit, asks_refetch);
        self.apply(Some(committed.transition), &settings, executor);
        outcome
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

    /// Mirror first, then run the effects in the order the engine listed
    /// them. `None` is a round-trip that never reached the engine: nothing to
    /// mirror and nothing to perform. `settings` is the snapshot the request
    /// ran under, so the mode a composition is recorded as begun in is the
    /// one its first key was typed in.
    fn apply(
        &mut self,
        transition: Option<ComposingTransition>,
        settings: &EngineSettings,
        executor: &mut dyn ComposingEffectExecutor,
    ) {
        let Some(transition) = transition else { return };
        self.mirror(&transition, settings.input_mode);
        let writes_preedit = transition
            .effects
            .iter()
            .any(|effect| matches!(effect, Effect::UpdatePreedit { .. }));
        if transition.is_composing && writes_preedit && settings.input_mode == InputMode::Tps {
            self.preedit_settings = Some(settings.clone());
        }
        for effect in &transition.effects {
            match effect {
                // Learning handshakes are not document effects; routed here so
                // the executor keeps its one job and the generation stays out
                // of the effect path.
                Effect::NextWordWordSelected {
                    text,
                    roman,
                    preceding,
                    ..
                } => {
                    self.next_word.word_selected(
                        text,
                        roman,
                        preceding,
                        self.clock.now_ms(),
                        settings,
                        self.current_generation,
                    );
                }
                Effect::NextWordUpdateLastSelectedWord { text, roman } => {
                    self.next_word.segment_nailed(
                        text,
                        roman,
                        self.clock.now_ms(),
                        settings,
                        self.current_generation,
                    );
                }
                // Hides predictions while keeping the context. The desktop
                // shows no predictions, so there is nothing to hide and the
                // context is exactly what must survive.
                Effect::NextWordClearForNewComposing => {}
                Effect::UpdatePreedit { .. }
                | Effect::ClearPreeditWithoutCommit
                | Effect::CommitTextReplacingPreedit(_)
                | Effect::ClearCandidates
                | Effect::RefreshCandidates
                | Effect::ResetCandidateContext => executor.execute(effect),
            }
        }
    }

    /// Updates the mirror from an engine answer, without performing anything.
    /// The read paths use this directly: a fetch response still carries the
    /// authoritative composition state.
    /// `input_mode` is the mode of the snapshot the caller already holds;
    /// it is recorded only when a composition begins.
    fn mirror(&mut self, transition: &ComposingTransition, input_mode: InputMode) {
        self.is_composing = transition.is_composing;
        self.raw_input = transition.raw_input.clone();
        self.display_text = transition.display_text.clone();
        self.display_caret_utf16 = transition.caret_utf16;
        if transition.is_composing {
            self.composition_input_mode = self.composition_input_mode.or(Some(input_mode));
        } else {
            self.composition_input_mode = None;
            self.preedit_settings = None;
        }
    }

    fn clear_mirror(&mut self) {
        self.is_composing = false;
        self.raw_input.clear();
        self.display_text.clear();
        self.display_caret_utf16 = 0;
        self.composition_input_mode = None;
        self.preedit_settings = None;
    }
}
