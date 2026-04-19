import Foundation

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.

/// Inputs to `NextWordEngine.decide`. The executor lowers every lifecycle
/// event into one of these cases so the engine stays pure (no Timer, no
/// clock, no @MainActor).
enum NextWordIntent: Equatable {
    /// User selected a candidate or committed composing text. `requireRomanMode`
    /// matches today's Enter-commits-raw-romanization path; `triggerPrediction`
    /// is false on Space.
    case wordSelected(text: String, roman: String, requireRomanMode: Bool, triggerPrediction: Bool)
    /// Backspace after a word selection — re-predict using the last remaining
    /// character but NEVER record an association.
    case backspace(lastChar: String)
    /// Executor's context-timeout Timer fired. Engine clears state and bumps
    /// generation so any in-flight prediction query is invalidated.
    case contextTimeoutFired
    /// User began composing a new syllable / digit — hide suggestions but keep
    /// association state intact (matches today's `clearDisplay`).
    case clearForNewComposing
    /// Full reset (sentence-end punctuation outside wordSelected, or empty document).
    case resetFull
}

/// Foundation-only settings snapshot passed into the engine. Captures only
/// the fields the engine actually reads so Rust/Kotlin ports have a minimal
/// obligation. The executor reads `EngineSettingsProvider.current` once per
/// intent and constructs this value.
struct NextWordEngineSettings: Equatable {
    let inputMode: InputMode
    let isTranslateSwapped: Bool
    let isAssociationRecordingEnabled: Bool
}

/// Long-lived state the engine mutates. Separate from `NextWordDecisionInput`
/// so the executor cannot accidentally persist a stale `nowMs` or settings.
struct NextWordPersistedState: Equatable {
    var lastSelectedWord: String?
    var lastSelectedRoman: String?
    var lastSelectionTimeMs: Int64
    var isShowing: Bool
    /// Monotonic counter bumped on every invalidating intent. The executor
    /// tags in-flight prediction queries with the generation at dispatch
    /// time; a mismatch at query resolution means the intent that launched
    /// the query has been superseded, and the result is dropped.
    var currentGeneration: UInt64

    static let initial = NextWordPersistedState(
        lastSelectedWord: nil,
        lastSelectedRoman: nil,
        lastSelectionTimeMs: 0,
        isShowing: false,
        currentGeneration: 0,
    )
}

/// Per-call snapshot supplied by the executor. The engine never reads a
/// clock or a settings provider itself — everything it needs arrives here.
struct NextWordDecisionInput: Equatable {
    let nowMs: Int64
    let settings: NextWordEngineSettings
}

/// Named DTO for a bigram association pair. Preferred over labeled tuples
/// so `NextWordOutcome` synthesizes `Equatable` cleanly and the shape ports
/// directly to Kotlin data classes / Rust structs.
struct NextWordAssociationPair: Equatable {
    let prev: String
    let prevTl: String
    let next: String
    let nextTl: String
}

/// Engine decision output. `effects` executes in order on the platform side.
struct NextWordOutcome: Equatable {
    enum Effect: Equatable {
        /// `after` is in seconds (Foundation `TimeInterval` = `Double`);
        /// Kotlin/Rust ports should read it as plain `Double` seconds.
        case rescheduleContextTimeout(after: TimeInterval)
        case cancelContextTimeout
        case recordAssociation(NextWordAssociationPair)
        case recordCompoundAssociations([NextWordAssociationPair])
        case queryPredictions(word: String, roman: String, generation: UInt64)
        /// `generation` is informational — the engine always bumps
        /// generation on the same intent that emits `clearPredictionsUI`,
        /// so the effect is current-generation by construction and the
        /// platform executor does not need to re-check it before clearing.
        case clearPredictionsUI(generation: UInt64)
    }

    let newState: NextWordPersistedState
    let effects: [Effect]
}
