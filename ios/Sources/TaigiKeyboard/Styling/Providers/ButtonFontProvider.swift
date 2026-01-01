import KeyboardKit
import Foundation

/// 按鈕字型提供者
///
/// 根據使用者設定的字型類型（系統/粉圓/芫荽）回傳對應的 KeyboardFont。
class ButtonFontProvider {

    private let keyboardContext: KeyboardContext

    init(keyboardContext: KeyboardContext) {
        self.keyboardContext = keyboardContext
    }

    /// 取得按鍵對應的字型
    func buttonKeyboardFont(for action: KeyboardAction) -> KeyboardFont {
        let fontSize = action.standardButtonFontSize(for: keyboardContext)
        let fontType = SharedSettings.shared.fontType

        switch fontType {
        case .system:
            return KeyboardFont.system(size: fontSize)
        case .openHuninn:
            return KeyboardFont.custom(
                KeyboardModels.Fonts.openHuninnFontName,
                size: fontSize,
                weight: .regular
            )
        case .iansui:
            return KeyboardFont.custom(
                KeyboardModels.Fonts.iansuiFontName,
                size: fontSize,
                weight: .regular
            )
        }
    }
}
