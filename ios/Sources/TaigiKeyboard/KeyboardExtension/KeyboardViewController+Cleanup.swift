// 鍵盤擴充的清理擴充 — 鍵盤離開或被重建時呼叫,確保下一次啟動是乾淨狀態。

import Foundation
import KeyboardKit
import UIKit

// MARK: - Cleanup Operations

extension KeyboardViewController {
    // 主清理入口 — 冪等,清掉 input state、服務參考與 emoji delegate。
    func performCleanup() {
        guard !isCleanedUp else {
            return
        }

        isCleanedUp = true

        cleanupInputState()
        cleanupServices()

        if let emojiService = emojiServiceStorage {
            emojiService.delegate = nil
            emojiServiceStorage = nil
        }
    }

    // 釋放服務參考 — 主要是 actionHandler。
    func cleanupServices() {
        actionHandler = nil
    }

    /// Ensure clean state for next keyboard activation
    // 重置 ComposingManager / markedText / autocomplete,讓下一次鍵盤啟動是乾淨狀態。
    func cleanupInputState() {
        if let handler = actionHandler {
            handler.composingManager.reset()
        }
        clearMarkedText()
        state.autocompleteContext.reset()
    }
}
