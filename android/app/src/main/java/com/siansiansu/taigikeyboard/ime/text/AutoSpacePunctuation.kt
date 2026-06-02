// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion

package com.siansiansu.taigikeyboard.ime.text

// Classifies punctuation that attaches to the preceding word under auto-space.

/**
 * Punctuation classification for the auto-space "smart punctuation" swap.
 *
 * When auto-space is active, every committed word is followed by a trailing
 * space. Typing *attaching* punctuation next must move that space to AFTER the
 * punctuation (`guá ` + `?` → `guá? `), not leave it before (`guá ?`).
 */
object AutoSpacePunctuation {
    // CROSS-PLATFORM INVARIANT — mirrors
    // ios/Sources/TaigiKeyboard/Input/AutoSpacePunctuation.swift.
    // Drift causes silent divergence.
    //
    // Sentence-end + clause separators + CLOSING brackets/quotes. OPENING
    // brackets/quotes (`(（[「『`) are deliberately excluded — they need a
    // LEADING space, not attachment. ASCII straight quotes (`"` `'`) are
    // excluded because the same glyph serves as both opening and closing;
    // attaching them would corrupt `guá "…"` into `guá" …`.
    private val ATTACHING: Set<Char> =
        setOf(
            '。', '！', '？', '.', '!', '?',
            '，', ',', '、', '；', ';', '：', ':',
            ')', '）', ']', '】', '」', '』',
        )

    /** True when [text] is a single attaching-punctuation character. */
    fun isAttaching(text: String): Boolean {
        if (text.length != 1) return false
        return text[0] in ATTACHING
    }
}
