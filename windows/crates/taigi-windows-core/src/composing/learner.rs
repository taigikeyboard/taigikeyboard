//! Turns commit handshakes into learned bigrams. Port of `NextWordLearner.swift`.
//!
//! The engine decides WHAT is worth learning — the 10-second window, how a
//! compound splits, whether the text is noise (`engine/nextword/src/decide.rs`)
//! — and this only carries the answer to the store
//! (`INVARIANT_NEXTWORD_LEARNING_DECISION_CONTRACT`, invariants §40).

use super::stores::{AssociationSink, Clock};
use crate::engine::{self, NextWordEffect, NextWordOutcome};
use crate::settings::EngineSettings;

pub struct NextWordLearner {
    store: Box<dyn AssociationSink>,
    clock: Box<dyn Clock>,
}

impl NextWordLearner {
    pub fn new(store: Box<dyn AssociationSink>, clock: Box<dyn Clock>) -> Self {
        Self { store, clock }
    }

    /// The user finalized `text` into the document.
    pub fn word_selected(
        &self,
        text: &str,
        roman: &str,
        settings: &EngineSettings,
        generation: u64,
    ) {
        self.apply(engine::nextword_word_selected(
            text,
            roman,
            self.clock.now_ms(),
            settings,
            generation,
        ));
    }

    /// A continuous composition nailed a segment without finalizing.
    pub fn segment_nailed(
        &self,
        text: &str,
        roman: &str,
        settings: &EngineSettings,
        generation: u64,
    ) {
        self.apply(engine::nextword_update_last_selected_word(
            text,
            roman,
            self.clock.now_ms(),
            settings,
            generation,
        ));
    }

    /// Drops the current context so nothing that follows is learned as
    /// having followed it.
    pub fn forget_context(&self, settings: &EngineSettings, generation: u64) {
        self.apply(engine::nextword_reset_full(
            self.clock.now_ms(),
            settings,
            generation,
        ));
    }

    /// `None` is a round-trip that never reached the engine: nothing was
    /// decided, so there is nothing to write. Counts only, never the words,
    /// reach the log (`security-rules.md`).
    fn apply(&self, outcome: Option<NextWordOutcome>) {
        let Some(outcome) = outcome else { return };
        for effect in &outcome.effects {
            match effect {
                NextWordEffect::RecordAssociation(pair) => {
                    log::debug!("recordAssociation");
                    self.store.record(std::slice::from_ref(pair));
                }
                NextWordEffect::RecordCompoundAssociations(pairs) => {
                    log::debug!("recordCompoundAssociations count={}", pairs.len());
                    self.store.record(pairs);
                }
            }
        }
    }
}
