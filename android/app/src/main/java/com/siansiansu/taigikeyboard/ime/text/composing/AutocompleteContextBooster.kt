// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion
package com.siansiansu.taigikeyboard.ime.text.composing

import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord

/**
 * Reorder candidates so that entries whose display-text first character is
 * a bigram-predicted next character float to the top. The prediction set
 * is computed by the caller (via `NextWordService`); this helper stays
 * pure — no DB, no async, no side effects.
 *
 * Mirrors `ios/Sources/TaigiKeyboard/NextWord/AutocompleteContextBooster.swift`.
 */
object AutocompleteContextBooster {
    /**
     * Preserves original order within the boosted and rest partitions.
     * Empty `predictedFirstChars` short-circuits: returns `words` untouched.
     */
    fun boost(
        words: List<TaigiWord>,
        predictedFirstChars: Set<String>,
    ): List<TaigiWord> {
        if (predictedFirstChars.isEmpty()) return words

        val boosted = mutableListOf<TaigiWord>()
        val rest = mutableListOf<TaigiWord>()
        for (word in words) {
            val firstChar = word.displayText.firstOrNull()?.toString()
            if (firstChar != null && firstChar in predictedFirstChars) {
                boosted.add(word)
            } else {
                rest.add(word)
            }
        }
        return boosted + rest
    }
}
