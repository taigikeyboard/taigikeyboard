// ActionHandler extension: input classification utilities (composing character detection).

import Foundation

extension ActionHandler {
    /// Whether a character enters composing mode (allowlist).
    /// Letters, TPS bopomofo/tone symbols, hyphen, ˙ → composing.
    /// Everything else (punctuation, emoji, etc.) → direct output.
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
