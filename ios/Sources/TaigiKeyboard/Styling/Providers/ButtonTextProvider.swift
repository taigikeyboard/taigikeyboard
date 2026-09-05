import Foundation
import KeyboardKit

/// Button text provider — second in the render chain after `ButtonImageProvider`.
///
/// Returns the text label and hint for a key. Returns nil to preserve KeyboardKit's default content.
/// Handles: character case transforms, display overrides (˙→·, nn→ⁿ), tone diacritics,
/// TPS callout hints, MOE punctuation hints, keyboard-type labels, and composing confirm text.
///
/// Created by: `TaigiKeyboardView.RenderProviders`
/// Queried by: `TaigiButtonContent.body` (text branch, after image check)
/// Depends on: `SettingsSnapshot`, `KeyboardContext`, `TaigiCallouts.TPSCallouts`
final class ButtonTextProvider {
    /// MOE1/MOE2 punctuation hint mappings.
    /// Shared between `isTextHint` (font style) and `buttonHintText` (content)
    /// so they cannot drift apart.
    private static let moePunctuationHints: [String: String] = [
        "-": "@",
        ",": ":;", "，": ":;",
        ".": "!?", "。": "!?",
    ]

    private let keyboardContext: KeyboardContext
    private let settings: SettingsSnapshot

    init(keyboardContext: KeyboardContext, settings: SettingsSnapshot) {
        self.keyboardContext = keyboardContext
        self.settings = settings
    }

    // MARK: - Hint Classification

    /// Returns true if the hint is a TPS callout hint (larger hint, smaller main text).
    func isTPSHint(for action: KeyboardAction) -> Bool {
        guard settings.keyboardLayoutType == .tps,
              case let .character(char) = action else { return false }
        return TaigiCallouts.TPSCallouts.actions[char] != nil
    }

    /// Returns true if the hint is a text hint (smaller font), false for standalone diacritics (larger font).
    /// Used by `TaigiButtonContent.standardHintContent` to choose hint font size.
    func isTextHint(for action: KeyboardAction) -> Bool {
        let layoutType = settings.keyboardLayoutType
        guard case let .character(char) = action else { return false }

        // TPS layout: callout hints are text hints
        if layoutType == .tps {
            return TaigiCallouts.TPSCallouts.actions[char] != nil
        }

        guard layoutType == .moe1 || layoutType == .moe2 else { return false }
        return Self.moePunctuationHints[char] != nil
    }

    /// Vertical offset for tone hint diacritics above number keys.
    /// ˈ (U+02C8, tone 8) sits lower than other diacritics, so it uses a smaller offset
    /// to keep visual distance from the key label consistent.
    func hintOffsetY(for action: KeyboardAction) -> CGFloat {
        if case let .character(char) = action, char == "8" {
            return 3
        }
        return 6
    }

    // MARK: - Hint Text

    /// Returns the hint text displayed above a key label:
    /// - TPS layout: callout variants (space-separated)
    /// - Number keys: standalone tone diacritics (space placeholder for 0/1/4 to keep alignment)
    /// - MOE1/MOE2: punctuation hints on `-`, `,`, `.` keys
    /// Returns nil for keys without hints, or in English mode / non-MOE layouts.
    func buttonHintText(for action: KeyboardAction) -> String? {
        let layoutType = settings.keyboardLayoutType
        guard case let .character(char) = action else { return nil }

        // TPS layout: show callout variants as hint
        if layoutType == .tps {
            return tpsHintText(for: char)
        }

        guard settings.inputMode != .english else { return nil }

        // Tone diacritics for number keys
        if let tone = toneHintText(for: char) {
            return tone
        }

        // MOE1/MOE2 punctuation hints
        if layoutType == .moe1 || layoutType == .moe2 {
            return Self.moePunctuationHints[char]
        }

        return nil
    }

    // MARK: - Button Text

    /// Returns the text label for the given keyboard action.
    /// Returns nil to let KeyboardKit render its default content.
    func buttonText(for action: KeyboardAction) -> String? {
        switch action {
        case let .character(char):
            characterText(for: char)
        case .keyboardType(.numeric):
            "123"
        case .keyboardType(.alphabetic):
            "ABC"
        case .keyboardType(.symbolic):
            "#+="
        case .space:
            nil
        case .primary(.return) where keyboardContext.isComposingText:
            confirmKeyText()
        case .settings:
            nil
        case let .custom(name):
            name == "translate" ? nil : name
        default:
            nil
        }
    }

    // MARK: - Private Helpers

    /// TPS callout hint: space-separated variant characters.
    private func tpsHintText(for char: String) -> String? {
        TaigiCallouts.TPSCallouts.actions[char]?.joined(separator: " ")
    }

    /// Tone diacritic hint for number keys (0-9).
    /// Returns a space placeholder for keys without a visible tone mark (0, 1, 4)
    /// to keep the vertical layout consistent across the number row.
    private func toneHintText(for char: String) -> String? {
        switch char {
        case "0", "1", "4": " "
        case "2": "\u{02CA}" // ˊ MODIFIER LETTER ACUTE ACCENT
        case "3": "\u{02CB}" // ˋ MODIFIER LETTER GRAVE ACCENT
        case "5": "\u{02C6}" // ˆ MODIFIER LETTER CIRCUMFLEX ACCENT
        case "6": "\u{02C7}" // ˇ CARON
        case "7": "\u{02C9}" // ˉ MODIFIER LETTER MACRON
        case "8": "\u{02C8}" // ˈ MODIFIER LETTER VERTICAL LINE
        case "9":
            // POJ: breve, TL: double prime (more visible than U+02DD)
            settings.inputMode == .poj ? "\u{02D8}" : "\u{02BA}"
        default:
            nil
        }
    }

    /// Character key text with display overrides and case transforms.
    private func characterText(for char: String) -> String {
        // Display label override for hard-to-see characters
        if char == "˙" { return "·" } // U+02D9 → U+00B7 (middle dot)

        // TPS layout: display full-width comma
        if char == ",", settings.keyboardLayoutType == .tps { return "，" }

        // "nn" key shows nasal marker ⁿ/ᴺ in POJ mode
        // In TL mode, display as literal "nn" (falls through to case transform)
        if char == "nn", settings.inputMode == .poj {
            return keyboardContext.keyboardCase == .lowercased
                ? "\u{207F}" : "\u{1D3A}" // ⁿ / ᴺ
        }

        switch keyboardContext.keyboardCase {
        case .capsLocked:
            return char.uppercased() // "tsh" → "TSH"
        case .uppercased:
            return char.capitalized // "tsh" → "Tsh"
        case .lowercased:
            return char.lowercased()
        @unknown default:
            return char.lowercased()
        }
    }

    /// Confirmation text for the Enter key during composing mode.
    /// TPS/translate-swapped: "選"; POJ: "soán"; TL/English: "suán"
    private func confirmKeyText() -> String {
        // TPS layout: always show "選" (bopomofo users don't read romanization)
        if settings.inputMode == .tps {
            return "選"
        }

        if settings.isTranslateSwapped {
            return "選"
        }

        switch settings.inputMode {
        case .poj:
            return "soán"
        case .tl, .english:
            return "suán"
        case .tps:
            // Unreachable — handled by early return above
            return "選"
        }
    }
}
