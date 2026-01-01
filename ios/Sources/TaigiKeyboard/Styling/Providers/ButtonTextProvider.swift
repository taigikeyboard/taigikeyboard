import KeyboardKit
import Foundation

/// 按鈕文字提供者
///
/// 回傳按鍵應顯示的文字標籤。回傳 nil 時使用 KeyboardKit 預設內容。
class ButtonTextProvider {

    private let keyboardContext: KeyboardContext

    init(keyboardContext: KeyboardContext) {
        self.keyboardContext = keyboardContext
    }

    /// 取得按鍵對應的文字標籤
    func buttonText(for action: KeyboardAction) -> String? {
        switch action {
        case let .character(char):
            let currentCase = keyboardContext.keyboardCase
            if currentCase.isUppercasedOrCapslocked {
                return char.uppercased()
            } else {
                return char.lowercased()
            }
        case .keyboardType(.numeric):
            return "123"
        case .keyboardType(.alphabetic):
            return "ABC"
        case .keyboardType(.symbolic):
            return "#+="
        case .space:
            return nil
        case .primary(.return) where keyboardContext.isComposingText:
            // 組字模式顯示確認文字
            return ConfirmKeyTextHelper.getConfirmKeyText()
        case .settings:
            return nil
        case let .custom(name):
            switch name {
            case "translate":
                return nil
            default:
                return name
            }
        default:
            return nil
        }
    }
}
