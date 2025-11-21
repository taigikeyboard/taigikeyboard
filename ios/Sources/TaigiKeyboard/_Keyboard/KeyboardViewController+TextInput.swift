import KeyboardKit
import Foundation

// MARK: - Text Input Operations

extension KeyboardViewController {
    /// 設置 markedText（遵循 KeyboardKit 架構）
    func setMarkedText(_ text: String) {
        textDocumentProxy.setMarkedText(text, selectedRange: NSRange(location: text.utf16.count, length: 0))
    }

    /// 確認當前 markedText
    func commitCurrentMarkedText() {
        // unmarkText() 自動將 markedText 內容確認到文檔
        textDocumentProxy.unmarkText()
        // 確保 markedText 完全清除，避免殘留
        textDocumentProxy.setMarkedText("", selectedRange: NSRange(location: 0, length: 0))
    }

    /// 替換 markedText 內容
    func replaceMarkedText(with text: String) {
        textDocumentProxy.setMarkedText(text, selectedRange: NSRange(location: text.count, length: 0))
    }

    /// 清除 markedText（完全移除）
    func clearMarkedText() {
        textDocumentProxy.setMarkedText("", selectedRange: NSRange(location: 0, length: 0))
        textDocumentProxy.unmarkText()
    }

    /// 手動刪除字符
    func deleteBackwardManually() {
        textDocumentProxy.deleteBackward()
    }

    /// 插入文字到文檔
    func insertTextToDocument(_ text: String) {
        textDocumentProxy.insertText(text)
    }

    /// 使用 KeyboardKit 的 replaceCurrentWord 替換當前詞
    func replaceCurrentWordWithSuggestion(_ text: String) {
        textDocumentProxy.replaceCurrentWord(with: text)
    }
}
