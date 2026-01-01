import SwiftUI

#if canImport(UIKit)
    import UIKit
#endif

enum KeyboardModels {
    enum UI {
        enum Emoji {
            static let defaultRecentCount = 50
        }
    }

    enum Fonts {
        static let openHuninnFontName = "jf-openhuninn-2.1"
        static let iansuiFontName = "Iansui-Regular"

        static let keyboardButtonFontSize: CGFloat = 22

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

        /// 根據字型類型取得對應的 Font
        static func font(for fontType: FontType, size: CGFloat) -> Font {
            switch fontType {
            case .system:
                return Font.system(size: size)
            case .openHuninn:
                return Font.custom(openHuninnFontName, size: size)
            case .iansui:
                return Font.custom(iansuiFontName, size: size)
            }
        }

        /// 根據字型類型取得對應的 UIFont（用於 UIKit 元件）
        static func uiFont(for fontType: FontType, size: CGFloat) -> UIFont {
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
