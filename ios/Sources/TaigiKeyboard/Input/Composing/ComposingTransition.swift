import Foundation

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.

/// Platform-neutral description of what `ComposingState.apply(...)` decided.
///
/// The wrapper consumes this in three strictly-ordered phases (see
/// `composing-state-boundary.md` §2.4):
/// 1. mutate + publish state,
/// 2. execute `effects` in order against the platform adapter,
/// 3. notify the composing-context sink.
///
/// `Effect` names describe behavior, not iOS/Android APIs. The iOS adapter
/// maps them to `UITextDocumentProxy`; Android maps to `InputConnection`.
struct ComposingTransition: Equatable {
    enum Effect: Equatable {
        /// Show `text` as the current preedit (marked text).
        case updatePreedit(String)

        /// Clear the preedit region without committing its current contents
        /// to the text document.
        case clearPreeditWithoutCommit

        /// Atomically replace the current preedit with `text` in the text
        /// document. iOS: `clearMarkedText` + `insertText(text)`. Android:
        /// `commitText(text, 1)`.
        case commitTextReplacingPreedit(String)

        /// Delete one grapheme backward from the backing text document.
        case deleteBackwardFromDocument

        /// Engine-side autocomplete suggestion list should be cleared.
        case resetAutocomplete

        /// Engine-side autocomplete should run a fresh query based on the
        /// current composing buffer.
        case performAutocomplete

        /// Engine-side autocomplete context (selection / bigram history)
        /// should be reset.
        case resetAutocompleteContext
    }

    let newPhase: ComposingState.Phase
    let newSelectedIndex: Int
    let effects: [Effect]
    let derivedDisplay: String
}
