import KeyboardKit
import UIKit

/// Converts KeyDef layout to KeyboardKit's KeyboardLayout
struct LayoutConverter {
    let context: KeyboardContext
    let config: KeyboardLayoutConfiguration

    /// Entry point: converts [[KeyDef]] to KeyboardLayout. The 文/A key is
    /// dropped where it could flip nothing — Romanization Only (always half-width) and TPS
    /// (always full-width); `.space` is `.available`, so it takes the freed width.
    func convert(_ keyDefs: [[KeyDef]]) -> KeyboardLayout {
        let showsTranslateKey = context.candidateDisplayMode.allowsSwapToggle
            && SharedSettings.shared.keyboardLayoutType != .tps
        // Read once per layout, not per key.
        let typesFullWidth = context.isFullWidthPunctuation
        let itemRows = keyDefs.map { row in
            row.compactMap { keyDef -> KeyboardLayoutItem? in
                if case .translate = keyDef, !showsTranslateKey {
                    return nil
                }
                return createItem(from: keyDef, typesFullWidth: typesFullWidth)
            }
        }
        return KeyboardLayout(itemRows: itemRows, configuration: config)
    }

    // MARK: - Private

    private func createItem(from keyDef: KeyDef, typesFullWidth: Bool) -> KeyboardLayoutItem {
        let action = keyDefToAction(keyDef, typesFullWidth: typesFullWidth)
        let width = widthFor(keyDef)
        return action.standardLayoutItem(for: config, width: width)
    }

    /// Converts KeyDef to KeyboardAction; `.char` takes its `fullWidth` form
    /// when `typesFullWidth`.
    private func keyDefToAction(_ keyDef: KeyDef, typesFullWidth: Bool) -> KeyboardAction {
        switch keyDef {
        case let .char(char, fullWidth):
            .character(typesFullWidth ? (fullWidth ?? char) : char)

        case .shift:
            .shift(context.keyboardCase)

        case .backspace:
            .backspace

        case .space:
            .space

        case .return:
            .primary(.return)

        case .translate:
            .custom(named: "translate")

        case .numeric:
            .keyboardType(.numeric)

        case .symbolic:
            .keyboardType(.symbolic)

        case .alphabetic:
            .keyboardType(.alphabetic)

        case .globe:
            .nextKeyboard

        case .emoji:
            .keyboardType(.emojis)
        }
    }

    /// Determines key width by KeyDef kind + screen orientation; `.char`
    /// returns nil to fall back to the default input-key width.
    private func widthFor(_ keyDef: KeyDef) -> KeyboardLayoutItem.Width? {
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
