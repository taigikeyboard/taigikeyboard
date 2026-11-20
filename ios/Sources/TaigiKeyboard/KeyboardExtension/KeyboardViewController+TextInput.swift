import Foundation
import KeyboardKit
import UIKit

// MARK: - ComposingDelegate

extension KeyboardViewController {
    /// Translate a platform-neutral `RustEngineBridge.ComposingTransition.Effect`
    /// to the iOS `UITextDocumentProxy` surface. Binding contract (iOS +
    /// Android) is documented in `composing-state-boundary.md` §2.2.
    func execute(_ effect: RustEngineBridge.ComposingTransition.Effect) {
        logger.debug({
            let kind = switch effect {
            case let .updatePreedit(text):
                "updatePreedit len=\(text.count)"
            case .clearPreeditWithoutCommit:
                "clearPreeditWithoutCommit"
            case let .commitTextReplacingPreedit(text):
                "commitTextReplacingPreedit len=\(text.count)"
            case .clearCandidates:
                "clearCandidates"
            case .refreshCandidates:
                "refreshCandidates"
            case .resetCandidateContext:
                "resetCandidateContext"
            case let .nextWordDecided(result):
                "nextWordDecided effects=\(result.effects.count)"
            }
            return "[COMMIT] fn=execute effect=\(kind)"
        }())
        // Effects come from the composing manager the action handler owns, so
        // its host writer is always there when one arrives.
        switch effect {
        case let .updatePreedit(text):
            actionHandler?.hostText.update(text)
        case .clearPreeditWithoutCommit:
            actionHandler?.hostText.clear()
        case let .commitTextReplacingPreedit(text):
            // Replaces the preedit region atomically from the user's
            // perspective (Android achieves this via `commitText(text, 1)`).
            actionHandler?.hostText.commit(text)
        case .clearCandidates:
            resetAutocomplete()
        case .refreshCandidates:
            performAutocomplete()
        case .resetCandidateContext:
            state.autocompleteContext.reset()
        case let .nextWordDecided(result):
            // The composition's next-word handshake (a commit's learning, a
            // nail, an abort's clear), already applied by the engine in this
            // same call; run its answer here, in the engine's order.
            actionHandler?.nextWordController.applyComposingDecision(result)
        }
    }
}
