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
        guard let actionFont = KeyboardFonts.keyboardFont(for: fontType, size: 20, weight: .regular),
              let inputFont = KeyboardFonts.keyboardFont(for: fontType, size: 32, weight: .light)
        else {
            return .standard
        }
        return KeyboardCalloutStyle(actionItemFont: actionFont, inputItemFont: inputFont)
    }

    /// Paints the callout from the theme's `calloutFill` (the key fill, or the background a
    /// see-through key shows) + key text, so a light palette's callout stays light in system
    /// dark mode (KeyboardKit's default `.keyboardButtonBackground` / `.primary` follow the
    /// system appearance). See-through keys over the adaptive background take the adaptive
    /// keyboard background; other adaptive themes keep KeyboardKit's colors. The selected
    /// action item takes the pressed fill (a hovered variant is a pressed key) with the key text.
    // CROSS-PLATFORM INVARIANT — mirrors android .../ime/popup/KeyPopupManager.kt resolveDisplayParams.
    func themed(by colors: KeyboardColorSettings) -> KeyboardCalloutStyle {
        guard let fill = colors.calloutFill, let text = colors.keyTextColor else {
            guard colors.hasTransparentKeys else { return self }
            var style = self
            style.backgroundColor = .keyboardBackground
            return style
        }
        var style = self
        style.backgroundColor = fill.color
        style.foregroundColor = text.color
        if let selected = fill.pressedKeyFill {
            style.selectedBackgroundColor = selected.color
            style.selectedForegroundColor = text.color
        }
        return style
    }
}
