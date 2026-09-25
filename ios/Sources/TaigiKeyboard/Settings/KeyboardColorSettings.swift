// Persisted keyboard color settings (Codable + UserDefaults); a nil role falls back to KeyboardKit's dynamic color.

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

/// A linear keyboard-background gradient: ≥2 color `stops` from start to end plus
/// the direction `angle` in degrees, CSS / Figma convention (`0` = bottom→top,
/// `90` = left→right, `180` = top→bottom, clockwise). Built-in gradient themes use
/// the vertical `defaultAngle`.
///
/// "≥2 stops" is enforced at construction (`init` precondition, decode error), so
/// every `ThemeGradient` a render site sees is renderable. Decode is
/// forward-compatible: an `angle` absent from old JSON reads as `defaultAngle`.
// CROSS-PLATFORM INVARIANT — mirrors android .../ime/core/KeyboardColorSettings.kt ThemeGradient
// (stops + angle, same degree convention and unit-point math).
struct ThemeGradient: Codable, Equatable {
    /// Vertical top→bottom, the direction every built-in gradient theme uses.
    static let defaultAngle: Double = 180
    static let minimumStops = 2
    /// How far the end stop is lifted toward white in `seeded(from:)`.
    private static let seedLightenFactor: Double = 0.45

    var stops: [CodableColor]
    var angle: Double

    init(stops: [CodableColor], angle: Double = ThemeGradient.defaultAngle) {
        precondition(stops.count >= Self.minimumStops, "a gradient needs at least \(Self.minimumStops) stops")
        self.stops = stops
        self.angle = angle
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let stops = try container.decode([CodableColor].self, forKey: .stops)
        guard stops.count >= Self.minimumStops else {
            throw DecodingError.dataCorruptedError(forKey: .stops, in: container, debugDescription: "fewer than \(Self.minimumStops) stops")
        }
        self.stops = stops
        angle = try container.decodeIfPresent(Double.self, forKey: .angle) ?? Self.defaultAngle
    }

    /// The first vertical gradient a user sees when switching a solid background to
    /// Gradient: the solid color running into a lighter tint of itself.
    static func seeded(from solid: CodableColor) -> ThemeGradient {
        ThemeGradient(stops: [solid, solid.lightened(towardWhite: seedLightenFactor)])
    }

    var colors: [Color] {
        stops.map(\.color)
    }

    /// Spacing of the eight preset directions (↑ → ↓ ← and the diagonals) the editors
    /// offer: Android as arrow chips, iOS as snap targets of the preview drag.
    static let presetStep: Double = 45

    /// The unit direction vector of `angle` in screen coordinates (y down): `0` → (0, −1),
    /// `90` → (1, 0). Shared by `unitPoints` and the editor's direction overlay.
    var direction: CGVector {
        Self.direction(degrees: angle)
    }

    static func direction(degrees: Double) -> CGVector {
        let radians = degrees * .pi / 180
        return CGVector(dx: sin(radians), dy: -cos(radians))
    }

    /// Inverse of `direction(degrees:)`: the angle of a screen-space vector, in `-180 ... 180`
    /// (callers wrap it into `0 ..< 360` as they see fit).
    static func degrees(of vector: CGVector) -> Double {
        atan2(vector.dx, -vector.dy) * 180 / .pi
    }

    /// SwiftUI `LinearGradient` start / end points for `angle`, in the unit square of the
    /// painted surface. The CSS direction vector `(sin θ, -cos θ)` (y down) is normalised by
    /// its larger component so the diagonal presets run corner to corner (135° = top-left →
    /// bottom-right) and the axis presets run edge to edge (180° = top-centre → bottom-centre).
    var unitPoints: (start: UnitPoint, end: UnitPoint) {
        let (dx, dy) = (direction.dx, direction.dy)
        let magnitude = max(abs(dx), abs(dy))
        let halfX = dx / magnitude / 2
        let halfY = dy / magnitude / 2
        return (
            start: UnitPoint(x: 0.5 - halfX, y: 0.5 - halfY),
            end: UnitPoint(x: 0.5 + halfX, y: 0.5 + halfY),
        )
    }

