// NOTE: Not shared-core — Android-side adapter that forwards LoggerBackend
// calls to android.util.Log. The LoggerBackend interface itself (in
// LoggerBackend.kt) is the shared-core contract.

// 中文: Android 平台 logger adapter — 將 LoggerBackend 介面接到 android.util.Log。
// 中文: d/i/w/e 全部受 BuildConfig.DEBUG 控制 — release build 一律無 logcat 輸出
// 中文: (security-rules.md「Release builds must have zero logs」隱私契約;
// 中文:  ProGuard -assumenosideeffects 僅為 secondary 防線)。

package com.siansiansu.taigikeyboard.ime.core.logging

import android.util.Log
import com.siansiansu.taigikeyboard.BuildConfig

/**
 * Platform-side `LoggerBackend` that forwards to `android.util.Log`.
 * **All** levels (d / i / w / e) are gated on `BuildConfig.DEBUG`, so
 * release builds emit zero logcat output — `rules/security-rules.md`
 * "Release builds must have zero logs" (IME error paths can carry
 * user-typed text through exception context). The `BuildConfig.DEBUG`
 * guard is the PRIMARY defense; the ProGuard `-assumenosideeffects`
 * strip is only the secondary one. This mirrors the iOS `DebugLogger`
 * release no-op. Production error visibility comes from the
 * user-initiated `DiagnosticService`, never logcat.
 *
 * Note: the guard suppresses logcat output, not message construction —
 * `e(TAG, "...", throwable)` callers still build the string and capture
 * the `Throwable` before the call (unlike the lazy `debug { ... }`
 * overload). Acceptable for error paths, which are not hot.
 *
 * [isDebugEnabled] is exposed so inline-extension callers (e.g. the
 * `d { ... }` lazy overload in `LoggerBackend.kt`) can skip string
 * interpolation in release builds.
 */
class AndroidLoggerBackend : LoggerBackend {
    override val isDebugEnabled: Boolean = BuildConfig.DEBUG

    override fun d(
        tag: String,
        msg: String,
    ) {
        if (BuildConfig.DEBUG) Log.d(tag, msg)
    }

    override fun i(
        tag: String,
        msg: String,
    ) {
        if (BuildConfig.DEBUG) Log.i(tag, msg)
    }

    override fun w(
        tag: String,
        msg: String,
        t: Throwable?,
    ) {
        if (BuildConfig.DEBUG) {
            if (t != null) Log.w(tag, msg, t) else Log.w(tag, msg)
        }
    }

    override fun e(
        tag: String,
        msg: String,
        t: Throwable?,
    ) {
        if (BuildConfig.DEBUG) {
            if (t != null) Log.e(tag, msg, t) else Log.e(tag, msg)
        }
    }
}
