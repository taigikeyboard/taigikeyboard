import KeyboardKit
import Foundation

// MARK: - Text Input Operations

extension KeyboardViewController {
    /// 設置 markedText（遵循 KeyboardKit 架構）
    func setMarkedText(_ text: String) {
        textDocumentProxy.setMarkedText(text, selectedRange: NSRange(location: text.utf16.count, length: 0))
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

}
