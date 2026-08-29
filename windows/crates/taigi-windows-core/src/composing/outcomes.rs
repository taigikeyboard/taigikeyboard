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
