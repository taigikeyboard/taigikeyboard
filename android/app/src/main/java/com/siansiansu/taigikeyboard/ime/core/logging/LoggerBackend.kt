// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion
package com.siansiansu.taigikeyboard.ime.core.logging

/**
 * Shared-core logging contract. Engine code emits diagnostics through this
 * interface; platform adapters (Android, iOS, tests) decide where the
 * messages go. Mirrors iOS `Common/LoggerBackend.swift`.
 *
 * Callers on hot paths should prefer the inline `d { ... }` extension so
 * that string interpolation is skipped entirely when [isDebugEnabled] is
 * `false` — matching the old `if (BuildConfig.DEBUG) Log.d(...)` guards
 * that R8 used to dead-code-eliminate in release builds.
 */
interface LoggerBackend {
    fun d(
        tag: String,
        msg: String,
    )

    fun i(
        tag: String,
        msg: String,
    )

    fun w(
        tag: String,
        msg: String,
        t: Throwable? = null,
    )

    fun e(
        tag: String,
        msg: String,
        t: Throwable? = null,
    )

    /**
     * `true` when [d] output will actually be emitted. Inline call-sites
     * consult this flag before building a debug message so release builds
     * pay zero allocation cost.
     */
    val isDebugEnabled: Boolean
}

/** Drop-all logger for tests and shared-core-only call paths. */
object NullLoggerBackend : LoggerBackend {
    override val isDebugEnabled: Boolean = false

    override fun d(
        tag: String,
        msg: String,
    ) {}

    override fun i(
        tag: String,
        msg: String,
    ) {}

    override fun w(
        tag: String,
        msg: String,
        t: Throwable?,
    ) {}

    override fun e(
        tag: String,
        msg: String,
        t: Throwable?,
    ) {}
}

/**
 * Lazy-message debug overload. [msg] is only evaluated when
 * [LoggerBackend.isDebugEnabled] is `true`; because this function is
 * `inline`, the lambda is erased at the call site so release builds
 * allocate nothing and R8 can DCE the whole branch.
 *
 * Named `debug` (not `d`) because Kotlin resolution prefers a member
 * function with the same name — an extension `fun d(tag, () -> String)`
 * would be shadowed by the `d(tag, String)` member.
 */
inline fun LoggerBackend.debug(
    tag: String,
    msg: () -> String,
) {
    if (isDebugEnabled) d(tag, msg())
}
