//! The composing orchestration: one user intent in, the engine mirror
//! updated and the engine's effects handed to the client that asked.
//!
//! Port of `macos/Sources/TaigiInputMethodCore/Composing/*.swift` +
//! `NextWord/NextWordPort.swift` + `Candidates/{CandidateDocumentText,
//! CandidateCellContent,CandidateScript}.swift`. The TSF shell implements
//! [`ComposingEffectExecutor`] over an edit session; the engine keeps the
//! user's data (picks reach it through [`UsageRecorder`]); tests record in
//! memory.

mod clock;
mod coordinator;
mod document_text;
mod manager;
mod next_word;
mod outcomes;
mod presentation;
mod usage;

pub use clock::{Clock, SystemClock};
pub use coordinator::{ComposingSessionCoordinator, ContextToken};
pub use document_text::{CandidateCellContent, CandidateScript, ResolvedCommit};
pub use manager::{ComposingEffectExecutor, ComposingManager};
pub use next_word::{EngineNextWord, NextWordPort};
pub use outcomes::{CandidateCommitOutcome, CandidateFetchOutcome, CandidateListChange};
pub use presentation::{CandidateSource, PresentedCandidate};
pub use usage::{EngineUsage, NoUsage, Usage, UsageRecorder};
