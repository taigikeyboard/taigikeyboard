// 主題外觀預設值常數(6 角色 adaptive 色 / 5 個尺寸 / 全域預設字型)。
// 原 AppearanceSettingsViewModel.Defaults 抽出 — 供自訂主題卡按鈕預覽與 ThemeEditorView reset 共用。

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

    // 尺寸預設的單一來源 = ThemeAppearance.default(factory 外觀)。
    static let keyHeightScale = ThemeAppearance.default.keyHeightScale
    static let keyFontSizeScale = ThemeAppearance.default.keyFontSizeScale
    static let candidateTextSizeScale = ThemeAppearance.default.candidateTextSizeScale
    static let keyCornerRadius = ThemeAppearance.default.keyCornerRadius
    static let keyBorderWidth = ThemeAppearance.default.keyBorderWidth

    // 字型為全域設定(非主題),預設值取 FontType 單一來源。
    static let fontType = FontType.keyboardDefault
}
