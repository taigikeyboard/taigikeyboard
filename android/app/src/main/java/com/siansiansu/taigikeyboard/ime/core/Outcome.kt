// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion
package com.siansiansu.taigikeyboard.ime.core

/**
 * Typed result of a fallible operation at a module boundary.
 *
 * Maps 1:1 to Rust's `enum Result<T, E> { Ok(T), Err(E) }`. Used instead of
 * Java `Throwable` propagation so shared-core candidate files never depend
 * on JVM exception types (`rules/android-guidelines.md` §10).
 *
 * Consumers destructure via `when` — no extension helpers are defined
 * here because shared-core candidate files forbid `inline` and
 * boundary-crossing extension functions (`rules/android-guidelines.md` §1).
 */
sealed class Outcome<out T, out E> {
    data class Success<out T>(
        val value: T,
    ) : Outcome<T, Nothing>()

    data class Failure<out E>(
        val error: E,
    ) : Outcome<Nothing, E>()
}
