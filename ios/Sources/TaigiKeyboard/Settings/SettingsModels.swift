import Foundation

// MARK: - Input Mode Display Name

/// Platform-side localization for `InputMode` (defined in `InputMode.swift`
/// as a Foundation-only shared-core candidate).
extension InputMode {
    var displayName: String {
        switch self {
        case .poj: SettingsTexts.pojMode
        case .tl: SettingsTexts.tlMode
        case .english: SettingsTexts.englishMode
        case .tps: SettingsTexts.tpsMode
        }
    }
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

    var displayName: String {
        switch self {
        case .system: LayoutTexts.fontSystemDefault
        case .openHuninn: CommonTexts.fontOpenHuninn
        case .iansui: CommonTexts.fontIansui
        case .genYoMin: CommonTexts.fontGenYoMin
        case .genYoGothic: CommonTexts.fontGenYoGothic
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
