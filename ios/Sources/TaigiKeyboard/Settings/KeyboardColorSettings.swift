// Persisted keyboard color settings (Codable + UserDefaults); a nil field falls back to KeyboardKit's dynamic color.

import Foundation
import SwiftUI
import UIKit

// MARK: - Codable Color

/// A color value that persists a single static RGBA to UserDefaults.
///
/// This deliberately stores one color for both light and dark modes.
/// When no custom color is set (`KeyboardColorSettings` field is `nil`),
/// the keyboard falls back to KeyboardKit's dynamic adaptive colors.
struct CodableColor: Codable, Equatable {
    let red: Double
    let green: Double
    let blue: Double
    let alpha: Double

    var color: Color {
        Color(red: red, green: green, blue: blue, opacity: alpha)
    }

    /// Perceived luminance below mid-gray (Rec. 601 weighting). Used to derive a
    /// theme's palette appearance (dark keyText ⇒ light-palette theme) so the emoji
    /// key can pick the matching KeyboardKit asset variant.
    var isDark: Bool {
        (0.299 * red + 0.587 * green + 0.114 * blue) < 0.5
    }

    init(_ color: Color) {
        let uiColor = UIColor(color)
        var r: CGFloat = 0, g: CGFloat = 0, b: CGFloat = 0, a: CGFloat = 0
        uiColor.getRed(&r, green: &g, blue: &b, alpha: &a)
        red = Double(r)
        green = Double(g)
        blue = Double(b)
        alpha = Double(a)
    }

    /// Builds an opaque color from a `0xRRGGBB` literal (any high 8 bits are
    /// ignored — pass `0xRRGGBB`, not `0xAARRGGBB`). sRGB components, matching
    /// the `Color(red:green:blue:opacity:)` reconstruction in `color`. Used by
    /// the built-in theme table; no string parse, no failure path.
    init(hex: UInt32) {
        red = Double((hex >> 16) & 0xFF) / 255.0
        green = Double((hex >> 8) & 0xFF) / 255.0
        blue = Double(hex & 0xFF) / 255.0
        alpha = 1.0
    }
}

// MARK: - Theme gradient

/// A vertical (top→bottom) keyboard-background gradient. `stops` are ordered
/// top→bottom and must have ≥2 entries to render; the render layer ignores a
/// gradient with fewer than 2 stops and falls back to the flat `backgroundColor`.
/// Built-in gradient themes (Standard Blue/Green/Purple) set this; flat themes
/// leave it nil. Synthesized `Codable` — an absent key in old JSON decodes to nil.
struct ThemeGradient: Codable, Equatable {
    let stops: [CodableColor]
}

// MARK: - Keyboard Color Settings

/// The customizable keyboard color roles; a nil field means "use the KeyboardKit default".
struct KeyboardColorSettings: Codable, Equatable {
    var backgroundColor: CodableColor?
    var keyTextColor: CodableColor?
    /// Letter-key fill.
    var normalKeyFillColor: CodableColor?
    /// Fill for Shift / Backspace / Enter and other special keys.
    var specialKeyFillColor: CodableColor?
    var candidateTextColor: CodableColor?
    var candidateBackgroundColor: CodableColor?
    /// Vertical background gradient; overrides `backgroundColor` and turns the candidate bar transparent.
    var backgroundGradient: ThemeGradient?

    static let `default` = KeyboardColorSettings()

    /// Whether a renderable gradient is set (≥2 stops). Single source for the
    /// render branch, the liquid-glass gate, and the candidate-bar transparency.
    var hasBackgroundGradient: Bool {
        (backgroundGradient?.stops.count ?? 0) >= 2
    }

    /// Factors used to derive the candidate strip's first-candidate highlight and
    /// pressed tints from a gradient theme's top stop, so those states match the theme
    /// hue instead of a neutral keycap color. The highlight is LIGHTENED toward white
    /// (a light tint of the hue, lighter than the gradient bar so it stays visible);
    /// the pressed state is DEEPENED toward black (a darker press feedback). A
    /// flat/scaffold theme (no gradient) keeps the neutral KeyboardKit fallback.
    // CROSS-PLATFORM INVARIANT — mirrors android/app/src/main/java/com/siansiansu/taigikeyboard/ime/core/KeyboardColorSettings.kt
    // CANDIDATE_HIGHLIGHT_LIGHTEN_FACTOR / CANDIDATE_PRESSED_DEEPEN_FACTOR. Drift causes silent divergence.
    static let candidateHighlightLightenFactor: Double = 0.5
    static let candidatePressedDeepenFactor: Double = 0.65
}

extension CodableColor {
    /// Returns an opaque variant lightened toward white by `factor`: each 0-255 RGB
    /// component is lifted by `component + (255 - component) * factor`, truncated
    /// toward zero. Used to derive the candidate first-candidate highlight — a light
    /// tint of the gradient theme's top stop.
    func lightened(towardWhite factor: Double) -> CodableColor {
        func scaled(_ component: Double) -> UInt32 {
            let byte = UInt32((component * 255).rounded())
            return byte + UInt32(Double(255 - byte) * factor)
        }
        let hex = (scaled(red) << 16) | (scaled(green) << 8) | scaled(blue)
        return CodableColor(hex: hex)
    }

    /// Returns an opaque variant deepened toward black by `factor`: each 0-255 RGB
    /// component is recovered, multiplied, and truncated toward zero. Used to derive
    /// the candidate pressed tint from a gradient theme's top stop.
    ///
    /// The 0-1 → 0-255 → 0-1 (`init(hex:)`) round-trip is deliberate, not redundant:
    /// it forces per-byte integer truncation so the result is byte-identical to
    /// Android's `deepenedArgb` (`.toInt()`), keeping the CROSS-PLATFORM INVARIANT
    /// exact. A direct `Color(red: red * factor, …)` would keep float precision and
    /// drift from Android by sub-byte amounts. Do not "simplify" away the round-trip.
    func deepened(by factor: Double) -> CodableColor {
        func scaled(_ component: Double) -> UInt32 {
            let byte = UInt32((component * 255).rounded())
            return UInt32(Double(byte) * factor)
        }
        let hex = (scaled(red) << 16) | (scaled(green) << 8) | scaled(blue)
        return CodableColor(hex: hex)
    }
}
