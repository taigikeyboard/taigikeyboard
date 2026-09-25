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
    /// ⁿ becomes ᴺ in capitals (§53): the `nn` key label and the suggestion case transform read it.
    let isNasalMarkerUppercaseEnabled: Bool
    let keyFontSizeScale: CGFloat
    let keyCornerRadius: CGFloat
    let colorSettings: KeyboardColorSettings
    let candidateTextSizeScale: CGFloat
    let keyBorderWidth: CGFloat
    // Per-theme key shadow, tri-state: nil = keep KeyboardKit's standard shadow (default / built-in themes),
    // 0 = explicitly no shadow (custom theme slider at 0), >0 = explicit shadow point size.
    let keyShadowIntensity: CGFloat?
}
