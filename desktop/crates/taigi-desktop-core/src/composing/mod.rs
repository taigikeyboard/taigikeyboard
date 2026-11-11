//! The composing orchestration: one user intent in, the engine mirror
//! updated and the engine's effects handed to the client that asked.
//!
//! The one copy the three desktops run; macOS reaches it through
//! `taigi-macos-ffi`. The TSF shell implements
//! [`ComposingEffectExecutor`] over an edit session; the engine resolves what
//! a pick writes and keeps the user's data (it counts the picks itself);
//! tests record in memory.

mod cell_content;
mod clock;
mod coordinator;
mod intent_executor;
mod manager;
mod next_word;
mod outcomes;
mod presentation;

pub use cell_content::{CandidateCellContent, CandidateScript};
pub use clock::{Clock, SystemClock};
pub use coordinator::{ComposingSessionCoordinator, ContextToken};
pub use intent_executor::{
    insert_symbol, pass_through_may_consume, pass_through_may_swap, perform_intent, refresh_list,
    represent_list, IntentSurface,
};
pub use manager::{ComposingEffectExecutor, ComposingManager, TpsKeyOutcome};
pub use next_word::{EngineNextWord, NextWordPort};
pub use outcomes::{
    CandidateCommitOutcome, CandidateFetchOutcome, CandidateListChange,
    ConfirmCompositionThenInsertOutcome,
};
pub use presentation::{CandidateSource, PresentedCandidate};
