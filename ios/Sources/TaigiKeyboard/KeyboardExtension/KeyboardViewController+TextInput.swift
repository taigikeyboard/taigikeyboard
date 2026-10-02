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
            case let .nextWordUpdateLastSelectedWord(text, roman):
                "nextWordUpdateLastSelectedWord text.len=\(text.count) roman.len=\(roman.count)"
            case let .nextWordWordSelected(text, roman, triggerPrediction, preceding):
                "nextWordWordSelected text.len=\(text.count) roman.len=\(roman.count) trigger=\(triggerPrediction) preceding=\(preceding.count)"
            case .nextWordClearForNewComposing:
                "nextWordClearForNewComposing"
            }
            return "[COMMIT] fn=execute effect=\(kind)"
        }())
        switch effect {
        case let .updatePreedit(text):
            markedText.update(text, on: textDocumentProxy)
        case .clearPreeditWithoutCommit:
            markedText.clear(on: textDocumentProxy)
        case let .commitTextReplacingPreedit(text):
            // Replaces the preedit region atomically from the user's
            // perspective (Android achieves this via `commitText(text, 1)`).
            markedText.commit(text, on: textDocumentProxy)
        case .clearCandidates:
            resetAutocomplete()
        case .refreshCandidates:
            performAutocomplete()
        case .resetCandidateContext:
            state.autocompleteContext.reset()
        case let .nextWordUpdateLastSelectedWord(text, roman):
            // Nail / unnail handshake — NextWord learns nothing from it (§40);
            // the final commit's `preceding` carries the nailed segments.
            actionHandler?.nextWordController.updateLastSelectedWord(text: text, roman: roman)
        case let .nextWordWordSelected(text, roman, triggerPrediction, preceding):
            // Final-commit handshake. Forward triggerPrediction + preceding
            // verbatim — the engine already decided what to predict and learn.
            actionHandler?.nextWordController.process(
                text: text,
                roman: roman,
                requireRomanMode: false,
                triggerPrediction: triggerPrediction,
                preceding: preceding,
            )
        case .nextWordClearForNewComposing:
            // ClearForNewComposing ≠ ResetAll — clearDisplay() sends the
            // matching `nextwordClearForNewComposing` intent. Do NOT route
            // to `resetAndClearUI()` (that maps to `nextwordResetAll`).
            actionHandler?.nextWordController.clearDisplay()
        }
    }
}
