// 中文: KeyDef → KeyboardKit KeyboardLayout 的轉換器。
// 中文: 處理 char 的全形/半形切換(isTranslateSwapped、TPS 永遠全形)、
// 中文: 各功能鍵對應的 KeyboardAction,以及方向感知的按鍵寬度。

import KeyboardKit
import UIKit

/// Converts KeyDef layout to KeyboardKit's KeyboardLayout
// 中文: 將 [[KeyDef]] 轉換為 KeyboardKit 的 KeyboardLayout。
struct LayoutConverter {
    let context: KeyboardContext
    let config: KeyboardLayout.DeviceConfiguration

    /// Converts [[KeyDef]] to KeyboardLayout
    // 中文: 進入點 — 把外部 [[KeyDef]] 轉成 KeyboardLayout。
    func convert(_ keyDefs: [[KeyDef]]) -> KeyboardLayout {
        let itemRows = keyDefs.map { row in
            row.map { keyDef in
                createItem(from: keyDef)
            }
        }
        return KeyboardLayout(itemRows: itemRows, deviceConfiguration: config)
    }

    // MARK: - Private

    private func createItem(from keyDef: KeyDef) -> KeyboardLayout.Item {
        let action = keyDefToAction(keyDef)
        let width = widthFor(keyDef)
        return action.standardLayoutItem(for: config, width: width)
    }

    /// Converts KeyDef to KeyboardAction
    // 中文: KeyDef → KeyboardAction 對應表。char 會依 isTranslateSwapped / TPS 決定半形或全形。
    private func keyDefToAction(_ keyDef: KeyDef) -> KeyboardAction {
        switch keyDef {
        case let .char(char, fullWidth):
            let isTPSLayout = SharedSettings.shared.keyboardLayoutType == .tps
            let actualChar: String = if isTPSLayout {
                // TPS layout: always use full-width (independent of isTranslateSwapped)
                fullWidth ?? char
            } else {
                context.isTranslateSwapped ? (fullWidth ?? char) : char
            }
            return .character(actualChar)

        case .shift:
            return .shift(context.keyboardCase)

        case .backspace:
            return .backspace

        case .space:
            return .space

        case .return:
            return .primary(.return)

        case .translate:
            return .custom(named: "translate")

        case .numeric:
            return .keyboardType(.numeric)

        case .symbolic:
            return .keyboardType(.symbolic)

        case .alphabetic:
            return .keyboardType(.alphabetic)

        case .globe:
            return .nextKeyboard

        case .emoji:
            return .keyboardType(.emojis)
        }
    }

    /// Determines key width
    // 中文: 依 KeyDef 種類與螢幕方向決定寬度比例;.char 回 nil 走預設輸入鍵寬。
    private func widthFor(_ keyDef: KeyDef) -> KeyboardLayout.ItemWidth? {
        // Use UIKit native API for orientation (KeyboardKit 10 no longer provides interfaceOrientation)
        let screenBounds = UIScreen.main.bounds
        let isPortrait = screenBounds.height > screenBounds.width

        switch keyDef {
        case .shift, .backspace:
            return .percentage(LayoutConstants.shiftBackspace)

        case .space:
            return .available

        case .return:
            return .percentage(
                isPortrait ? LayoutConstants.ReturnButton.portrait
                    : LayoutConstants.ReturnButton.landscape,
            )

        case .numeric, .symbolic, .alphabetic, .globe, .emoji:
            return .percentage(
                isPortrait ? LayoutConstants.BottomSystemButton.portrait
                    : LayoutConstants.BottomSystemButton.landscape,
            )

        case .translate:
            let layoutType = SharedSettings.shared.keyboardLayoutType
            let scale: CGFloat = (layoutType == .phahTaigi || layoutType == .moe1) ? 2.0 : 1.5
            return .inputPercentage(scale)

        case .char:
            return nil // Use default input width
        }
    }
}
