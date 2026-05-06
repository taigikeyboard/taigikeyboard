// 中文: 鍵面文字 provider — 渲染鏈第二棒,接手 ButtonImageProvider 沒處理的鍵。
// 中文: 負責大小寫轉換、顯示覆寫(˙→·、nn→ⁿ)、調符 hint、TPS callout hint、
// 中文: MOE 標點 hint、鍵盤切換鍵文字、組字模式的 return 鍵文字。

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
/// Depends on: `SettingsSnapshot`, `KeyboardContext`, `Callouts.TPSCallouts`
// 中文: 鍵面文字 provider — 渲染鏈第二棒。
final class ButtonTextProvider {
    /// MOE1/MOE2 punctuation hint mappings.
    /// Shared between `isTextHint` (font style) and `buttonHintText` (content)
    /// so they cannot drift apart.
    // 中文: MOE1 / MOE2 標點鍵的 hint 對照;isTextHint 與 buttonHintText 共用同一張表避免漂移。
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
    // 中文: 是否為 TPS callout hint(hint 字較大、主鍵字較小)。
    func isTPSHint(for action: KeyboardAction) -> Bool {
        guard settings.keyboardLayoutType == .tps,
              case let .character(char) = action else { return false }
        return Callouts.TPSCallouts.actions[char] != nil
    }

    /// Returns true if the hint is a text hint (smaller font), false for standalone diacritics (larger font).
    /// Used by `TaigiButtonContent.standardHintContent` to choose hint font size.
    // 中文: 區分文字 hint(較小字)與獨立調符 hint(較大字),供 standardHintContent 決定字級。
    func isTextHint(for action: KeyboardAction) -> Bool {
        let layoutType = settings.keyboardLayoutType
        guard case let .character(char) = action else { return false }

        // TPS layout: callout hints are text hints
        if layoutType == .tps {
            return Callouts.TPSCallouts.actions[char] != nil
        }

        guard layoutType == .moe1 || layoutType == .moe2 else { return false }
        return Self.moePunctuationHints[char] != nil
    }

    /// Vertical offset for tone hint diacritics above number keys.
    /// ˈ (U+02C8, tone 8) sits lower than other diacritics, so it uses a smaller offset
    /// to keep visual distance from the key label consistent.
    // 中文: 數字鍵上方調符的垂直偏移;tone 8 (ˈ) 字符基線較低,用較小 offset 保持與鍵面字距離一致。
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
    // 中文: 鍵面上方的 hint 文字;TPS callout、數字鍵調符、MOE 標點 hint 三類。
    // 中文: 英文模式或非 MOE 佈局回 nil。
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
    // 中文: 回傳鍵面文字;nil 讓 KeyboardKit 用預設內容。
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
        Callouts.TPSCallouts.actions[char]?.joined(separator: " ")
    }

    /// Tone diacritic hint for number keys (0-9).
    /// Returns a space placeholder for keys without a visible tone mark (0, 1, 4)
    /// to keep the vertical layout consistent across the number row.
    // 中文: 0-9 數字鍵上的調符 hint;沒有可見調符的鍵(0/1/4)回空白佔位以維持垂直對齊。
    // 中文: 第 9 鍵 POJ 用 breve(˘),TL 改用 double prime(ʺ)以提升辨識度。
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
    // 中文: 字元鍵的最終顯示文字 — 套用顯示覆寫(˙→·、TPS 全形逗號、POJ 的 nn→ⁿ/ᴺ)
    // 中文: 與大小寫轉換(capsLocked / uppercased / lowercased)。
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
    // 中文: 組字模式中 Enter 鍵顯示的確認字 — TPS / 譯字模式為「選」,POJ 為「soán」,TL/英文為「suán」。
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
