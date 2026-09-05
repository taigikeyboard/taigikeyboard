import Foundation
import KeyboardKit

/// Button font provider — resolves the KeyboardFont for each key.
///
/// Combines user-selected font family (via `SettingsSnapshot.fontType`) with
/// KeyboardKit's standard size, user scale, and layout-specific adjustments.
///
/// Created by: `TaigiKeyboardView.RenderProviders`
/// Queried by: `TaigiButtonContent.standardHintContent` and `textOnlyContent`
/// Also called by: `TaigiKeyboardView.coreKeyboard` (keyboardButtonStyle closure)
/// Depends on: `SettingsSnapshot`, `KeyboardContext`
final class ButtonFontProvider {
    /// MOE2 layout: shrink factor for 3+ char keys (e.g. "tsh", "chh") to fit within key width
    private static let moe2MultiCharShrinkFactor: CGFloat = 0.75

    private let keyboardContext: KeyboardContext
    private let settings: SettingsSnapshot

    init(keyboardContext: KeyboardContext, settings: SettingsSnapshot) {
        self.keyboardContext = keyboardContext
        self.settings = settings
    }

    /// Returns the KeyboardFont for the given action, combining user font choice with adjusted size.
    func buttonKeyboardFont(for action: KeyboardAction) -> KeyboardFont {
        let baseFontSize = action.standardButtonFontSize(for: keyboardContext)
        let fontSize = adjustedFontSize(for: action, baseFontSize: baseFontSize)

        // FontType.customFontName returns nil for .system, PostScript name for custom fonts
        if let name = settings.fontType.customFontName {
            return KeyboardFont.custom(name, size: fontSize, weight: .regular)
        }
        return KeyboardFont.system(size: fontSize)
    }

    /// Adjusts font size: applies user scale, and shrinks MOE2 multi-char keys to fit.
    private func adjustedFontSize(for action: KeyboardAction, baseFontSize: CGFloat) -> CGFloat {
        let userScale = settings.keyFontSizeScale

        guard case let .character(char) = action else {
            return baseFontSize * userScale
        }

        if settings.keyboardLayoutType == .moe2, settings.inputMode != .english {
            // Only shrink 3-char keys (tsh/chh) to fit within key width
            if char.count >= 3 {
                return baseFontSize * Self.moe2MultiCharShrinkFactor * userScale
            }
        }

        return baseFontSize * userScale
    }
}