    /// The unit points remapped into a panel that covers `[topInset, fullHeight]` of the
    /// keyboard: x is unchanged (same width), `y' = (y · fullHeight − topInset) / panelHeight`,
    /// so the panel paints exactly its slice of the whole-keyboard gradient and stays
    /// continuous with the keyboard above it. A degenerate panel height falls back to the
    /// unshifted points.
    func unitPoints(in slice: KeyboardSurfaceSlice) -> (start: UnitPoint, end: UnitPoint) {
        let points = unitPoints
        let panelHeight = slice.fullHeight - slice.topInset
        guard panelHeight > 0 else { return points }
        func mapped(_ point: UnitPoint) -> UnitPoint {
            UnitPoint(x: point.x, y: (point.y * slice.fullHeight - slice.topInset) / panelHeight)
        }
        return (start: mapped(points.start), end: mapped(points.end))
    }
}

/// Where a painted surface sits inside the whole keyboard: the keyboard's full height and
/// the height of the chrome above the surface (the candidate bar for the overlay panels).
struct KeyboardSurfaceSlice: Equatable {
    let fullHeight: CGFloat
    let topInset: CGFloat

    /// The whole keyboard's frame in the coordinates of a surface `width` wide that shows
    /// this slice: same width, full height, shifted up by the chrome above it.
    func keyboardRect(width: CGFloat) -> CGRect {
        CGRect(x: 0, y: -topInset, width: width, height: fullHeight)
    }
}

// MARK: - Theme image background

/// A photo as the keyboard surface: `file` is the JPEG's name inside the App Group
/// `ThemeImageStore` directory (written by the host app, read by the extension), `dim`
/// the opacity of the tone overlay laid over the desaturated photo so keys stay readable
/// (USER 2026-09-19: "the photo's saturation must not be too loud"). The overlay is white when the key text is
/// dark and black otherwise. `focusX` / `focusY` say which part of the aspect-filled photo
/// stays in view on each axis: 0 = its left / top edge, 1 = its right / bottom edge, 0.5 =
/// centred (the default). An alignment, not a focal point, so the crop never exposes a gap
/// and the same values fit every keyboard aspect (portrait, landscape, iPad).
// CROSS-PLATFORM INVARIANT — mirrors android .../ime/core/KeyboardColorSettings.kt ThemeImageBackground
// (same JSON fields, `saturation`, `dimRange`, `defaultDim`, `defaultFocus`). Drift causes silent divergence.
struct ThemeImageBackground: Codable, Equatable {
    /// Saturation multiplier applied to every photo (1 = untouched).
    static let saturation: Double = 0.7
    static let dimRange: ClosedRange<Double> = 0 ... 0.8
    static let dimStep: Double = 0.05
    static let defaultDim: Double = 0.35
    static let defaultFocus: Double = 0.5
    static let centredFocus = CGPoint(x: defaultFocus, y: defaultFocus)

    let file: String
    var dim: Double
    var focusX: Double
    var focusY: Double

    init(
        file: String,
        dim: Double = ThemeImageBackground.defaultDim,
        focusX: Double = ThemeImageBackground.defaultFocus,
        focusY: Double = ThemeImageBackground.defaultFocus,
    ) {
        self.file = file
        self.dim = min(max(dim, Self.dimRange.lowerBound), Self.dimRange.upperBound)
        self.focusX = min(max(focusX, 0), 1)
        self.focusY = min(max(focusY, 0), 1)
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let file = try container.decode(String.self, forKey: .file)
        guard !file.isEmpty else {
            throw DecodingError.dataCorruptedError(forKey: .file, in: container, debugDescription: "empty image file name")
        }
        try self.init(
            file: file,
            dim: container.decodeIfPresent(Double.self, forKey: .dim) ?? Self.defaultDim,
            focusX: container.decodeIfPresent(Double.self, forKey: .focusX) ?? Self.defaultFocus,
            focusY: container.decodeIfPresent(Double.self, forKey: .focusY) ?? Self.defaultFocus,
        )
    }

