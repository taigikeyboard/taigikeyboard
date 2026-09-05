// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion

// 模組邊界回傳的 typed result(Success / Failure),對齊 Rust Result<T, E>。
// 取代 JVM Throwable 在 shared-core 候選檔流通,避免依賴 JVM 例外型別。

package com.siansiansu.taigikeyboard.ime.core

/**
 * Typed result of a fallible operation at a module boundary.
 *
 * Maps 1:1 to Rust's `enum Result<T, E> { Ok(T), Err(E) }`. Used instead of
 * Java `Throwable` propagation so shared-core candidate files never depend
 * on JVM exception types (`.claude/rules/android-guidelines.md` §10).
 *
 * Consumers destructure via `when` — no extension helpers are defined
 * here because shared-core candidate files forbid `inline` and
 * boundary-crossing extension functions (`.claude/rules/android-guidelines.md` §1).
 */
sealed class Outcome<out T, out E> {
    data class Success<out T>(
        val value: T,
    ) : Outcome<T, Nothing>()

    data class Failure<out E>(
        val error: E,
    ) : Outcome<Nothing, E>()
}
