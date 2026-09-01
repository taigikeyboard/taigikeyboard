//! What the two candidate operations answer: a query's three kinds of
//! nothing, and what a commit actually did. Port of `CandidateOutcomes.swift`.

// 中文: 候選查詢的三種「沒有」與候選送出的四種結果 — 從效果讀,不從鏡像讀。

use crate::engine::{ComposingTransition, ContinuousCandidate, Effect};

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

/// What committing a candidate did, read from the engine's effects rather
/// than from the composing mirror: a generation mismatch silently resets the
/// engine to Idle before the intent runs, which turns the commit into a noop
/// while flipping `is_composing` to false — so a mirror read reports "the
/// composition ended" for a commit that never happened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandidateCommitOutcome {
    /// The round-trip never reached the engine.
    Unavailable,
    /// The engine rejected the commit — a stale offset, or a composition
    /// that had already gone. Nothing changed.
    Ignored,
    /// The segment was nailed and the composition continues. Under Model B
    /// this writes nothing to the document (`transition.rs:864-889`).
    Nailed,
    /// The whole composition was consumed, written to the document in one
    /// mutation, and the engine returned to Idle (`transition.rs:842-861`).
    Finalized,
}

impl CandidateCommitOutcome {
    /// Model B leaves exactly one usable success signal per kind of commit: a
    /// final commit is the only one that writes text, and a nail is marked by
    /// the per-segment learning effect. A noop emits neither.
    pub fn from_transition(transition: &ComposingTransition) -> Self {
        let did_write_document = transition
            .effects
            .iter()
            .any(|effect| matches!(effect, Effect::CommitTextReplacingPreedit(_)));
        let did_nail = transition
            .effects
            .iter()
            .any(|effect| matches!(effect, Effect::NextWordUpdateLastSelectedWord { .. }));
        match (did_write_document, did_nail) {
            (true, _) => {
                if transition.is_composing {
                    Self::Nailed
                } else {
                    Self::Finalized
                }
            }
            (false, true) => Self::Nailed,
            (false, false) => Self::Ignored,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::CandidateMode;

    fn candidate(roman: &str) -> ContinuousCandidate {
        ContinuousCandidate {
            consumed_span_start: 0,
            consumed_span_end: 0,
            syllable_count: 1,
            display_text: roman.to_owned(),
            score: 0.0,
            form: 1,
            mode: CandidateMode::Unspecified,
            roman: roman.to_owned(),
            hanji: None,
            canonical_tl: roman.to_owned(),
        }
    }

    #[test]
    fn a_refetch_replaces_only_on_a_non_empty_answer() {
        let found = vec![candidate("tâi")];
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

    fn transition(effects: Vec<Effect>, is_composing: bool) -> ComposingTransition {
        ComposingTransition {
            raw_input: String::new(),
            display_text: String::new(),
            effects,
            selected_candidate_index: 0,
            is_composing,
        }
    }

    #[test]
    fn outcome_reads_effects_not_the_mirror() {
        assert_eq!(
            CandidateCommitOutcome::from_transition(&transition(
                vec![Effect::CommitTextReplacingPreedit("台語".into())],
                false
            )),
            CandidateCommitOutcome::Finalized
        );
        assert_eq!(
            CandidateCommitOutcome::from_transition(&transition(
                vec![Effect::NextWordUpdateLastSelectedWord {
                    text: "台".into(),
                    roman: "tâi".into()
                }],
                true
            )),
            CandidateCommitOutcome::Nailed
        );
        assert_eq!(
            CandidateCommitOutcome::from_transition(&transition(vec![], false)),
            CandidateCommitOutcome::Ignored,
            "a generation-reset noop flips is_composing without writing anything"
        );
    }
}
