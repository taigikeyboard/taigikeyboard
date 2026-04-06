import KeyboardKit
import UIKit

/// Converts KeyDef layout to KeyboardKit's KeyboardLayout
struct LayoutConverter {
    let context: KeyboardContext
    let config: KeyboardLayout.DeviceConfiguration

    /// Converts [[KeyDef]] to KeyboardLayout
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
