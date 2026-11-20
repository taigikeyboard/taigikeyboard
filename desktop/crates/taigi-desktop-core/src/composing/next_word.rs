//! Where the manager reports what reaches next word without a composition.
//!
//! The engine decides WHAT is worth learning (`engine/nextword/src/decide.rs`)
//! and records it itself (user-data-engine-roadmap P9b). A composition's
//! commits reach it inside the composing call (`engine/dispatch/src/next_word_handshake.rs`).
//! The manager only reports what no composition carries
//! (`INVARIANT_NEXTWORD_LEARNING_DECISION_CONTRACT`, invariants §40).

use crate::engine::{self, RequestStamp};
use crate::settings::EngineSettings;

/// Where the reports go: the engine in production ([`EngineNextWord`]); a
/// recorder in the manager tests, which check what is reported and when.
pub trait NextWordPort: Send {
    /// `character` reached the document outside any composition (punctuation,
    /// a symbol), so the engine can end the context on sentence-end
    /// punctuation.
    fn character_typed(&self, character: &str, settings: &EngineSettings, stamp: RequestStamp);
    /// The context is gone: nothing that follows follows it.
    fn forget_context(&self, settings: &EngineSettings, stamp: RequestStamp);
}

/// The engine's next-word slice.
pub struct EngineNextWord;

impl NextWordPort for EngineNextWord {
    fn character_typed(&self, character: &str, settings: &EngineSettings, stamp: RequestStamp) {
        engine::nextword_character_typed(character, settings, stamp);
    }

    fn forget_context(&self, settings: &EngineSettings, stamp: RequestStamp) {
        engine::nextword_reset_all(settings, stamp);
    }
}
