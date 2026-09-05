import Foundation
import SwiftUI

/// Default appearance constants shared by the custom-theme card button preview
/// (`ThemePickerView`) and the user-theme editor (`ThemeEditorView`): the six
/// adaptive role colors, the five size scalars (single-sourced from
/// `ThemeAppearance.default`), and the global default font.
enum ThemeDefaults {
    static let keyboardBackground = Color.keyboardBackground
    static let keyText = Color.keyboardButtonForeground
    static let normalKeyFill = Color.keyboardButtonBackground
    static let specialKeyFill = Color.keyboardDarkButtonBackground
    static let candidateText = Color(.label)
    static let candidateBackground = Color.keyboardBackground

    static let keyHeightScale = ThemeAppearance.default.keyHeightScale
    static let keyFontSizeScale = ThemeAppearance.default.keyFontSizeScale
    static let candidateTextSizeScale = ThemeAppearance.default.candidateTextSizeScale
    static let keyCornerRadius = ThemeAppearance.default.keyCornerRadius
    static let keyBorderWidth = ThemeAppearance.default.keyBorderWidth

    // Font is a global setting, not per-theme.
    static let fontType = FontType.keyboardDefault
}
