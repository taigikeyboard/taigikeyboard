// 中文: 外觀預覽專用的 KeyboardEnvironment。讓 KeyboardPreviewPanel 用一份固定/草稿 ThemeAppearance
// 中文: 渲染真實 TaigiKeyboardView,而不去讀寫全域 SharedSettings —— 編輯器是 draft-and-save。

import Foundation
import SwiftUI

/// A display-only `KeyboardEnvironment` for the appearance preview.
///
/// All appearance reads return the supplied `appearance` (a draft user theme, or
/// the default buffer), so the preview renders exactly what is being edited
/// without writing the live settings. Non-appearance reads (layout type, the
/// notification store, translate/TPS flags) are read from `SharedSettings.shared`
/// so the preview matches the real keyboard's mode.
///
/// Writable setters are **local, no-op against persistence**: the preview hosts a
/// real `CandidateView` whose input-mode toggle would otherwise mutate the
/// user's global `inputMode`. Keeping `inputMode` / `isFullAccessEnabled` local
/// makes the preview inert.
// 中文: 唯讀預覽環境。外觀讀草稿,其餘讀全域;可寫 setter 全留在本地(避免預覽內的模式切換污染真實設定)。
final class ThemePreviewEnvironment: KeyboardEnvironment {
    private let appearance: ThemeAppearance
    private let appliesThemeShadow: Bool
    private let base: SharedSettings

    /// Local mode state — initialized from the real setting but never written back.
    var inputMode: InputMode
    /// Local only; the preview never changes full-access state.
    var isFullAccessEnabled: Bool = false

    /// - Parameters:
    ///   - appearance: the theme appearance to render (draft user theme / default buffer).
    ///   - appliesThemeShadow: `true` for user-theme drafts (slider 0 = flat); `false` for
    ///     the default buffer (keeps KeyboardKit's standard shadow, matching the real keyboard).
    init(appearance: ThemeAppearance, appliesThemeShadow: Bool, base: SharedSettings = .shared) {
        self.appearance = appearance
        self.appliesThemeShadow = appliesThemeShadow
        self.base = base
        inputMode = base.inputMode
    }

    var colorSettings: KeyboardColorSettings { appearance.colors }
    func resolvedAppearance(for _: ColorScheme) -> ThemeAppearance { appearance }
    var keyFontSizeScale: CGFloat { CGFloat(appearance.keyFontSizeScale) }
    var keyBorderWidth: CGFloat { CGFloat(appearance.keyBorderWidth) }
    var candidateTextSizeScale: CGFloat { CGFloat(appearance.candidateTextSizeScale) }
    var fontType: FontType { appearance.fontType }
    var resolvedFontType: FontType { appearance.fontType }
    var keyboardLayoutType: KeyboardLayoutType { base.keyboardLayoutType }
    var settingsUserDefaults: UserDefaults { base.settingsUserDefaults }

    func snapshot(for _: ColorScheme) -> SettingsSnapshot {
        SettingsSnapshot(
            inputMode: inputMode,
            fontType: appearance.fontType,
            keyboardLayoutType: base.keyboardLayoutType,
            isTranslateSwapped: base.isTranslateSwapped,
            isTpsOrMappedToER: base.isTpsOrMappedToER,
            keyFontSizeScale: CGFloat(appearance.keyFontSizeScale),
            keyCornerRadius: CGFloat(appearance.keyCornerRadius),
            colorSettings: appearance.colors,
            candidateTextSizeScale: CGFloat(appearance.candidateTextSizeScale),
            keyBorderWidth: CGFloat(appearance.keyBorderWidth),
            keyShadowIntensity: appliesThemeShadow ? CGFloat(appearance.keyShadowIntensity) : nil,
        )
    }
}
