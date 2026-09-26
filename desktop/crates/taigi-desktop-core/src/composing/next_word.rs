//! Where the manager reports commits for next-word learning. Port of
//! `NextWordPort.swift`.
//!
//! The engine decides WHAT is worth learning — the 10-second window, how a
//! compound splits, whether the text is noise (`engine/nextword/src/decide.rs`)
//! — and records it itself (user-data-engine-roadmap P9b). The manager only
//! reports the commits (`INVARIANT_NEXTWORD_LEARNING_DECISION_CONTRACT`,
//! invariants §40).

use crate::engine;
use crate::settings::EngineSettings;

/// Where the handshakes go: the engine in production ([`EngineNextWord`]); a
/// recorder in the manager tests, which check what is reported and when.
pub trait NextWordPort: Send {
    /// The user finalized `text` into the document.
    fn word_selected(
        &self,
        text: &str,
        roman: &str,
        now_ms: i64,
        settings: &EngineSettings,
        generation: u64,
    );
    /// A continuous composition nailed a segment without finalizing.
    fn segment_nailed(
        &self,
        text: &str,
        roman: &str,
        now_ms: i64,
        settings: &EngineSettings,
        generation: u64,
    );
    /// The context is gone: nothing that follows follows it.
    fn forget_context(&self, now_ms: i64, settings: &EngineSettings, generation: u64);
}

/// The engine's next-word slice.
pub struct EngineNextWord;

impl NextWordPort for EngineNextWord {
    fn word_selected(
        &self,
        text: &str,
        roman: &str,
        now_ms: i64,
        settings: &EngineSettings,
        generation: u64,
    ) {
        engine::nextword_word_selected(text, roman, now_ms, settings, generation);
    }

    fn segment_nailed(
        &self,
        text: &str,
        roman: &str,
        now_ms: i64,
        settings: &EngineSettings,
        generation: u64,
    ) {
        engine::nextword_update_last_selected_word(text, roman, now_ms, settings, generation);
    }

    fn forget_context(&self, now_ms: i64, settings: &EngineSettings, generation: u64) {
        engine::nextword_reset_full(now_ms, settings, generation);
    }
}
