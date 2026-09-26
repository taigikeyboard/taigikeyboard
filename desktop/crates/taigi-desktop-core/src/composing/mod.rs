//! The composing orchestration: one user intent in, the engine mirror
//! updated and the engine's effects handed to the client that asked.
//!
//! Port of `macos/Sources/TaigiInputMethodCore/Composing/*.swift` +
//! `NextWord/NextWordLearner.swift` + `Candidates/{CandidateDocumentText,
//! CandidateCellContent,CandidateScript}.swift`. The TSF shell implements
//! [`ComposingEffectExecutor`] over an edit session; the engine keeps the
//! user's data (picks reach it through [`UsageRecorder`]); tests record in
//! memory.

mod clock;
mod coordinator;
mod document_text;
mod learner;
mod manager;
mod outcomes;
mod presentation;
mod usage;

pub use clock::{Clock, SystemClock};
pub use coordinator::{ComposingSessionCoordinator, ContextToken};
pub use document_text::{CandidateCellContent, CandidateScript, ResolvedCommit};
pub use learner::NextWordLearner;
pub use manager::{ComposingEffectExecutor, ComposingManager};
pub use outcomes::{CandidateCommitOutcome, CandidateFetchOutcome, CandidateListChange};
pub use presentation::{CandidateSource, PresentedCandidate};
pub use usage::{EngineUsage, NoUsage, Usage, UsageRecorder};
// The learner's persistence seam, kept only so the desktop's own context
// rules stay testable: the engine records associations once the stores
// are open and strips the effect, so production passes `NoStores` —
// temporary, removed in user-data-engine-roadmap P9 (U9).
pub use userdata::{AssociationSink, NoStores};
