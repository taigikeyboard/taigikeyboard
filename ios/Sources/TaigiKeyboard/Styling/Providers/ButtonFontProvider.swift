import Foundation
import KeyboardKit
import UIKit

/// Button font provider — resolves the KeyboardFont for each key.
///
/// Combines user-selected font family (via `SettingsSnapshot.fontType`) with
/// KeyboardKit's standard size, user scale, and layout-specific adjustments.
///
/// Created by: `TaigiKeyboardView.RenderProviders`
/// Queried by: `TaigiButtonContent.standardHintContent` (via `buttonUIFont`) and `textOnlyContent`
/// Also called by: `TaigiKeyboardView.coreKeyboard` (keyboardButtonStyle closure)
/// Depends on: `SettingsSnapshot`, `KeyboardContext`
final class ButtonFontProvider {
    /// Shrink factor for 3+ char keys (e.g. "tsh", "chh") to fit within key width
    private static let multiCharShrinkFactor: CGFloat = 0.75

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

        // FontType.customFontName returns nil for .system, PostScript name for custom fonts.
        // Fixed size like `.system(size:)`: `custom(_:size:)` follows Dynamic Type and outgrows the key.
        if let name = settings.fontType.customFontName {
            return KeyboardFont.custom(name, fixedSize: fontSize, weight: .regular)
        }
        return KeyboardFont.system(size: fontSize)
    }

    /// The same font as `buttonKeyboardFont(for:)`, as a UIFont for glyph measurement
    /// (`KeyHintStack`).
    func buttonUIFont(for action: KeyboardAction) -> UIFont {
        let fontSize = adjustedFontSize(for: action, baseFontSize: action.standardButtonFontSize(for: keyboardContext))
        return KeyboardFonts.uiFont(for: settings.fontType, size: fontSize)
    }

    /// Adjusts font size: applies user scale, and shrinks multi-char keys to fit.
    private func adjustedFontSize(for action: KeyboardAction, baseFontSize: CGFloat) -> CGFloat {
        let userScale = settings.keyFontSizeScale

        guard case let .character(char) = action else {
            return baseFontSize * userScale
        }

        // Only shrink 3-char keys (MOE2 tsh/chh, on any layout a custom layout puts them) to fit
        // within key width. No built-in key outside MOE2 has three characters.
        if settings.inputMode != .english, char.count >= 3 {
            return baseFontSize * Self.multiCharShrinkFactor * userScale
        }

        return baseFontSize * userScale
    }
}
