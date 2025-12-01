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
        static let extensionFontName = "jf-openhuninn-2.1"

        static let keyboardButtonFontSize: CGFloat = 22

        static func globalFont(size: CGFloat) -> Font {
            let useCustomFont = SharedSettings.shared.isCustomFontEnabled
            return useCustomFont
                ? Font.custom(extensionFontName, size: size)
                : Font.system(size: size)
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
