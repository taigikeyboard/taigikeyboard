//! The composing orchestration: one user intent in, the engine mirror
//! updated and the engine's effects handed to the client that asked.
//!
//! Port of `macos/Sources/TaigiInputMethodCore/Composing/*.swift` +
//! `NextWord/NextWordLearner.swift` + `Candidates/{CandidateDocumentText,
//! CandidateCellContent,CandidateScript}.swift`. The TSF shell implements
//! [`ComposingEffectExecutor`] over an edit session; the storage crate
//! implements the three store traits; tests implement all of them in memory.

mod coordinator;
mod document_text;
mod learner;
mod manager;
mod outcomes;
mod presentation;
mod stores;

pub use coordinator::{ComposingSessionCoordinator, ContextToken};
pub use document_text::{CandidateCellContent, CandidateScript, ResolvedCommit};
pub use learner::NextWordLearner;
pub use manager::{ComposingEffectExecutor, ComposingManager};
pub use outcomes::{CandidateCommitOutcome, CandidateFetchOutcome, CandidateListChange};
pub use presentation::{CandidateSource, PresentedCandidate};
pub use stores::{
    AssociationSink, Clock, CustomDictionarySource, FrequencySource, NoStores, SystemClock,
};
