import SwiftUI

#if canImport(UIKit)
    import UIKit
#endif

/// Font utilities for the keyboard extension and main app.
///
/// Two font resolution paths exist due to KeyboardKit's architecture:
/// - Keyboard keys: use `ButtonFontProvider` (fed by `SettingsSnapshot`, called by KeyboardKit internally)
/// - Toolbar / candidate UI: use `globalFont` / `globalUIFont` (our custom SwiftUI views, not in KeyboardKit's pipeline)
///
/// Both paths resolve the same user-selected font (System / Open Huninn / Iansui).
enum KeyboardModels {
    enum Fonts {
        /// PostScript font name for jf-openhuninn (粉圓)
        static let openHuninnFontName = "jf-openhuninn-2.1"
        /// PostScript font name for Iansui (芫荽)
        static let iansuiFontName = "Iansui-Regular"

        /// Returns a SwiftUI Font based on the user's font setting.
        /// Used by toolbar buttons and candidate views (not keyboard keys).
        static func globalFont(size: CGFloat) -> Font {
            let fontType = SharedSettings.shared.fontType
            switch fontType {
            case .system:
                return Font.system(size: size)
            case .openHuninn:
                return Font.custom(openHuninnFontName, size: size)
            case .iansui:
                return Font.custom(iansuiFontName, size: size)
            }
        }

        /// Returns a UIKit UIFont based on the user's font setting.
        /// Used where UIKit measurement is needed (e.g. candidate cell width calculation).
        static func globalUIFont(size: CGFloat) -> UIFont {
            let fontType = SharedSettings.shared.fontType
            switch fontType {
            case .system:
                return UIFont.systemFont(ofSize: size)
            case .openHuninn:
                return UIFont(name: openHuninnFontName, size: size) ?? UIFont.systemFont(ofSize: size)
            case .iansui:
                return UIFont(name: iansuiFontName, size: size) ?? UIFont.systemFont(ofSize: size)
            }
        }
    }
}
