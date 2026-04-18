import Foundation
import SwiftUI

/// ViewModel for `AppearanceSettingsView`.
///
/// Owns every piece of appearance state (colors, sliders, font) plus its
/// defaults and persistence. The view binds directly to published properties
/// and calls the `set…` / `reset…` methods for side effects.
@MainActor
final class AppearanceSettingsViewModel: ObservableObject {
    // MARK: - Defaults

    enum Defaults {
        static let keyboardBackground = Color.keyboardBackground
        static let keyText = Color.keyboardButtonForeground
        static let normalKeyFill = Color.keyboardButtonBackground
        static let specialKeyFill = Color.keyboardDarkButtonBackground
        static let candidateText = Color(.label)
        static let candidateBackground = Color.keyboardBackground

        static let keyHeightScale: Double = 1.0
        static let keyFontSizeScale: Double = 1.0
        static let candidateTextSizeScale: Double = 1.0
        static let keyCornerRadius: Double = 6.0
        static let keyBorderWidth: Double = 0
        static let fontType: FontType = .openHuninn
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
