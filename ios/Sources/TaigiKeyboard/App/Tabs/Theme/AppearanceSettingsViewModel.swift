// 中文: AppearanceSettingsView 的 ViewModel。集中管理外觀相關的 published state
// 中文: 與其 SharedSettings 持久化動作。

import Foundation
import SwiftUI

/// ViewModel for `AppearanceSettingsView`.
///
/// Owns every piece of appearance state (colors, sliders, font) plus its
/// defaults and persistence. The view binds directly to published properties
/// and calls the `set…` / `reset…` methods for side effects.
// 中文: 外觀設定 ViewModel(@MainActor + ObservableObject)。
// 中文: 持有所有 slider / 顏色 / 字型 published 屬性,並透過 SharedSettings 落盤。
@MainActor
final class AppearanceSettingsViewModel: ObservableObject {
    // MARK: - Defaults

    // 中文: 外觀設定的預設值常數集合(顏色、slider 範圍、字型)。
    enum Defaults {
        static let keyboardBackground = Color.keyboardBackground
        static let keyText = Color.keyboardButtonForeground
        static let normalKeyFill = Color.keyboardButtonBackground
        static let specialKeyFill = Color.keyboardDarkButtonBackground
        static let candidateText = Color(.label)
        static let candidateBackground = Color.keyboardBackground

        // 中文: 尺寸/字型預設的單一來源 = ThemeAppearance.default(factory 外觀);避免三處各寫一份。
        static let keyHeightScale = ThemeAppearance.default.keyHeightScale
        static let keyFontSizeScale = ThemeAppearance.default.keyFontSizeScale
        static let candidateTextSizeScale = ThemeAppearance.default.candidateTextSizeScale
        static let keyCornerRadius = ThemeAppearance.default.keyCornerRadius
        static let keyBorderWidth = ThemeAppearance.default.keyBorderWidth
        static let fontType = ThemeAppearance.default.fontType
    }

    // MARK: - Sliders

    @Published var keyHeightScale: Double
    @Published var keyFontSizeScale: Double
    @Published var candidateTextSizeScale: Double
    @Published var keyCornerRadius: Double
    @Published var keyBorderWidth: Double

    // MARK: - Font

    @Published var selectedFontType: FontType

    // MARK: - Colors

    @Published var keyboardBackground: Color
    @Published var keyText: Color
    @Published var normalKeyFill: Color
    @Published var specialKeyFill: Color
    @Published var candidateText: Color
    @Published var candidateBackground: Color

    /// Tracks which colors have been explicitly customized (non-nil = customized).
    // 中文: 記錄哪幾個顏色 row 被使用者改過(non-nil 即代表已自訂),驅動 reset 按鈕顯示。
    @Published var savedColors: KeyboardColorSettings

    // MARK: - Dependencies

    private let settings = SharedSettings.shared

    // MARK: - Init

    init() {
        keyHeightScale = settings.keyHeightScale
        keyFontSizeScale = settings.keyFontSizeScale
        candidateTextSizeScale = settings.candidateTextSizeScale
        keyCornerRadius = settings.keyCornerRadius
        keyBorderWidth = settings.keyBorderWidth
        selectedFontType = settings.fontType

        let c = settings.colorSettings
        keyboardBackground = c.backgroundColor?.color ?? Defaults.keyboardBackground
        keyText = c.keyTextColor?.color ?? Defaults.keyText
        normalKeyFill = c.normalKeyFillColor?.color ?? Defaults.normalKeyFill
        specialKeyFill = c.specialKeyFillColor?.color ?? Defaults.specialKeyFill
        candidateText = c.candidateTextColor?.color ?? Defaults.candidateText
        candidateBackground = c.candidateBackgroundColor?.color ?? Defaults.candidateBackground
        savedColors = c
    }

    // MARK: - Slider / font persistence

    func setKeyHeightScale(_ value: Double) {
        settings.keyHeightScale = value
    }

    func setKeyFontSizeScale(_ value: Double) {
        settings.keyFontSizeScale = value
    }

    func setCandidateTextSizeScale(_ value: Double) {
        settings.candidateTextSizeScale = value
    }

    func setKeyCornerRadius(_ value: Double) {
        settings.keyCornerRadius = value
    }

    func setKeyBorderWidth(_ value: Double) {
        settings.keyBorderWidth = value
    }

    func setFontType(_ font: FontType) {
        settings.fontType = font
    }

    // MARK: - Color persistence

    /// Persist a color change for `keyPath` and mark it as customized.
    // 中文: 寫入單一顏色 row 的最新值並標記為已自訂(savedColors 同步更新)。
    func applyColorChange(
        _ keyPath: WritableKeyPath<KeyboardColorSettings, CodableColor?>,
        to color: Color,
    ) {
        var cs = settings.colorSettings
        cs[keyPath: keyPath] = CodableColor(color)
        settings.colorSettings = cs
        savedColors = cs
    }

    /// Reset a single color row to default (marks as not customized).
    // 中文: 把單一顏色 row 還原為 nil(預設色),解除自訂標記。
    func resetColor(
        _ keyPath: WritableKeyPath<KeyboardColorSettings, CodableColor?>,
    ) {
        var cs = settings.colorSettings
        cs[keyPath: keyPath] = nil
        settings.colorSettings = cs
        savedColors = cs
    }

    // MARK: - Reset all

    /// Reset every appearance setting (sliders, font, colors) to defaults.
    // 中文: 一次重置所有外觀設定(slider、字型、顏色)到 Defaults 並落盤。
    func resetAllAppearance() {
        selectedFontType = Defaults.fontType
        settings.fontType = Defaults.fontType

        keyHeightScale = Defaults.keyHeightScale
        settings.keyHeightScale = Defaults.keyHeightScale
        keyFontSizeScale = Defaults.keyFontSizeScale
        settings.keyFontSizeScale = Defaults.keyFontSizeScale
        candidateTextSizeScale = Defaults.candidateTextSizeScale
        settings.candidateTextSizeScale = Defaults.candidateTextSizeScale
        keyCornerRadius = Defaults.keyCornerRadius
        settings.keyCornerRadius = Defaults.keyCornerRadius
        keyBorderWidth = Defaults.keyBorderWidth
        settings.keyBorderWidth = Defaults.keyBorderWidth

        keyboardBackground = Defaults.keyboardBackground
        keyText = Defaults.keyText
        normalKeyFill = Defaults.normalKeyFill
        specialKeyFill = Defaults.specialKeyFill
        candidateText = Defaults.candidateText
        candidateBackground = Defaults.candidateBackground
        settings.colorSettings = .default
        savedColors = .default
    }
}
