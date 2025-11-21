import KeyboardKit
import SwiftUI

/// Alphabetic 鍵盤佈局建構器
struct AlphabeticLayoutBuilder {
    let config: KeyboardLayout.DeviceConfiguration

    func buildLayout(context: KeyboardContext) -> KeyboardLayout {
        let rows = [
            createNumberRow(),
            createFirstLetterRow(),
            createSecondLetterRow(),
            createThirdLetterRow(for: context),
        ]

        return KeyboardLayout(itemRows: rows)
    }

    // MARK: - Row Builders

    private func createNumberRow() -> KeyboardLayout.ItemRow {
        "1234567890".map { item(.character(String($0))) }
    }

    private func createFirstLetterRow() -> KeyboardLayout.ItemRow {
        let settings = SharedSettings.shared
        let keys = settings.phahTaigiLayoutEnabled
            ? PhahTaigiMapping.firstRow
            : "qwertyuiop"
        return keys.map { item(.character(String($0))) }
    }

    private func createSecondLetterRow() -> KeyboardLayout.ItemRow {
        let settings = SharedSettings.shared

        let letters: String
        if settings.phahTaigiLayoutEnabled {
            letters = PhahTaigiMapping.secondRow
        } else {
            letters = settings.inputMode == .tl ? "asdfghjkl" : "asdfghjklo͘"
        }

        return letters.map { item(.character(String($0))) }
    }

    private func createThirdLetterRow(for context: KeyboardContext) -> KeyboardLayout.ItemRow {
        let settings = SharedSettings.shared
        let functionWidth = KeyboardLayout.ItemWidth.percentage(LayoutConstants.shiftBackspace)

        let letters = settings.phahTaigiLayoutEnabled
            ? PhahTaigiMapping.thirdRowLetters
            : "zxcvbnm"

        var row: KeyboardLayout.ItemRow = [item(.shift(context.keyboardCase), width: functionWidth)]
        row.append(contentsOf: letters.map { item(.character(String($0))) })
        row.append(item(.backspace, width: functionWidth))

        return row
    }

    // MARK: - Helper

    private func item(
        _ action: KeyboardAction,
        width: KeyboardLayout.ItemWidth? = nil
    ) -> KeyboardLayout.Item {
        action.standardLayoutItem(for: config, width: width)
    }
}