    /// The rectangle that scales `imageSize` to cover `bounds` (aspect fill), aligned on each
    /// axis by `focus` (see `focusX` / `focusY`) — the photo's drawn frame over the whole
    /// keyboard, from which a panel shows its slice.
    static func coverRect(imageSize: CGSize, in bounds: CGRect, focus: CGPoint) -> CGRect {
        guard imageSize.width > 0, imageSize.height > 0 else { return bounds }
        let scale = max(bounds.width / imageSize.width, bounds.height / imageSize.height)
        let size = CGSize(width: imageSize.width * scale, height: imageSize.height * scale)
        return CGRect(
            x: bounds.minX + (bounds.width - size.width) * focus.x,
            y: bounds.minY + (bounds.height - size.height) * focus.y,
            width: size.width,
            height: size.height,
        )
    }

    /// `focusX` / `focusY` as a point, the form `coverRect` takes.
    var focus: CGPoint {
        CGPoint(x: focusX, y: focusY)
    }

    /// This photo with Fade `dim` (clamped).
    func with(dim: Double) -> ThemeImageBackground {
        ThemeImageBackground(file: file, dim: dim, focusX: focusX, focusY: focusY)
    }

    /// This photo moved to `focus` (clamped into 0…1).
    func with(focus: CGPoint) -> ThemeImageBackground {
        ThemeImageBackground(file: file, dim: dim, focusX: focus.x, focusY: focus.y)
    }
}

// MARK: - Theme background

/// What paints the keyboard surface — one field, mutually exclusive cases. The
/// candidate bar is the same surface: a solid background colours both, a gradient
/// or photo paints once behind both (the bar goes transparent). `nil` on
/// `KeyboardColorSettings.background` means "adaptive" (KeyboardKit's dynamic
/// background + Liquid Glass) and is reserved for the Filled Default head. Rendering
/// lives in `ThemeBackgroundSurface`.
///
/// JSON: `{"type":"solid","color":{…}}` / `{"type":"gradient","stops":[…],"angle":180}` /
/// `{"type":"image","file":"<uuid>.jpg","dim":0.35,"focusX":0.5,"focusY":0.5}`.
// CROSS-PLATFORM INVARIANT — mirrors android .../ime/core/KeyboardColorSettings.kt ThemeBackground
// (same `type` discriminator and field names; Android stores the colour as an ARGB int).
enum ThemeBackground: Codable, Equatable {
    case solid(CodableColor)
    case gradient(ThemeGradient)
    case image(ThemeImageBackground)

    /// The JSON discriminator, also the editor's Solid / Gradient / Photo segmented choice.
    enum Kind: String, Codable, CaseIterable {
        case solid, gradient, image
    }

    var kind: Kind {
        switch self {
        case .solid: .solid
        case .gradient: .gradient
        case .image: .image
        }
    }

    var solidColor: CodableColor? {
        if case let .solid(color) = self {
            return color
        }
        return nil
    }

    var gradient: ThemeGradient? {
        if case let .gradient(gradient) = self {
            return gradient
        }
        return nil
    }

    var image: ThemeImageBackground? {
        if case let .image(image) = self {
            return image
        }
        return nil
    }

    private enum CodingKeys: String, CodingKey {
        case type, color
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        switch try container.decode(Kind.self, forKey: .type) {
        case .solid:
            self = try .solid(container.decode(CodableColor.self, forKey: .color))
        case .gradient:
            // The gradient's keys sit beside `type` in the same object.
            self = try .gradient(ThemeGradient(from: decoder))
        case .image:
            self = try .image(ThemeImageBackground(from: decoder))
        }
    }

