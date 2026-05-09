// 中文: 端到端 trace ID 機制 — TraceId.next() 在 DEBUG 配發毫秒+序號的 ID,release 走 "untraced"(zero-overhead)。
// 中文: TraceContext 用 ThreadLocal<ArrayDeque<String>> 維護當前堆疊頂端的 trace ID,
// 中文: 用於 keystroke 全鏈路 fn=field 結構化日誌(對齊 iOS [COMPOSE] fn=... 模式)。

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
