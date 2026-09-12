import Foundation
import SwiftUI

/// A display-only `KeyboardEnvironment` for the appearance preview.
///
/// All appearance reads return the supplied `appearance` (a draft user theme, or
/// the default buffer), so the preview renders exactly what is being edited
/// without writing the live settings. Non-appearance reads (layout type, the
/// notification store, translate/TPS flags, and the GLOBAL font — font is not
/// part of a theme) are read from `SharedSettings.shared` so the preview matches
/// the real keyboard's mode + font.
///
/// Writable setters are **local, no-op against persistence**: the preview hosts a
/// real `CandidateView` whose input-mode toggle would otherwise mutate the
/// user's global `inputMode`. Keeping `inputMode` / `isFullAccessEnabled` local
/// makes the preview inert.
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

    var colorSettings: KeyboardColorSettings {
        appearance.colors
    }

    func resolvedAppearance(for _: ColorScheme) -> ThemeAppearance {
        appearance
    }

    var keyFontSizeScale: CGFloat {
        CGFloat(appearance.keyFontSizeScale)
    }

    var keyBorderWidth: CGFloat {
        CGFloat(appearance.keyBorderWidth)
    }

    var candidateTextSizeScale: CGFloat {
        CGFloat(appearance.candidateTextSizeScale)
    }

    var fontType: FontType {
        base.fontType
    }

    var keyboardLayoutType: KeyboardLayoutType {
        base.keyboardLayoutType
    }

    var settingsUserDefaults: UserDefaults {
        base.settingsUserDefaults
    }

    func snapshot(for _: ColorScheme) -> SettingsSnapshot {
        SettingsSnapshot(
            inputMode: inputMode,
            fontType: base.fontType,
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
