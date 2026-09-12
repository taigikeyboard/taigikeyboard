// Candidate-view style system — a Style / ItemStyle pair, with iOS 26 Liquid Glass support.

import KeyboardKit
import SwiftUI

// MARK: - Style definitions

extension CandidateView {
    struct Style: Codable, Equatable, Hashable {
        init(
            height: CGFloat = CandidateTheme.standard.height,
            backgroundColor: Color? = nil,
            itemStyle: ItemStyle = .standard,
        ) {
            self.height = height
            self.backgroundColor = backgroundColor
            self.itemStyle = itemStyle
        }

        var height: CGFloat

        /// `nil` uses the adaptive color.
        var backgroundColor: Color?

        var itemStyle: ItemStyle
    }

    struct ItemStyle: Codable, Equatable, Hashable {
        init(
            horizontalPadding: Double = 8,
            verticalPadding: Double = 0,
            backgroundColor: Color? = nil,
            selectedBackgroundColor: Color? = nil,
            cornerRadius: CGFloat? = nil,
        ) {
            self.horizontalPadding = horizontalPadding
            self.verticalPadding = verticalPadding
            self.backgroundColor = backgroundColor
            self.selectedBackgroundColor = selectedBackgroundColor
            self.cornerRadius = cornerRadius
        }

        var horizontalPadding: Double

        var verticalPadding: Double

        /// `nil` uses the adaptive color.
        var backgroundColor: Color?

        /// `nil` uses the adaptive color.
        var selectedBackgroundColor: Color?

        /// `nil` picks the radius from the OS version.
        var cornerRadius: CGFloat?
    }
}

// MARK: - Standard styles

extension CandidateView.Style {
    static var standard: Self {
        .init()
    }

    static var liquidGlass: Self {
        .init(itemStyle: .liquidGlass)
    }
}

extension CandidateView.ItemStyle {
    static var standard: Self {
        .init()
    }

    static var liquidGlass: Self {
        .init(
            cornerRadius: 9, // matches KeyboardKit's Liquid Glass corner radius
        )
    }
}

// MARK: - Style helpers

extension CandidateView.Style {
    /// Picks the style matching the context's Liquid Glass state.
    static func adaptive(for context: KeyboardContext) -> Self {
        if context.isLiquidGlassEnabled {
            .liquidGlass
        } else {
            .standard
        }
    }

    /// Single source of truth for "is Liquid Glass on", shared by `CandidateView`,
    /// `ExpandedCandidateOverlay` and `CandidateButtonView` so the rule lives in one place.
    var isLiquidGlassEnabled: Bool {
        itemStyle.cornerRadius == 9 && backgroundColor == nil
    }

    /// Resolves the candidate bar background (transparent under iOS 26 Liquid Glass).
    func resolvedBarBackground(for colorScheme: ColorScheme) -> Color {
        if isLiquidGlassEnabled {
            // Near-zero opacity keeps touch handling while letting the system Liquid Glass show through.
            return Color.white.opacity(0.001)
        }
        return backgroundColor ?? Color.keyboardBackground(for: colorScheme)
    }
}

extension CandidateView.ItemStyle {
    /// Resolves the item background for the given state.
    ///
    /// `isFirstCandidate` is the engine ranker's top hit (index 0) and fills the key-cap background
    /// as a visual hint. `firstCandidateThemeColor` / `pressedThemeColor` are derived from a gradient
    /// theme; `nil` means a non-gradient theme, which takes the neutral fallback below.
    func resolvedBackgroundColor(
        for colorScheme: ColorScheme,
        isSelected: Bool = false,
        isPressed: Bool = false,
        isFirstCandidate: Bool = false,
        isLiquidGlassEnabled: Bool = false,
        firstCandidateThemeColor: Color? = nil,
        pressedThemeColor: Color? = nil,
    ) -> Color {
        // State priority, identical for every theme: a real press wins with the dark pressed look;
        // then the first candidate takes the light highlight even while selected, because while
        // typing the engine pins `selectedCandidateIndex` to 0 so it is always "selected" yet must
        // still read as a hint; other selected candidates (hardware navigation) take pressed; else idle.
        // A gradient theme overrides the neutral colors with its derived ones.
        if isPressed, let pressedThemeColor {
            return pressedThemeColor
        }
        if isFirstCandidate, let firstCandidateThemeColor {
            return firstCandidateThemeColor
        }
        if isSelected, let pressedThemeColor {
            return pressedThemeColor
        }

        // Non-gradient fallback, same priority: pressed > firstCandidate > selected > idle.
        if isLiquidGlassEnabled {
            // Transparent background lets the system effect through; pressed/selected use the idle
            // color at +0.6 opacity, the first candidate a lower 0.4 to keep the state hierarchy.
            let pressedLook = (backgroundColor ?? Color.keyboardButtonBackgroundLiquid(for: colorScheme)).opacity(0.6)
            if isPressed {
                return pressedLook
            }
            if isFirstCandidate {
                return Color.keyboardButtonBackgroundLiquid(for: colorScheme).opacity(0.4)
            }
            if isSelected {
                return pressedLook
            }
            return Color.white.opacity(0.001)
        } else {
            // Follows KeyboardKit's `backgroundColorPressed`; the first candidate fills the key-cap
            // background as a light hint, matching Android `key_bgColor` and the Rime-family
            // highlighted-candidate convention.
            let pressedLook = selectedBackgroundColor ?? Color.keyboardDarkButtonBackground(for: colorScheme)
            if isPressed {
                return pressedLook
            }
            if isFirstCandidate {
                return Color.keyboardButtonBackground
            }
            if isSelected {
                return pressedLook
            }
            return backgroundColor ?? Color.clear
        }
    }
}
