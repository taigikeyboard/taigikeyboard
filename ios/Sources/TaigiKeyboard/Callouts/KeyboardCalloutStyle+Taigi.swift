import KeyboardKit

// internal, not public — the `FontType` parameter is internal; public would violate access control.
extension KeyboardCalloutStyle {
    /// Builds the long-press callout style for a keyboard `fontType`.
    ///
    /// `.system` keeps KeyboardKit's standard callout font; every custom font
    /// uses the action/input item sizes long-tuned for Taigi callouts (20pt
    /// action, 32pt input). Single source for the keyboard extension and the
    /// appearance-editor preview, so the two never drift.
    static func taigi(for fontType: FontType) -> KeyboardCalloutStyle {
        guard let fontName = fontType.customFontName else {
            return .standard
        }
        return KeyboardCalloutStyle(
            actionItemFont: KeyboardFont.custom(fontName, size: 20, weight: .regular),
            inputItemFont: KeyboardFont.custom(fontName, size: 32, weight: .light),
        )
    }
}