    func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encode(kind, forKey: .type)
        switch self {
        case let .solid(color):
            try container.encode(color, forKey: .color)
        case let .gradient(gradient):
            try gradient.encode(to: encoder)
        case let .image(image):
            try image.encode(to: encoder)
        }
    }
}

// MARK: - Keyboard Color Settings

/// The customizable keyboard color roles; a nil field means "use the KeyboardKit default".
struct KeyboardColorSettings: Equatable {
    /// The keyboard + candidate-bar surface. `nil` = adaptive (KeyboardKit dynamic
    /// background, Liquid Glass eligible).
    var background: ThemeBackground?
    var keyTextColor: CodableColor?
    /// Letter-key fill.
    var normalKeyFillColor: CodableColor?
    /// Fill for Shift / Backspace / Enter and other special keys.
    var specialKeyFillColor: CodableColor?
    var candidateTextColor: CodableColor?

    static let `default` = KeyboardColorSettings()

    /// The solid surface color, or nil for a gradient / adaptive background.
    var solidBackgroundColor: CodableColor? {
        background?.solidColor
    }

    /// The user-theme key fill: one colour for letter and special keys alike
    /// (USER 2026-09-26). Reads the letter fill; writes both fills.
    var keyFillColor: CodableColor? {
        get { normalKeyFillColor }
        set {
            normalKeyFillColor = newValue
            specialKeyFillColor = newValue
        }
    }

    /// The background gradient, or nil for a solid / photo / adaptive background. Single
    /// source for the candidate-tint derivation and the built-in theme tests.
    var backgroundGradient: ThemeGradient? {
        background?.gradient
    }

    /// The custom surface to paint (`ThemeBackgroundSurface`), or nil for the adaptive
    /// default. The photo tone overlay is white when the key text is dark and black
    /// otherwise (the seed's black text is the fallback, so an unset role reads as "light").
    var surface: ThemeSurface? {
        background.map { ThemeSurface(background: $0, dimsTowardWhite: (keyTextColor ?? UserThemeSeed.keyText).isDark) }
    }

    /// Factors used to derive the candidate strip's first-candidate highlight and
    /// pressed tints from a gradient theme's first stop, so those states match the theme
    /// hue instead of a neutral keycap color. The highlight is LIGHTENED toward white
    /// (a light tint of the hue, lighter than the gradient bar so it stays visible);
    /// the pressed state is DEEPENED toward black (a darker press feedback). A
    /// flat/scaffold theme (no gradient) keeps the neutral KeyboardKit fallback.
    // CROSS-PLATFORM INVARIANT — mirrors android/app/src/main/java/com/siansiansu/taigikeyboard/ime/core/KeyboardColorSettings.kt
    // CANDIDATE_HIGHLIGHT_LIGHTEN_FACTOR / CANDIDATE_PRESSED_DEEPEN_FACTOR. Drift causes silent divergence.
    static let candidateHighlightLightenFactor: Double = 0.5
    static let candidatePressedDeepenFactor: Double = 0.65
}

/// A custom keyboard surface together with the tone its photo overlay takes — resolved once
/// from `KeyboardColorSettings` so no render site can pair a background with the wrong tone.
struct ThemeSurface: Equatable {
    let background: ThemeBackground
    /// Whether a photo's dim overlay is white (dark key text) rather than black.
    let dimsTowardWhite: Bool
}

