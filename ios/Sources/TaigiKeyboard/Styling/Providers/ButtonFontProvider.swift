// 中文: 鍵面字型解析 — 結合使用者選的字型家族、KeyboardKit 標準大小、使用者縮放,
// 中文: 以及 layout-specific 調整(MOE2 多字鍵自動縮小)。

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
// 中文: 鍵面字型 provider — 由 TaigiButtonContent 與 KeyboardKit 的 buttonStyle 取得 KeyboardFont。
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
    // 中文: 結合使用者字型選擇與調整後字級,回傳該鍵的 KeyboardFont。
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
    // 中文: 套用使用者字級縮放;MOE2 模式下若鍵面字 ≥ 3 個字元(如 tsh / chh)會再縮 0.75 倍以塞進按鍵。
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
