//! The composing orchestration: one user intent in, the engine mirror
//! updated and the engine's effects handed to the client that asked.
//!
//! Port of `macos/Sources/TaigiInputMethodCore/Composing/*.swift` +
//! `NextWord/NextWordLearner.swift` + `Candidates/{CandidateDocumentText,
//! CandidateCellContent,CandidateScript}.swift`. The TSF shell implements
//! [`ComposingEffectExecutor`] over an edit session; the engine `userdata`
//! implements the three store traits; tests implement all of them in memory.

mod clock;
mod coordinator;
mod document_text;
mod learner;
mod manager;
mod outcomes;
mod presentation;

pub use clock::{Clock, SystemClock};
pub use coordinator::{ComposingSessionCoordinator, ContextToken};
pub use document_text::{CandidateCellContent, CandidateScript, ResolvedCommit};
pub use learner::NextWordLearner;
pub use manager::{ComposingEffectExecutor, ComposingManager};
pub use outcomes::{CandidateCommitOutcome, CandidateFetchOutcome, CandidateListChange};
pub use presentation::{CandidateSource, PresentedCandidate};
// The store seams moved to the engine `userdata` crate (user-data-engine-roadmap
// P1); re-exported here until the desktop switch (P5) drops them.
pub use userdata::{
    AssociationSink, CustomDictionarySource, FrequencySource, LearnedPhraseSource, NoStores,
};
