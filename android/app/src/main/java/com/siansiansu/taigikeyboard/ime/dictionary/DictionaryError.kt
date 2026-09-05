// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion

package com.siansiansu.taigikeyboard.ime.dictionary

/**
 * Typed failure surface for dictionary operations.
 *
 * Carried inside [com.siansiansu.taigikeyboard.ime.core.Outcome.Failure] at
 * `LexiconService` public boundaries. No `Throwable` parent — Java
 * exceptions never cross shared-core boundaries per
 * `.claude/rules/android-guidelines.md` §10.
 */
sealed class DictionaryError {
    data class QueryExecutionFailed(
        val reason: String,
    ) : DictionaryError()
}
