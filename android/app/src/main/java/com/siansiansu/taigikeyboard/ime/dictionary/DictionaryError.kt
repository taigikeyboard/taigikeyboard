// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion

// 字典操作的型別化錯誤面 — 以 Outcome.Failure 包覆穿過 LexiconService 邊界。
// 不繼承 Throwable;Java exception 永不跨 shared-core 邊界(android-guidelines.md §10)。

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
    object DatabaseNotFound : DictionaryError()

    object DatabaseNotAvailable : DictionaryError()

    data class DatabaseConnectionFailed(
        val reason: String,
    ) : DictionaryError()

    data class QueryExecutionFailed(
        val reason: String,
    ) : DictionaryError()

    data class QueryPreparationFailed(
        val reason: String,
    ) : DictionaryError()

    object TrieNotLoaded : DictionaryError()
}
