import KeyboardKit
import SwiftUI

class CustomLayoutService: KeyboardLayout.StandardLayoutService {
    override func keyboardLayout(for context: KeyboardContext) -> KeyboardLayout {
        // Use custom layout for alphabetic and webSearch modes
        if context.keyboardType == .alphabetic || context.keyboardType == .webSearch {
            return createFullCustomLayout(for: context)
        }

        // Add translate button to numeric keyboard and apply full-width conversion
        if context.keyboardType == .numeric {
            let layout = super.keyboardLayout(for: context)
            let modifiedLayout = addTranslateButtonToNumericLayout(to: layout, context: context)
            let adjustedLayout = adjustReturnButtonWidth(to: modifiedLayout, context: context)
            return applyFullWidthConversion(to: adjustedLayout, context: context)
        }

        // Apply full-width conversion to symbolic keyboard
        if context.keyboardType == .symbolic {
            let layout = super.keyboardLayout(for: context)
            let adjustedLayout = adjustReturnButtonWidth(to: layout, context: context)
            return applyFullWidthConversion(to: adjustedLayout, context: context)
        }

        return super.keyboardLayout(for: context)
    }

    // MARK: - Custom Layout

    private func createFullCustomLayout(for context: KeyboardContext) -> KeyboardLayout {
        let deviceConfig = DeviceConfiguration(context: context)
        let settings = SharedSettings.shared
        let config = KeyboardLayout.DeviceConfiguration.standard(for: context)

        let alphabeticBuilder = AlphabeticLayoutBuilder(config: config)
        let bottomRowBuilder = BottomRowBuilder(
            deviceConfig: deviceConfig,
            settings: settings,
            config: config
        )

        var rows = alphabeticBuilder.buildLayout(context: context).itemRows
        rows.append(bottomRowBuilder.buildBottomRow(context: context))

        let layout = KeyboardLayout(itemRows: rows)

        // Apply full-width conversion for alphabetic layout
        return applyFullWidthConversion(to: layout, context: context)
    }

    // MARK: - Numeric Layout

    /// 在數字鍵盤的底部列空白鍵右側插入 translate 按鍵
    private func addTranslateButtonToNumericLayout(
        to layout: KeyboardLayout,
        context: KeyboardContext
    ) -> KeyboardLayout {
        var rows = layout.itemRows
        let bottomRowIndex = rows.count - 1

        guard bottomRowIndex >= 0 else { return layout }

        // 建立 translate 按鍵 item
        let config = KeyboardLayout.DeviceConfiguration.standard(for: context)
        let translateItem = KeyboardAction.custom(named: "translate").standardLayoutItem(
            for: config,
            width: .input
        )

        // 在空白鍵後插入
        rows.insert(translateItem, after: .space, inRow: bottomRowIndex)

        return KeyboardLayout(itemRows: rows)
    }

    // MARK: - Post-Processing

    /// 調整 Return 按鍵寬度，使其與 alphabetic 鍵盤一致
    private func adjustReturnButtonWidth(to layout: KeyboardLayout, context: KeyboardContext) -> KeyboardLayout {
        let isPortrait = context.interfaceOrientation.isPortrait
        let returnWidth: CGFloat = isPortrait ? LayoutConstants.ReturnButton.portrait
                                               : LayoutConstants.ReturnButton.landscape

        let adjustedRows = layout.itemRows.map { row in
            row.map { item in
                // 找到 Return 按鍵並調整寬度
                if case .primary(.return) = item.action {
                    return KeyboardLayout.Item(
                        action: item.action,
                        size: KeyboardLayout.ItemSize(
                            width: .percentage(returnWidth),
                            height: item.size.height
                        ),
                        edgeInsets: item.edgeInsets
                    )
                } else {
                    return item
                }
            }
        }

        return KeyboardLayout(itemRows: adjustedRows)
    }

    /// 當 isTranslateSwapped = true 時，將半形標點符號轉換為全形
    private func applyFullWidthConversion(to layout: KeyboardLayout, context: KeyboardContext) -> KeyboardLayout {
        guard context.isTranslateSwapped else { return layout }

        let convertedRows = layout.itemRows.map { row in
            row.map { item in
                // 轉換符號
                if case let .character(char) = item.action,
                   let fullWidthChar = PunctuationMapping.fullWidthCharacter(
                       for: char,
                       keyboardType: context.keyboardType
                   )
                {
                    return KeyboardLayout.Item(
                        action: .character(fullWidthChar),
                        size: item.size,
                        edgeInsets: item.edgeInsets
                    )
                } else {
                    return item
                }
            }
        }

        return KeyboardLayout(itemRows: convertedRows)
    }
}
