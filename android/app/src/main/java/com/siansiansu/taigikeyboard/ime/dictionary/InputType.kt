// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion
package com.siansiansu.taigikeyboard.ime.dictionary

/**
 * Enum representing the type of input for dictionary search
 */
sealed class InputType {
    /** Search by romanization without tone marks (e.g., "goa") */
    object RomanWithoutTone : InputType()

    /** Search by romanization with tone marks (e.g., "góa") */
    object RomanWithTone : InputType()

    /** Search by Chinese characters (e.g., "我") */
    object Hanzi : InputType()
}
