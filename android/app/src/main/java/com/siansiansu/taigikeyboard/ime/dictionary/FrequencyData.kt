// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion
package com.siansiansu.taigikeyboard.ime.dictionary

/**
 * User-frequency lookup result for a single word. Hoisted from
 * `UserFrequencyService` to mirror iOS `Lexicon/Models/FrequencyData.swift`
 * so that shared-core scoring code can depend on the type without reaching
 * into the platform service.
 */
data class FrequencyData(
    val count: Int,
    val lastUsedMillis: Long,
) {
    companion object {
        val EMPTY = FrequencyData(count = 0, lastUsedMillis = 0L)
    }
}
