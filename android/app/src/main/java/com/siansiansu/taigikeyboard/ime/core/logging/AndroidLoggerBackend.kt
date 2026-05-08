// NOTE: Not shared-core — Android-side adapter that forwards LoggerBackend
// calls to android.util.Log. The LoggerBackend interface itself (in
// LoggerBackend.kt) is the shared-core contract.

// 中文: Android 平台 logger adapter — 將 LoggerBackend 介面接到 android.util.Log。
// 中文: d/i/w 受 BuildConfig.DEBUG 控制(release build dead-code 消除),e 永遠輸出。

package com.siansiansu.taigikeyboard.ime.core.logging

import android.util.Log
import com.siansiansu.taigikeyboard.BuildConfig

/**
 * Platform-side `LoggerBackend` that forwards to `android.util.Log`.
 * Debug / info / warning messages are gated on `BuildConfig.DEBUG` to
 * preserve the original `if (BuildConfig.DEBUG) Log.x(...)` semantics
 * every caller used before this class existed. Errors always log.
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
        if (t != null) Log.e(tag, msg, t) else Log.e(tag, msg)
    }
}
