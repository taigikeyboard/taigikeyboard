package com.siansiansu.taigikeyboard.ime.core.logging

import com.siansiansu.taigikeyboard.BuildConfig
import java.util.ArrayDeque
import java.util.concurrent.atomic.AtomicLong

object TraceId {
    const val untraced: String = "untraced"

    private val seq = AtomicLong(0L)

    fun next(): String =
        if (BuildConfig.DEBUG) "${System.currentTimeMillis()}-${seq.incrementAndGet()}" else untraced
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
