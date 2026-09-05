// ActionHandler extension: input classification utilities (composing character detection).
// ActionHandler 的輸入分類小工具 — 用來判斷一個字元要進組字模式還是直接輸出。

import Foundation

extension ActionHandler {
    /// Whether a character enters composing mode (allowlist).
    /// Letters, TPS bopomofo/tone symbols, hyphen, ˙ → composing.
    /// Everything else (punctuation, emoji, etc.) → direct output.
    // 判斷字元是否屬於 "進入組字模式" 的 allowlist。
    // 字母 / TPS 注音與調符 / 連字號 / ˙ 進組字;其餘標點與 emoji 直接送出。
    func isComposingCharacter(_ char: String) -> Bool {
        guard let first = char.first else { return false }
        // isLetter covers: a-z, A-Z (Lu/Ll), TPS bopomofo ㄅ-ㆷ (Lo),
        // TPS tone marks ˋ ˊ ˇ ˆ (Lm).
        // Three TPS tone marks are Sk (Symbol, modifier), not caught by isLetter:
        //   ˪ (U+02EA, tone 3), ˫ (U+02EB, tone 7), ˙ (U+02D9, tone 8)
        return first.isLetter || first == "-"
            || first == "\u{02EA}" || first == "\u{02EB}" || first == "\u{02D9}"
    }
}
