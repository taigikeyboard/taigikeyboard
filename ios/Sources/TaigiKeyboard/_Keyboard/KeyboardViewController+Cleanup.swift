import KeyboardKit
import Foundation

// MARK: - Cleanup Operations

extension KeyboardViewController {
    /// 執行完整清理
    func performCleanup() {
        guard !isCleanedUp else {
            return
        }

        isCleanedUp = true

        // 清理輸入狀態
        cleanupInputState()

        // 清理服務
        cleanupServices()

        if let emojiService = emojiSvc {
            emojiService.delegate = nil
            emojiSvc = nil
        }
    }

    /// 清理服務連結
    func cleanupServices() {
        if let handler = actionHandler {
            handler.keyboardViewController = nil
        }
        actionHandler = nil
        services.spaceDragGestureHandler.action = { _ in }
    }

    /// 清理所有輸入相關狀態，確保鍵盤重新啟動時是乾淨的
    func cleanupInputState() {
        // 1. 清理 ComposingManager 狀態
        if let handler = actionHandler {
            handler.composingManager.reset()
        }

        // 2. 清理 TextDocumentProxy 的 markedText
        textDocumentProxy.setMarkedText("", selectedRange: NSRange(location: 0, length: 0))
        textDocumentProxy.unmarkText()

        // 3. 清理 AutocompleteContext
        state.autocompleteContext.reset()
    }
}
