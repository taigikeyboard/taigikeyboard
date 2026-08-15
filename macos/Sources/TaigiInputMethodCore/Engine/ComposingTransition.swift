// Swift-side value types for one composing round-trip: engine snapshot, the
// ordered effects it wants the platform to run, and the continuous candidates.

import Foundation

/// One engine response, decoded. `effects` is the whole reason this type exists:
/// the engine decides *what* happens to the host document and in what order, and
/// the platform only decides *how* to perform each step.
struct ComposingTransition: Equatable, Sendable {
    /// The complete effect vocabulary of `engine/protos/proto/composing.proto`
    /// `Effect`. All ten cases are decoded even though PR3 acts on only some of
    /// them: an effect that is silently dropped at decode time is invisible when
    /// the slice that needs it lands, whereas an explicitly ignored case shows
    /// up in every future `switch` the compiler checks.
    enum Effect: Equatable, Sendable {
        case updatePreedit(String)
        case clearPreeditWithoutCommit
        case commitTextReplacingPreedit(String)
        /// Emitted only by the `Phase::Composing` backspace-to-empty branch
        /// (`engine/composing/src/transition.rs:266-276`); the continuous branch
        /// never emits it (`transition.rs:289-292`).
        case deleteBackwardFromDocument
        case resetAutocomplete
        case performAutocomplete
        case resetAutocompleteContext
        /// Continuous-input mid-commit handshake for the next-word learner.
        case nextWordUpdateLastSelectedWord(text: String, roman: String)
        /// Continuous-input final-commit handshake. `triggerPrediction` is the
        /// engine's decision — forward it, never hardcode it.
        case nextWordWordSelected(text: String, roman: String, triggerPrediction: Bool)
        /// Continuous-input abort handshake. Distinct from a full next-word
        /// reset; they are different engine intents.
        case nextWordClearForNewComposing
    }

    /// The keystrokes as typed, with numeric tones (the engine's search key).
    let rawInput: String
    /// The rendered composition, with tone diacritics (what the user reads).
    let displayText: String
    let effects: [Effect]
    let selectedCandidateIndex: Int
    let isComposing: Bool
}

/// MOE-aligned candidate-type discriminator, derived in Rust
/// (`engine/lexicon/src/continuous.rs::derive_mode`). Platforms read it and
/// never recompute it — sniffing the display text would be a second, drifting
/// implementation of the same rule.
enum CandidateMode: Equatable, Sendable {
    /// The wire carried no mode, or one this build does not know. Means
    /// "ignore mode", never "guess the mode locally".
    case unspecified
    case hant
    case tailo
    case mixed

    static func decode(_ wire: Int) -> CandidateMode {
        switch wire {
        case 1: .hant
        case 2: .tailo
        case 3: .mixed
        default: .unspecified
        }
    }
}

/// One span-local continuous-input candidate.
///
/// The span offsets are byte offsets into the raw buffer the engine holds, and
/// `canonicalTl` is the identity romanization (Core Principle #7 keys a word on
/// the `(漢字, canonical TL)` pair). Both must be round-tripped back to the
/// engine verbatim on commit: recomputing either from `displayText` would key
/// the user's frequency and association data under a different word.
struct ContinuousCandidate: Equatable, Sendable {
    let consumedSpanStart: UInt32
    let consumedSpanEnd: UInt32
    let syllableCount: UInt32
    let displayText: String
    let score: Float
    let form: UInt32
    let mode: CandidateMode
    /// Display romanization for the candidate cell — POJ-rendered in POJ mode.
    /// Not an identity key; that is `canonicalTl`.
    let roman: String
    /// `nil` when the wire omitted it, which marks a romanization-only
    /// candidate. Distinct from an empty string.
    let hanji: String?
    /// Canonical TL, snapshotted before any display recasing. Empty only when
    /// no canonical TL is recoverable.
    let canonicalTl: String
}

/// Result of the read-only candidate query.
///
/// The query returns this optionally, and the two "nothing here" answers are
/// deliberately different types of nothing: a `nil` result means the round-trip
/// never reached the engine, whose state is therefore whatever it already was,
/// while `candidates == nil` means the engine answered and reported that it is
/// not in the continuous phase. Merging them corrupts state — applying a
/// synthesized snapshot for a call that never happened would tell the UI a
/// composition ended that is in fact still running.
///
/// `candidates == []` means the engine was in the continuous phase and found
/// nothing.
struct ContinuousFetchResult: Equatable, Sendable {
    let transition: ComposingTransition
    let candidates: [ContinuousCandidate]?
}
