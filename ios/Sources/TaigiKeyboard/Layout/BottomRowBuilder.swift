import KeyboardKit
import SwiftUI

/// 底部列建構器
struct BottomRowBuilder {
    let deviceConfig: DeviceConfiguration
    let settings: SharedSettings
    let config: KeyboardLayout.DeviceConfiguration

    func buildBottomRow(context: KeyboardContext) -> KeyboardLayout.ItemRow {
        var row: [KeyboardLayout.Item] = []

        addLanguageKey(to: &row, context: context)
        addNumericKey(to: &row, context: context)
        addLocaleKeyIfNeeded(to: &row, context: context)
        addEmojiKey(to: &row, context: context)
        addSpaceKey(to: &row)
        addHyphenKeyIfNeeded(to: &row)
        addTranslateAndReturnKeys(to: &row, context: context)

        return row
    }

    // MARK: - Key Builders

    /// Language 按鈕 - 只在 iPad 上顯示，放在最左邊
    private func addLanguageKey(to row: inout [KeyboardLayout.Item], context: KeyboardContext) {
        guard deviceConfig.isIPad else { return }
        row.append(item(.nextKeyboard, width: systemButtonWidth(for: context)))
    }

    /// 123 按鈕
    private func addNumericKey(to row: inout [KeyboardLayout.Item], context: KeyboardContext) {
        row.append(item(.keyboardType(.numeric), width: systemButtonWidth(for: context)))
    }

    /// 鍵盤切換按鈕 - 只在小螢幕 iPhone 上顯示，放在 Settings 左邊
    private func addLocaleKeyIfNeeded(to row: inout [KeyboardLayout.Item], context: KeyboardContext) {
        guard deviceConfig.isSmallIPhone else { return }
        row.append(item(.nextKeyboard, width: systemButtonWidth(for: context)))
    }

    /// 表情符號按鈕
    private func addEmojiKey(to row: inout [KeyboardLayout.Item], context: KeyboardContext) {
        row.append(item(.keyboardType(.emojis), width: systemButtonWidth(for: context)))
    }

    /// 空白鍵
    private func addSpaceKey(to row: inout [KeyboardLayout.Item]) {
        row.append(item(.space, width: .available))
    }

    /// 連字符鍵 - 在非 phahTaigi 佈局時顯示
    private func addHyphenKeyIfNeeded(to row: inout [KeyboardLayout.Item]) {
        guard !settings.phahTaigiLayoutEnabled else { return }
        row.append(item(.character("-"), width: .input))
    }

    /// Translate 和 Return 按鈕
    private func addTranslateAndReturnKeys(to row: inout [KeyboardLayout.Item], context: KeyboardContext) {
        row.append(item(.custom(named: "translate"), width: .input))
        row.append(item(.primary(.return), width: returnButtonWidth(for: context)))
    }

    // MARK: - Helper

    private func item(
        _ action: KeyboardAction,
        width: KeyboardLayout.ItemWidth? = nil
    ) -> KeyboardLayout.Item {
        action.standardLayoutItem(for: config, width: width)
    }

    /// 系統按鍵寬度（123, 😀, 地球鍵等）
    private func systemButtonWidth(for context: KeyboardContext) -> KeyboardLayout.ItemWidth {
        let isPortrait = context.interfaceOrientation.isPortrait
        return .percentage(
            isPortrait ? LayoutConstants.BottomSystemButton.portrait
                       : LayoutConstants.BottomSystemButton.landscape
        )
    }

    /// Return 按鍵寬度
    private func returnButtonWidth(for context: KeyboardContext) -> KeyboardLayout.ItemWidth {
        let isPortrait = context.interfaceOrientation.isPortrait
        return .percentage(
            isPortrait ? LayoutConstants.ReturnButton.portrait
                       : LayoutConstants.ReturnButton.landscape
        )
    }
}
