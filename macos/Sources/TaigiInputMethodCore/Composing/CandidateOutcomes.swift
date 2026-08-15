// What the two candidate operations answer: a query's three kinds of nothing,
// and what a commit actually did to the document.

import Foundation

/// The answer to a candidate query.
///
/// Three cases rather than an optional array because the caller acts on each
/// differently, and collapsing any two of them shows the user a stale list. A
/// failed round-trip left the engine exactly as it was, so the candidates on
/// screen may still be describing an older buffer; an engine that answers "not
/// composing" is authoritative that there is nothing to show.
enum CandidateFetchOutcome: Equatable {
    /// The round-trip never reached the engine.
    case unavailable
    /// The engine answered, and it is not in the continuous phase.
    case notComposing
    /// The engine answered. An empty array means it found nothing for a
    /// composition it IS holding, which is a different thing from `notComposing`.
    case found([ContinuousCandidate])
}

/// What committing a candidate did.
///
/// Read from the engine's effects rather than from the composing mirror. A
/// generation mismatch silently resets the engine to Idle before the intent
/// runs (`engine/composing/src/handle.rs:61-66`), which turns the commit into a
/// phase-mismatch noop while flipping `isComposing` to false — so a mirror read
/// reports "the composition ended" for a commit that never happened. iOS learnt
/// this the same way (`ios/…/Input/Composing/ComposingManager.swift:465-480`).
enum CandidateCommitOutcome: Equatable {
    /// The round-trip never reached the engine.
    case unavailable
    /// The engine rejected the commit — a stale byte offset, or a composition
    /// that had already gone. Nothing changed.
    case ignored
    /// The segment was nailed and the composition continues. Under Model B this
    /// writes nothing to the document; the marked region is re-rendered with the
    /// nailed prefix in front of the remaining tail (`transition.rs:864-889`).
    case nailed
    /// The whole composition was consumed, written to the document in one
    /// mutation, and the engine returned to Idle (`transition.rs:842-861`).
    case finalized

    /// Model B leaves exactly one usable success signal per kind of commit: a
    /// final commit is the only one that writes text, and a nail is marked by
    /// the per-segment learning effect. A noop emits neither.
    init(_ transition: ComposingTransition) {
        let didWriteDocument = transition.effects.contains { effect in
            if case .commitTextReplacingPreedit = effect { return true }
            return false
        }
        let didNail = transition.effects.contains { effect in
            if case .nextWordUpdateLastSelectedWord = effect { return true }
            return false
        }
        switch (didWriteDocument, didNail) {
        case (true, _): self = transition.isComposing ? .nailed : .finalized
        case (false, true): self = .nailed
        case (false, false): self = .ignored
        }
    }
}
