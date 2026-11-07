//! What the two candidate operations answer: a query's three kinds of
//! nothing, and what a commit actually did.

use protos::engine::{CommitOutcome, CommitResolution};

use crate::engine::ContinuousCandidate;

/// The answer to a candidate query. Three cases rather than an optional list
/// because the caller acts on each differently, and collapsing any two shows
/// the user a stale list.
#[derive(Clone, Debug, PartialEq)]
pub enum CandidateFetchOutcome {
    /// The round-trip never reached the engine.
    Unavailable,
    /// The engine answered, and it is not in the continuous phase.
    NotComposing,
    /// The engine answered. An empty list means it found nothing for a
    /// composition it IS holding — different from `NotComposing`.
    Found(Vec<ContinuousCandidate>),
}

/// What a fetch does to the list already on screen. The three kinds of
/// nothing and an empty `Found` all clear it: an empty list always takes the
/// window down, whichever way it came to be empty.
#[derive(Clone, Debug, PartialEq)]
pub enum CandidateListChange {
    /// A non-empty answer: show these in place of the old list.
    Replace(Vec<ContinuousCandidate>),
    /// Nothing to show: drop the list and the window with it.
    Clear,
}

impl CandidateFetchOutcome {
    /// Folds the outcome into the one decision every re-fetch of an open
    /// list makes — a key, a nail, or the display-mode cycle re-querying
    /// under the new mode (`ShortcutAction::CycleCandidateDisplayMode`).
    pub fn list_change(self) -> CandidateListChange {
        match self {
            Self::Found(candidates) if !candidates.is_empty() => {
                CandidateListChange::Replace(candidates)
            }
            Self::Found(_) | Self::Unavailable | Self::NotComposing => CandidateListChange::Clear,
        }
    }
}

/// The commit-and-insert key either wrote the whole composition and literal,
/// or changed nothing. Its spacing is already part of that one write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfirmCompositionThenInsertOutcome {
    Unavailable,
    Ignored,
    Finalized { leaves_trailing_auto_space: bool },
}

/// What committing a candidate did, as the engine reported it
/// (`ComposingResponse.commit`) — never read off the composing mirror: a
/// generation mismatch silently resets the engine to Idle before the intent
/// runs, which turns the commit into a noop while flipping `is_composing` to
/// false, so a mirror read reports "the composition ended" for a commit that
/// never happened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandidateCommitOutcome {
    /// The round-trip never reached the engine.
    Unavailable,
    /// The engine rejected the commit — a stale offset, a composition that
    /// had already gone, or the other script under Romanization Only.
    /// Nothing changed.
    Ignored,
    /// The segment was nailed and the composition continues. Under Model B
    /// this writes nothing to the document. `refetch` is whether the engine
    /// asked for the list again (`RefreshCandidates`): a nail lists what
    /// follows it, a pick under a TPS Hanji conversion answers
    /// `ClearCandidates` and the window closes (Hanji conversion H4).
    Nailed { refetch: bool },
    /// The whole composition was consumed, written to the document in one
    /// mutation, and the engine returned to Idle. `earns_auto_space` is the
    /// engine's §23 verdict on what the pick wrote (romanization, no
    /// trailing `-`); the live Auto-Space setting is still the caller's.
    Finalized { earns_auto_space: bool },
}

impl CandidateCommitOutcome {
    pub(crate) fn from_resolution(resolution: &CommitResolution, asks_refetch: bool) -> Self {
        match resolution.outcome() {
            CommitOutcome::Nailed => Self::Nailed {
                refetch: asks_refetch,
            },
            CommitOutcome::Finalized => Self::Finalized {
                earns_auto_space: resolution.earns_auto_space,
            },
            // UNSPECIFIED: an answer without a resolution changed nothing
            // this side can tell.
            CommitOutcome::Ignored | CommitOutcome::Unspecified => Self::Ignored,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::test_support::candidate;

    #[test]
    fn a_refetch_replaces_only_on_a_non_empty_answer() {
        let found = vec![candidate("tâi", None, 0)];
        assert_eq!(
            CandidateFetchOutcome::Found(found.clone()).list_change(),
            CandidateListChange::Replace(found)
        );
        assert_eq!(
            CandidateFetchOutcome::Found(Vec::new()).list_change(),
            CandidateListChange::Clear,
            "an empty answer takes the window down like the two kinds of nothing"
        );
        assert_eq!(
            CandidateFetchOutcome::Unavailable.list_change(),
            CandidateListChange::Clear
        );
        assert_eq!(
            CandidateFetchOutcome::NotComposing.list_change(),
            CandidateListChange::Clear
        );
    }

    fn resolution(outcome: CommitOutcome, earns_auto_space: bool) -> CommitResolution {
        CommitResolution {
            outcome: outcome as i32,
            earns_auto_space,
            ..CommitResolution::default()
        }
    }

    #[test]
    fn outcome_reads_the_engine_resolution() {
        assert_eq!(
            CandidateCommitOutcome::from_resolution(
                &resolution(CommitOutcome::Finalized, true),
                false
            ),
            CandidateCommitOutcome::Finalized {
                earns_auto_space: true
            }
        );
        assert_eq!(
            CandidateCommitOutcome::from_resolution(
                &resolution(CommitOutcome::Nailed, false),
                true
            ),
            CandidateCommitOutcome::Nailed { refetch: true }
        );
        assert_eq!(
            CandidateCommitOutcome::from_resolution(
                &resolution(CommitOutcome::Ignored, false),
                false
            ),
            CandidateCommitOutcome::Ignored
        );
        assert_eq!(
            CandidateCommitOutcome::from_resolution(&CommitResolution::default(), false),
            CandidateCommitOutcome::Ignored,
            "an answer with no resolution is not a commit"
        );
    }
}
