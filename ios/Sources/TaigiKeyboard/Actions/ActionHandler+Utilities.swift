import Foundation
import KeyboardKit

// MARK: - Utilities

extension ActionHandler {
    /// 根據鍵盤大小寫狀態轉換字元
    /// - Parameters:
    ///   - char: 原始字元
    ///   - keyboardCase: 鍵盤大小寫狀態
    /// - Returns: 轉換後的字元
    func transformCharacterForCase(
        _ char: String,
        keyboardCase: Keyboard.KeyboardCase
    ) -> String {
        switch keyboardCase {
        case .uppercased, .capsLocked:
            char.uppercased()
        case .lowercased, .auto:
            char.lowercased()
        }
    }

    /// 檢查字符是否為標點符號（除了連字符號）
    /// - Parameter char: 要檢查的字符
    /// - Returns: 如果是標點符號（除了 "-"）返回 true
    func isPunctuationExceptHyphen(_ char: String) -> Bool {
        // 定義常見標點符號（除了連字符號）
        let punctuationMarks = ".,!?;:()[]{}\"'`~@#$%^&*+=<>/\\|、。，！？；：（）「」『』《》【】〈〉…"
        return punctuationMarks.contains(char)
    }
}
