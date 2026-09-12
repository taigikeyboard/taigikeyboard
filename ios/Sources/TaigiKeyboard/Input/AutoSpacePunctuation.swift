// Classifies punctuation that attaches to the preceding word under auto-space.
// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.

import Foundation

/// Punctuation classification for the auto-space "smart punctuation" swap.
///
/// When auto-space is active, every committed word is followed by a trailing
/// space. Typing *attaching* punctuation next must move that space to AFTER the
/// punctuation (`guá ` + `?` → `guá? `), not leave it before (`guá ?`).
enum AutoSpacePunctuation {
    // CROSS-PLATFORM INVARIANT — mirrors
    // android/app/src/main/java/com/siansiansu/taigikeyboard/ime/text/AutoSpacePunctuation.kt.
    // Drift causes silent divergence.
    //
    // Sentence-end + clause separators + CLOSING brackets/quotes. OPENING
    // brackets/quotes (`(（[「『`) are deliberately excluded — they need a
    // LEADING space, not attachment. ASCII straight quotes (`"` `'`) are
    // excluded because the same glyph serves as both opening and closing;
    // attaching them would corrupt `guá "…"` into `guá" …`.
    private static let attaching: Set<Character> = [
        "。", "！", "？", ".", "!", "?",
        "，", ",", "、", "；", ";", "：", ":",
        ")", "）", "]", "】", "」", "』",
    ]

    /// True when `text` is a single attaching-punctuation character.
    static func isAttaching(_ text: String) -> Bool {
        guard text.count == 1, let char = text.first else { return false }
        return attaching.contains(char)
    }
}
