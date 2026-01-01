import KeyboardKit

/// 將 KeyDef 佈局轉換為 KeyboardKit 的 KeyboardLayout
struct LayoutConverter {
    let context: KeyboardContext
    let config: KeyboardLayout.DeviceConfiguration

    /// 將 [[KeyDef]] 轉換為 KeyboardLayout
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

    /// 將 KeyDef 轉換為 KeyboardAction
    private func keyDefToAction(_ keyDef: KeyDef) -> KeyboardAction {
        switch keyDef {
        case .char(let char, let fullWidth):
            // 根據 isTranslateSwapped 決定使用半形或全形
            let actualChar = context.isTranslateSwapped ? (fullWidth ?? char) : char
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

    /// 決定按鍵寬度
    private func widthFor(_ keyDef: KeyDef) -> KeyboardLayout.ItemWidth? {
        let isPortrait = context.interfaceOrientation.isPortrait

        switch keyDef {
        case .shift, .backspace:
            return .percentage(LayoutConstants.shiftBackspace)

        case .space:
            return .available

        case .return:
            return .percentage(
                isPortrait ? LayoutConstants.ReturnButton.portrait
                           : LayoutConstants.ReturnButton.landscape
            )

        case .numeric, .symbolic, .alphabetic, .globe, .emoji:
            return .percentage(
                isPortrait ? LayoutConstants.BottomSystemButton.portrait
                           : LayoutConstants.BottomSystemButton.landscape
            )

        case .translate:
            return .input

        case .char:
            return nil  // 使用預設 input 寬度
        }
    }
}