// Codable lives in an extension so the struct keeps its synthesized memberwise init.
extension KeyboardColorSettings: Codable {
    /// `background` replaced three older keys. Decoding still reads them so a theme
    /// written by an older build keeps its look: `backgroundGradient` → `.gradient` at
    /// the vertical `defaultAngle`, else `backgroundColor` → `.solid`.
    /// `candidateBackgroundColor` is dropped — the candidate bar is the keyboard
    /// surface now (USER 2026-09-19). Encoding writes only the current keys.
    private enum CodingKeys: String, CodingKey {
        case background, keyTextColor, normalKeyFillColor, specialKeyFillColor, candidateTextColor
        case legacyBackgroundColor = "backgroundColor"
        case legacyBackgroundGradient = "backgroundGradient"
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        keyTextColor = try container.decodeIfPresent(CodableColor.self, forKey: .keyTextColor)
        normalKeyFillColor = try container.decodeIfPresent(CodableColor.self, forKey: .normalKeyFillColor)
        specialKeyFillColor = try container.decodeIfPresent(CodableColor.self, forKey: .specialKeyFillColor)
        candidateTextColor = try container.decodeIfPresent(CodableColor.self, forKey: .candidateTextColor)
        // `try?`: a background `type` this build does not know (written by a newer build),
        // or a legacy gradient with too few stops, degrades to the next fallback instead
        // of failing the whole theme list.
        if let background = try? container.decodeIfPresent(ThemeBackground.self, forKey: .background) {
            self.background = background
        } else if let gradient = try? container.decodeIfPresent(ThemeGradient.self, forKey: .legacyBackgroundGradient) {
            background = .gradient(gradient)
        } else if let color = try container.decodeIfPresent(CodableColor.self, forKey: .legacyBackgroundColor) {
            background = .solid(color)
        } else {
            background = nil
        }
    }

    func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encodeIfPresent(background, forKey: .background)
        try container.encodeIfPresent(keyTextColor, forKey: .keyTextColor)
        try container.encodeIfPresent(normalKeyFillColor, forKey: .normalKeyFillColor)
        try container.encodeIfPresent(specialKeyFillColor, forKey: .specialKeyFillColor)
        try container.encodeIfPresent(candidateTextColor, forKey: .candidateTextColor)
    }
}

// MARK: - User-theme seed

/// The concrete light palette every user theme starts from, so a user theme never
/// carries a `nil` (scheme-following) role and renders identically in light and
/// dark mode (USER 2026-09-19). Background is the light keyboard grey; the key
/// fill is white (USER 2026-09-25) and shared by letter and special keys.
// CROSS-PLATFORM INVARIANT — mirrors android .../ime/core/KeyboardColorSettings.kt UserThemeSeed
// Drift = a new custom theme starts from different colors per platform.
enum UserThemeSeed {
    static let solidColor = CodableColor(hex: 0xD4D5DD)
    static let background = ThemeBackground.solid(solidColor)
    static let keyText = CodableColor(hex: 0x000000)
    static let keyFill = CodableColor(hex: 0xFFFFFF)
    static let candidateText = CodableColor(hex: 0x000000)

    static let colors = KeyboardColorSettings(
        background: background,
        keyTextColor: keyText,
        normalKeyFillColor: keyFill,
        specialKeyFillColor: keyFill,
        candidateTextColor: candidateText,
    )

    /// The seed value of one role (every role is set in `colors`).
    static func color(_ role: KeyPath<KeyboardColorSettings, CodableColor?>) -> CodableColor {
        colors[keyPath: role]!
    }
}

extension KeyboardColorSettings {
    /// Fills every `nil` role from `UserThemeSeed` and folds the special key fill into
    /// the letter fill (`keyFillColor`). Applied when user themes are loaded, so themes
    /// saved before the seed or the single key fill existed match the editor without a
    /// migration write.
    func seededForUserTheme() -> KeyboardColorSettings {
        var seeded = KeyboardColorSettings(
            background: background ?? UserThemeSeed.background,
            keyTextColor: keyTextColor ?? UserThemeSeed.keyText,
            candidateTextColor: candidateTextColor ?? UserThemeSeed.candidateText,
        )
        seeded.keyFillColor = normalKeyFillColor ?? UserThemeSeed.keyFill
        return seeded
    }
}

extension CodableColor {
    /// Returns an opaque variant lightened toward white by `factor`: each 0-255 RGB
    /// component is lifted by `component + (255 - component) * factor`, truncated
    /// toward zero. Used to derive the candidate first-candidate highlight — a light
    /// tint of the gradient theme's first stop.
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
    /// the candidate pressed tint from a gradient theme's first stop.
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
