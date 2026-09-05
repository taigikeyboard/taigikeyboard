import SwiftUI

/// Settings-dependent candidate UI theme.
///
/// Encapsulates the derived sizes and colors that candidate views render with,
/// resolved once from `candidateTextSizeScale` + `KeyboardColorSettings` at the
/// composition root. Passed down through the SwiftUI environment so
/// `CandidateViewModels` no longer reaches out to `SharedSettings.shared`.
struct CandidateTheme: Equatable {
    // MARK: - Baseline constants

    static let baseHeight: CGFloat = 50
    static let bottomPadding: CGFloat = 6
    static let basePrimaryFontSize: CGFloat = 20
    static let baseSecondaryFontSize: CGFloat = 15

    // MARK: - Resolved values

    let height: CGFloat
    let primaryFontSize: CGFloat
    let secondaryFontSize: CGFloat
    let primaryTextColor: Color
    let secondaryTextColor: Color

    /// Top→bottom stops of the active theme's background gradient, or nil for a
    /// flat/default theme. Consumed by the expanded candidate overlay so it paints
    /// the gradient as an opaque backdrop (continuous with the gradient-painted
    /// keyboard root) instead of inheriting the candidate strip's transparent style.
    let backgroundGradientColors: [Color]?

    /// Candidate-strip first-candidate highlight + pressed tints, derived from the
    /// gradient theme's top stop so those states match the theme hue: highlight is a
    /// light tint (lightened toward white), pressed is darker (deepened toward black).
    /// nil for a flat/default theme — the candidate view then keeps its neutral
    /// KeyboardKit fallback (white keycap / dark pressed).
    let firstCandidateHighlightColor: Color?
    let pressedCandidateColor: Color?

    // MARK: - Factory

    /// Derive a theme from user-adjustable settings.
    static func resolved(
        candidateTextSizeScale: CGFloat,
        colorSettings: KeyboardColorSettings,
        screenSizeClass: ScreenSizeClass = .current,
    ) -> CandidateTheme {
        let primaryBase: CGFloat = switch screenSizeClass {
        case .phoneCompact: basePrimaryFontSize
        case .phoneRegular: 21
        case .phoneLarge: basePrimaryFontSize
        case .pad: 23
        }

        let secondaryBase: CGFloat = switch screenSizeClass {
        case .phoneCompact: baseSecondaryFontSize
        case .phoneRegular: 16
        case .phoneLarge: baseSecondaryFontSize
        case .pad: 17
        }

        let customTextColor = colorSettings.candidateTextColor?.color
        let gradientStops = colorSettings.hasBackgroundGradient ? colorSettings.backgroundGradient?.stops : nil
        let gradientColors = gradientStops?.map(\.color)
        // Gradient themes tint the strip's first-candidate + pressed states with a
        // deepened version of the top stop; flat themes leave these nil (neutral fallback).
        let topStop = gradientStops?.first
        return CandidateTheme(
            height: baseHeight * candidateTextSizeScale + bottomPadding,
            primaryFontSize: primaryBase * candidateTextSizeScale,
            secondaryFontSize: secondaryBase * candidateTextSizeScale,
            primaryTextColor: customTextColor ?? Color(.label),
            secondaryTextColor: customTextColor?.opacity(0.7) ?? Color(.secondaryLabel),
            backgroundGradientColors: gradientColors,
            firstCandidateHighlightColor: topStop.map { $0.lightened(towardWhite: KeyboardColorSettings.candidateHighlightLightenFactor).color },
            pressedCandidateColor: topStop.map { $0.deepened(by: KeyboardColorSettings.candidatePressedDeepenFactor).color },
        )
    }

    /// Baseline theme (scale = 1.0, default system colors). Used as the
    /// SwiftUI environment fallback when no composition root is in scope.
    static let standard: CandidateTheme = .init(
        height: baseHeight + bottomPadding,
        primaryFontSize: basePrimaryFontSize,
        secondaryFontSize: baseSecondaryFontSize,
        primaryTextColor: Color(.label),
        secondaryTextColor: Color(.secondaryLabel),
        backgroundGradientColors: nil,
        firstCandidateHighlightColor: nil,
        pressedCandidateColor: nil,
    )
}
