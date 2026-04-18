import Foundation
import SwiftUI
import UIKit

// MARK: - Input Mode

/// Input mode
enum InputMode: String, CaseIterable {
    case poj // Pe̍h-ōe-jī
    case tl // Tâi-lô
    case english
    case tps // Taiwanese Phonetic Symbols
}

// MARK: - Font Type

/// Font type
enum FontType: String, CaseIterable {
    case system
    case openHuninn // jf open 粉圓
    case iansui // 芫荽
    case genYoMin // 源樣明體
    case genYoGothic // 源樣黑體

    /// PostScript font name for custom fonts, nil for system
    var customFontName: String? {
        switch self {
        case .system: nil
        case .openHuninn: KeyboardFonts.openHuninnFontName
        case .iansui: KeyboardFonts.iansuiFontName
        case .genYoMin: KeyboardFonts.genYoMinFontName
        case .genYoGothic: KeyboardFonts.genYoGothicFontName
        }
    }
}

// MARK: - Keyboard Layout Type

/// Keyboard layout type
enum KeyboardLayoutType: String, CaseIterable {
    case phahTaigi
    case qwerty
    case tps // Taiwanese Phonetic Symbols
    case moe1 // MOE input method layout 1
    case moe2 // MOE input method layout 2
}

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

    init(_ color: Color) {
        let uiColor = UIColor(color)
        var r: CGFloat = 0, g: CGFloat = 0, b: CGFloat = 0, a: CGFloat = 0
        uiColor.getRed(&r, green: &g, blue: &b, alpha: &a)
        red = Double(r)
        green = Double(g)
        blue = Double(b)
        alpha = Double(a)
    }
}

// MARK: - Keyboard Color Settings

struct KeyboardColorSettings: Codable, Equatable {
    var backgroundColor: CodableColor?
    var keyTextColor: CodableColor?
    var normalKeyFillColor: CodableColor?
    var specialKeyFillColor: CodableColor?
    var candidateTextColor: CodableColor?
    var candidateBackgroundColor: CodableColor?

    static let `default` = KeyboardColorSettings()
}

// MARK: - Settings Snapshot

/// Immutable snapshot of settings needed during a single render cycle.
/// Avoids repeated UserDefaults reads when rendering ~50 keys.
struct SettingsSnapshot {
    let inputMode: InputMode
    let fontType: FontType
    let keyboardLayoutType: KeyboardLayoutType
    let isTranslateSwapped: Bool
    let isTpsOrMappedToER: Bool
    let keyFontSizeScale: CGFloat
    let keyCornerRadius: CGFloat
    let colorSettings: KeyboardColorSettings
}
