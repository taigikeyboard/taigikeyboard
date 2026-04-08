import Foundation
import KeyboardKit

/// Button font provider
///
/// Returns the appropriate KeyboardFont based on user's font setting
/// (System / jf open 粉圓 / 芫荽 Iansui).
class ButtonFontProvider {
    private let keyboardContext: KeyboardContext
    private let settings: SettingsSnapshot

    init(keyboardContext: KeyboardContext, settings: SettingsSnapshot) {
        self.keyboardContext = keyboardContext
        self.settings = settings
    }

    /// Returns the font for the given keyboard action
    func buttonKeyboardFont(for action: KeyboardAction) -> KeyboardFont {
        let baseFontSize = action.standardButtonFontSize(for: keyboardContext)
        let fontSize = adjustedFontSize(for: action, baseFontSize: baseFontSize)

        switch settings.fontType {
        case .system:
            return KeyboardFont.system(size: fontSize)
        case .openHuninn:
            return KeyboardFont.custom(
                KeyboardFonts.openHuninnFontName,
                size: fontSize,
                weight: .regular,
            )
        case .iansui:
            return KeyboardFont.custom(
                KeyboardFonts.iansuiFontName,
                size: fontSize,
                weight: .regular,
            )
        }
    }

    /// Adjusts font size based on layout type and user scale
    private func adjustedFontSize(for action: KeyboardAction, baseFontSize: CGFloat) -> CGFloat {
        let userScale = settings.keyFontSizeScale

        guard case let .character(char) = action else {
            return baseFontSize * userScale
        }

        if settings.keyboardLayoutType == .moe2, settings.inputMode != .english {
            // Only shrink 3-char keys (tsh/chh) to fit within key width
            if char.count >= 3 {
                return baseFontSize * 0.75 * userScale
            }
        }

        return baseFontSize * userScale
    }
}
