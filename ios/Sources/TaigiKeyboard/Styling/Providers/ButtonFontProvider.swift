import KeyboardKit
import Foundation

/// Provider for button font customization
class ButtonFontProvider {

    private let keyboardContext: KeyboardContext
    private let standardService: KeyboardStyle.StandardStyleService

    init(keyboardContext: KeyboardContext) {
        self.keyboardContext = keyboardContext
        self.standardService = KeyboardStyle.StandardStyleService(keyboardContext: keyboardContext)
    }

    /// Get custom button font for the given action
    /// - Parameter action: The keyboard action
    /// - Returns: The keyboard font to use
    func buttonKeyboardFont(for action: KeyboardAction) -> KeyboardFont {
        // 獲取標準字體作為基礎
        let standardFont = standardService.buttonKeyboardFont(for: action)
        let fontSize = getFontSize(from: standardFont, for: action)
        let useCustomFont = SharedSettings.shared.isCustomFontEnabled

        // 如果啟用自訂字體，對所有按鍵使用自訂字體
        if useCustomFont {
            return KeyboardFont.custom(
                KeyboardModels.Fonts.extensionFontName,
                size: fontSize,
                weight: standardFont.weight ?? .regular
            )
        }

        // 自訂字體關閉時，只對 CJK Extension 字元使用自訂字體
        if case let .character(char) = action,
           KeyboardModels.Fonts.containsCJKExtension(char) {
            return KeyboardFont.custom(
                KeyboardModels.Fonts.extensionFontName,
                size: fontSize,
                weight: standardFont.weight ?? .regular
            )
        }

        // 其他情況使用標準字體
        return standardFont
    }

    /// 從 KeyboardFont 中提取字體大小
    private func getFontSize(from keyboardFont: KeyboardFont, for action: KeyboardAction) -> CGFloat {
        switch keyboardFont.type {
        case .system(let size), .custom(_, let size), .customFixed(_, let size):
            return size
        default:
            // 其他類型使用標準字體大小或預設值
            return action.standardButtonFontSize(for: keyboardContext)
        }
    }
}
