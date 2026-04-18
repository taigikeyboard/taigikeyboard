import CoreGraphics
import Foundation

/// Immutable snapshot of settings needed during a single render cycle.
/// Avoids repeated UserDefaults reads when rendering ~50 keys.
struct SettingsSnapshot {
    let inputMode: InputMode
    let fontType: FontType
    let keyboardLayoutType: KeyboardLayoutType
    let isTranslateSwapped: Bool
    let isTpsOrMappedToER: Bool
    let keyFontSizeScale: CGFloat
    let keyCornerRadius: CGFloat
    let colorSettings: KeyboardColorSettings
}
