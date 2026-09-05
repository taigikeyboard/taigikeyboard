// End-to-end trace ID mechanism — TraceId.next() issues a millis+sequence ID in DEBUG, "untraced"
// in release (zero-overhead). TraceContext keeps the top-of-stack trace ID in a
// ThreadLocal<ArrayDeque<String>> for keystroke-wide fn=field structured logging (mirrors iOS [COMPOSE] fn=... pattern).

package com.siansiansu.taigikeyboard.ime.core.logging

import com.siansiansu.taigikeyboard.BuildConfig
import java.util.ArrayDeque
import java.util.concurrent.atomic.AtomicLong

object TraceId {
    const val untraced: String = "untraced"

    private val seq = AtomicLong(0L)

    fun next(): String = if (BuildConfig.DEBUG) "${System.currentTimeMillis()}-${seq.incrementAndGet()}" else untraced
}

object TraceContext {
    @PublishedApi
    internal val stack = ThreadLocal.withInitial { ArrayDeque<String>() }

    val current: String?
        get() = if (BuildConfig.DEBUG) stack.get().peekLast() else null

    inline fun <T> withTrace(
        id: String,
        body: () -> T,
    ): T {
        if (!BuildConfig.DEBUG) return body()
        val currentStack = stack.get()
        currentStack.addLast(id)
        try {
            return body()
        } finally {
            currentStack.removeLast()
        }
    }
}

inline fun LoggerBackend.tdebug(
    tag: String,
    msg: () -> String,
) {
    debug(tag) {
        val trace = TraceContext.current
        if (trace == null) msg() else "[trace=$trace] ${msg()}"
    }
}
