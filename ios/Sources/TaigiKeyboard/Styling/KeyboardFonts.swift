import SwiftUI
import UIKit

/// Font utilities for the keyboard extension and main app.
///
/// Two font resolution paths exist due to KeyboardKit's architecture:
/// - **Key rendering**: `ButtonFontProvider` (per-render `SettingsSnapshot`, called via `TaigiButtonContent`)
/// - **Toolbar / candidate UI**: `globalFont` / `globalUIFont` below (reads `SharedSettings.shared` directly,
///   because these are called from ~15 SwiftUI views outside KeyboardKit's key pipeline)
///
/// Both paths resolve the same user-selected font (System / Open Huninn / Iansui)
/// using `FontType.customFontName` as the shared font-name source.
///
/// Callers (keyboard extension only): `CandidateButtonView`, `CandidateCellHelper`,
/// `CandidateSuggestionsRow`, `ToolShortcutsToolbar`, `ExpandedCandidateOverlay` + its grid
/// cell and control button, `OneHandedModeViews`, `SymbolSelectionOverlay`,
/// `SettingsSelectionOverlay`, `TaigiButtonContent` (space label)
enum KeyboardFonts {
    /// PostScript font name for jf-openhuninn (粉圓)
    static let openHuninnFontName = "jf-openhuninn-2.1"
    /// PostScript font name for Iansui (芫荽)
    static let iansuiFontName = "Iansui-Regular"
    /// PostScript font name for GenYoMin (源樣明體)
    static let genYoMinFontName = "GenYoMin2TW-R"
    /// PostScript font name for GenYoGothic (源樣烏體)
    static let genYoGothicFontName = "GenYoGothic2TW-R"

    /// SwiftUI Font based on the user's font setting.
    /// Used by toolbar buttons and candidate views (not keyboard keys).
    static func globalFont(size: CGFloat) -> Font {
        font(for: SharedSettings.shared.fontType, size: size)
    }

    /// A custom font at a fixed size, like `Font.system(size:)`: `Font.custom(_:size:)`
    /// would scale with Dynamic Type and outgrow the fixed-height candidate bar (and the
    /// `globalUIFont` width measurement), which System never does.
    static func font(for fontType: FontType, size: CGFloat) -> Font {
        if let name = fontType.customFontName {
            return Font.custom(name, fixedSize: size)
        }
        return Font.system(size: size)
    }

    /// UIKit UIFont based on the user's font setting.
    /// Used where UIKit measurement is needed (e.g. candidate cell width calculation).
    static func globalUIFont(size: CGFloat) -> UIFont {
        if let name = SharedSettings.shared.fontType.customFontName {
            return UIFont(name: name, size: size) ?? UIFont.systemFont(ofSize: size)
        }
        return UIFont.systemFont(ofSize: size)
    }
}
