import KeyboardKit
import Foundation

/// Button text provider
///
/// Returns the text label for a key. Returns nil to use KeyboardKit's default content.
class ButtonTextProvider {

    private let keyboardContext: KeyboardContext
    private let settings: SettingsSnapshot

    init(keyboardContext: KeyboardContext, settings: SettingsSnapshot) {
        self.keyboardContext = keyboardContext
        self.settings = settings
    }

    /// Returns true if the hint is a TPS callout hint (larger hint, smaller main text).
    func isTPSHint(for action: KeyboardAction) -> Bool {
        guard settings.keyboardLayoutType == .tps,
              case let .character(char) = action else { return false }
        return Callouts.TPSCallouts.actions[char] != nil
    }

    /// Returns true if the hint is a text hint (smaller font), false for standalone diacritics (larger font).
    func isTextHint(for action: KeyboardAction) -> Bool {
        let layoutType = settings.keyboardLayoutType
        guard case let .character(char) = action else { return false }

        // TPS layout: callout hints are text hints
        if layoutType == .tps {
            return Callouts.TPSCallouts.actions[char] != nil
        }

        guard layoutType == .moe1 || layoutType == .moe2 else { return false }
        return char == "-" || char == "," || char == "，" || char == "." || char == "。"
    }

    /// Standalone diacritics for tone hints on number keys.
    /// Returns a space for keys without a tone mark (0, 1, 4) to keep vertical alignment consistent.
    /// Returns nil for TPS layout or non-number keys.
    /// Vertical offset for tone hint diacritics.
    /// ˈ (U+02C8) sits lower than other diacritics, so use a smaller offset to keep visual distance consistent.
    func hintOffsetY(for action: KeyboardAction) -> CGFloat {
        if case let .character(char) = action, char == "8" {
            return 3
        }
        return 6
    }

    func buttonHintText(for action: KeyboardAction) -> String? {
        let layoutType = settings.keyboardLayoutType
        guard case let .character(char) = action else { return nil }

        // TPS layout: show callout variants as hint (space-separated for multiple)
        if layoutType == .tps {
            if let actions = Callouts.TPSCallouts.actions[char] {
                return actions.joined(separator: " ")
            }
            return nil
        }

        guard settings.inputMode != .english else { return nil }

        // Tone hints for number keys (all layouts except TPS)
        switch char {
        case "0", "1", "4": return " "  // No tone mark, space placeholder for alignment
        case "2": return "\u{02CA}"  // ˊ MODIFIER LETTER ACUTE ACCENT
        case "3": return "\u{02CB}"  // ˋ MODIFIER LETTER GRAVE ACCENT
        case "5": return "\u{02C6}"  // ˆ MODIFIER LETTER CIRCUMFLEX ACCENT
        case "6": return "\u{02C7}"  // ˇ CARON
        case "7": return "\u{02C9}"  // ˉ MODIFIER LETTER MACRON
        case "8": return "\u{02C8}"  // ˈ MODIFIER LETTER VERTICAL LINE
        case "9":
            // POJ: breve, TL: double prime (more visible than U+02DD)
            return settings.inputMode == .poj ? "\u{02D8}" : "\u{02BA}"
        default: break
        }

        // MOE1/MOE2 layout: punctuation key hints
        if layoutType == .moe1 || layoutType == .moe2 {
            switch char {
            case "-": return "@"
            case ",", "，": return ":;"
            case ".", "。": return "!?"
            default: break
            }
        }

        return nil
    }

    /// Returns the text label for the given keyboard action
    func buttonText(for action: KeyboardAction) -> String? {
        switch action {
        case let .character(char):
            // Display label override for hard-to-see characters
            if char == "˙" { return "·" }  // U+02D9 → U+00B7 (middle dot)

            // TPS layout: display full-width comma
            if char == ",", settings.keyboardLayoutType == .tps { return "，" }

            // Display override: "nn" key shows nasal marker ⁿ/ᴺ in POJ mode
            // In TL mode, display as literal "nn" (falls through to normal case logic)
            if char == "nn", settings.inputMode == .poj {
                return keyboardContext.keyboardCase == .lowercased
                    ? "\u{207F}" : "\u{1D3A}"  // ⁿ / ᴺ
            }

            let currentCase = keyboardContext.keyboardCase
            switch currentCase {
            case .capsLocked:
                // Caps Lock: fully uppercase (e.g., "tsh" → "TSH")
                return char.uppercased()
            case .uppercased:
                // Sentence case: capitalize first letter only (e.g., "tsh" → "Tsh")
                return char.capitalized
            case .lowercased:
                return char.lowercased()
            @unknown default:
                return char.lowercased()
            }
        case .keyboardType(.numeric):
            return "123"
        case .keyboardType(.alphabetic):
            return "ABC"
        case .keyboardType(.symbolic):
            return "#+="
        case .space:
            return nil
        case .primary(.return) where keyboardContext.isComposingText:
            // Show confirmation text during composing mode
            return ConfirmKeyTextHelper.getConfirmKeyText()
        case .settings:
            return nil
        case let .custom(name):
            switch name {
            case "translate":
                return nil
            default:
                return name
            }
        default:
            return nil
        }
    }
}
