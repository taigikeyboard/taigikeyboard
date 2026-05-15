// 中文: KeyboardViewController 的 ComposingDelegate 實作擴充。
// 中文: 把跨平台中性的 ComposingTransition.Effect 翻譯成 UITextDocumentProxy 動作。

import Foundation
import KeyboardKit
import UIKit

// MARK: - ComposingDelegate

extension KeyboardViewController {
    /// Translate a platform-neutral `RustEngineBridge.ComposingTransition.Effect`
    /// to the iOS `UITextDocumentProxy` surface. Binding contract (iOS +
    /// Android) is documented in `composing-state-boundary.md` §2.2.
    // 中文: 把 ComposingTransition.Effect 派送到 UITextDocumentProxy 對應動作。
    func execute(_ effect: RustEngineBridge.ComposingTransition.Effect) {
        logger.debug({
            let kind = switch effect {
            case let .updatePreedit(text):
                "updatePreedit len=\(text.count)"
            case .clearPreeditWithoutCommit:
                "clearPreeditWithoutCommit"
            case let .commitTextReplacingPreedit(text):
                "commitTextReplacingPreedit len=\(text.count)"
            case .deleteBackwardFromDocument:
                "deleteBackwardFromDocument"
            case .resetAutocomplete:
                "resetAutocomplete"
            case .performAutocomplete:
                "performAutocomplete"
            case .resetAutocompleteContext:
                "resetAutocompleteContext"
            case let .nextWordUpdateLastSelectedWord(text, roman):
                "nextWordUpdateLastSelectedWord text.len=\(text.count) roman.len=\(roman.count)"
            case let .nextWordWordSelected(text, roman, triggerPrediction):
                "nextWordWordSelected text.len=\(text.count) roman.len=\(roman.count) trigger=\(triggerPrediction)"
            case .nextWordClearForNewComposing:
                "nextWordClearForNewComposing"
            }
            return "[COMMIT] fn=execute effect=\(kind)"
        }())
        switch effect {
        case let .updatePreedit(text):
            setMarkedText(text)
        case .clearPreeditWithoutCommit:
            clearMarkedText()
        case let .commitTextReplacingPreedit(text):
            // Clear marked text first so the commit replaces the preedit
            // region atomically from the user's perspective (Android
            // achieves this via `commitText(text, 1)`).
            clearMarkedText()
            textDocumentProxy.insertText(text)
        case .deleteBackwardFromDocument:
            textDocumentProxy.deleteBackward()
        case .resetAutocomplete:
            resetAutocomplete()
        case .performAutocomplete:
            performAutocomplete()
        case .resetAutocompleteContext:
            state.autocompleteContext.reset()
        case let .nextWordUpdateLastSelectedWord(text, roman):
            // v3.5.8 Phase 4 mid-commit handshake. Updates state.last_selected_word
            // without bumping generation; controller injects nowMs / settings.
            actionHandler?.nextWordController.updateLastSelectedWord(text: text, roman: roman)
        case let .nextWordWordSelected(text, roman, triggerPrediction):
            // Final-commit handshake. Forward triggerPrediction verbatim —
            // the engine already decided whether prediction should fire.
            actionHandler?.nextWordController.process(
                text: text,
                roman: roman,
                requireRomanMode: false,
                triggerPrediction: triggerPrediction,
            )
        case .nextWordClearForNewComposing:
            // ClearForNewComposing ≠ ResetFull — clearDisplay() sends the
            // matching `nextwordClearForNewComposing` intent. Do NOT route
            // to `resetAndClearUI()` (that maps to `nextwordResetFull`).
            actionHandler?.nextWordController.clearDisplay()
        }
    }

    /// Set marked (composing) text with the caret placed at the end.
    /// Used by `.updatePreedit(_)` effect and by the explicit preedit
    /// clears in the textDidChange handler.
    // 中文: 設定組字中的 marked text,游標放在尾端。供 updatePreedit / textDidChange 使用。
    func setMarkedText(_ text: String) {
        textDocumentProxy.setMarkedText(text, selectedRange: NSRange(location: text.utf16.count, length: 0))
        // Length-only, no document read — avoids the synchronous proxy query
        // perturbing the mid→final timing race (Codex post-impl observer-effect
        // must-fix). Natural host callbacks below carry the doc-tail signal.
        logger.debug("[BUG3] setMarkedText len=\(text.count)")
    }

    /// Clear marked text + unmark (two steps required by UITextInput).
    /// Used by `.clearPreeditWithoutCommit` and `.commitTextReplacingPreedit`.
    // 中文: 清掉 marked text 並 unmark — UITextInput 需要分兩步,缺一不可。
    func clearMarkedText() {
        // Split markers (no document read) so a host callback that finalizes
        // the region can be attributed to setMarkedText("") vs unmarkText()
        // by interleaving with the natural-callback [BUG3] doc snapshots —
        // without injecting a perturbing synchronous proxy query here
        // (Codex post-impl observer-effect + attribution must-fix).
        logger.debug("[BUG3] clearMarkedText pre-setEmpty")
        textDocumentProxy.setMarkedText("", selectedRange: NSRange(location: 0, length: 0))
        logger.debug("[BUG3] clearMarkedText post-setEmpty pre-unmark")
        textDocumentProxy.unmarkText()
        logger.debug("[BUG3] clearMarkedText post-unmark")
    }
}

extension RustEngineBridge.ComposingTransition.Effect {
    /// v3.5.8 Phase 9 Bug 3 instrumentation (PR1, instrumentation-only —
    /// removed in the Bug 3 fix PR). Compact effect token for the `[BUG3]`
    /// effect-order log; single-sourced so the ordered effect list in
    /// `ComposingManager.commitContinuous` reads a stable vocabulary. Left
    /// unconditional (not `#if DEBUG`) because it is referenced inside a
    /// `DebugLogger.debug` `@autoclosure`, whose body must still type-check
    /// in release even though it is never evaluated there.
    var bug3Kind: String {
        switch self {
        case .updatePreedit: "preedit"
        case .clearPreeditWithoutCommit: "clearPreedit"
        case .commitTextReplacingPreedit: "commit"
        case .deleteBackwardFromDocument: "delBack"
        case .resetAutocomplete: "resetAC"
        case .performAutocomplete: "perfAC"
        case .resetAutocompleteContext: "resetCtx"
        case .nextWordUpdateLastSelectedWord: "nwUpd"
        case .nextWordWordSelected: "nwSel"
        case .nextWordClearForNewComposing: "nwClear"
        }
    }
}
