import Foundation

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.

/// Engine-layer prediction value produced by `NextWordController` and
/// consumed by the KeyboardKit adapter (`ActionHandler`), which is the
/// only place that knows how to turn this into an `Autocomplete.Suggestion`.
///
/// Replaces `Autocomplete.Suggestion` on the engine side so NextWord
/// logic never needs to import KeyboardKit.
struct EnginePrediction: Equatable {
    /// Display text the user sees in the autocomplete bar.
    let text: String
    /// Optional secondary display (hanzi beneath roman, etc.).
    let subtitle: String?
    /// Han-jī form, if any.
    let hanzi: String
    /// Tâi-lô raw form (the bigram source's TL string).
    let tl: String
}
