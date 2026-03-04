import KeyboardKit
import Foundation

/// Button font provider
///
/// Returns the appropriate KeyboardFont based on user's font setting
/// (System / jf open 粉圓 / 芫荽 Iansui).
class ButtonFontProvider {

    private let keyboardContext: KeyboardContext

    init(keyboardContext: KeyboardContext) {
        self.keyboardContext = keyboardContext
    }

    /// Returns the font for the given keyboard action
    func buttonKeyboardFont(for action: KeyboardAction) -> KeyboardFont {
        let baseFontSize = action.standardButtonFontSize(for: keyboardContext)
        let fontSize = adjustedFontSize(for: action, baseFontSize: baseFontSize)
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

    /// Adjusts font size based on layout type and user scale
    private func adjustedFontSize(for action: KeyboardAction, baseFontSize: CGFloat) -> CGFloat {
        let userScale = SharedSettings.shared.keyFontSizeScale

        guard case .character(let char) = action else {
            return baseFontSize * userScale
        }

        let layoutType = SharedSettings.shared.keyboardLayoutType
        if layoutType == .moe2, SharedSettings.shared.inputMode != .english {
            // Only shrink 3-char keys (tsh/chh) to fit within key width
            if char.count >= 3 {
                return baseFontSize * 0.75 * userScale
            }
        }

        return baseFontSize * userScale
    }
}
