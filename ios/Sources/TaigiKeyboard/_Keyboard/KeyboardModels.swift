import KeyboardKit
import SwiftUI

#if canImport(UIKit)
    import UIKit
#endif

enum KeyboardModels {
    enum UI {
        enum Keyboard {
            // 動態計算按鍵高度 - iPad 適中的按鍵高度
            static var buttonHeight: CGFloat {
                UIDevice.current.userInterfaceIdiom == .pad ? 52 : 44
            }

            static let horizontalSpacing: CGFloat = 6 // 按鈕間水平間距

            static let verticalSpacing: CGFloat = 8

            static let cornerRadius: CGFloat = 5 // 保持適中的圓角

            // 數字和符號鍵盤配置 - 動態計算，iPad 適中尺寸
            static var numericButtonHeight: CGFloat {
                UIDevice.current.userInterfaceIdiom == .pad ? 62 : 55
            }

            static let numericVerticalSpacing: CGFloat = 12
        }

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

// MARK: - Expanded View Models

// MARK: - Configuration Setup

extension KeyboardModels {
    /// 設置鍵盤裝置配置
    static func setupDeviceConfiguration() {
        guard !hasSetupConfiguration else { return }

        let compactConfig = KeyboardLayout.DeviceConfiguration(
            buttonCornerRadius: Double(UI.Keyboard.cornerRadius),
            buttonInsets: EdgeInsets(
                top: UI.Keyboard.verticalSpacing / 2,
                leading: UI.Keyboard.horizontalSpacing / 2,
                bottom: UI.Keyboard.verticalSpacing / 2,
                trailing: UI.Keyboard.horizontalSpacing / 2,
            ),
            rowHeight: Double(UI.Keyboard.buttonHeight + UI.Keyboard.verticalSpacing),
        )

        KeyboardLayout.DeviceConfiguration.standardPhone = compactConfig
        KeyboardLayout.DeviceConfiguration.standardPhoneLandscape = compactConfig
        KeyboardLayout.DeviceConfiguration.standardPhoneLarge = compactConfig
        KeyboardLayout.DeviceConfiguration.standardPhoneLargeLandscape = compactConfig

        hasSetupConfiguration = true
    }

    private static var hasSetupConfiguration = false

}
