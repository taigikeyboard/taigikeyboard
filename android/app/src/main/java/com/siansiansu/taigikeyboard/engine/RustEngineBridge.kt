package com.siansiansu.taigikeyboard.engine

import com.siansiansu.taigikeyboard.engine.proto.ErrorCode
import com.siansiansu.taigikeyboard.engine.proto.PhoneticsRequest
import com.siansiansu.taigikeyboard.engine.proto.Request
import com.siansiansu.taigikeyboard.engine.proto.Response
import com.siansiansu.taigikeyboard.ime.core.logging.LoggerBackend
import com.siansiansu.taigikeyboard.ime.core.logging.NullLoggerBackend
import java.util.concurrent.atomic.AtomicInteger

/**
 * Thin Kotlin wrapper around the Rust shared-core FFI exposed by
 * `engine/android-jni/src/lib.rs`.
 *
 * In D9.2 this bridge is **not** wired into the IME runtime — production
 * phonetics conversion still flows through `TaigiPhonetics`. The bridge
 * exists so instrumented tests can prove `librust_taigi.so` loads and
 * produces the right answer. It will replace `TaigiPhonetics` only after
 * D9.4 proves INVARIANT_* parity.
 */
object RustEngineBridge {
    init {
        System.loadLibrary("rust_taigi")
    }

    @Volatile
    private var installedBackend: LoggerBackend = NullLoggerBackend

    @Volatile
    private var installed: Boolean = false

    private val installLock = Any()
    private val nextId = AtomicInteger(0)

    /** Idempotent. Stash the backend then ask Rust to install the JNI bridge. */
    @JvmStatic
    fun install(backend: LoggerBackend) {
        synchronized(installLock) {
            installedBackend = backend
            if (!installed) {
                registerLogger()
                installed = true
            }
        }
    }

    fun tlToPoj(input: String): String =
        sendPhonetics(PhoneticsRequest.Op.OP_TL_TO_POJ, input)

    fun pojToTl(input: String): String =
        sendPhonetics(PhoneticsRequest.Op.OP_POJ_TO_TL, input)

    fun normalizeTone(input: String): String =
        sendPhonetics(PhoneticsRequest.Op.OP_NORMALIZE_TONE, input)

    fun stripTone(input: String): String =
        sendPhonetics(PhoneticsRequest.Op.OP_STRIP_TONE, input)

    // region Test seam (used by RustEngineBridgeTest)

    /** Sends arbitrary bytes for T4 / T5 / T7' tests. Returns null on parse failure. */
    fun sendRawBytes(bytes: ByteArray): Response? {
        val responseBytes = processRequestBytes(bytes)
        return runCatching { Response.parseFrom(responseBytes) }.getOrNull()
    }

    /** Drives T1. Throws UnsatisfiedLinkError on release `.so` (no panic-injector). */
    fun panicForTestRaw(): Response? {
        val responseBytes = panicForTest()
        return runCatching { Response.parseFrom(responseBytes) }.getOrNull()
    }

    // endregion

    // region JNI

    @JvmStatic
    private external fun processRequestBytes(bytes: ByteArray): ByteArray

    @JvmStatic
    private external fun registerLogger()

    /**
     * T1 panic injector. Available only in dev `.so` builds (built with
     * `--features panic-injector`). Calling on a release `.so` raises
     * `UnsatisfiedLinkError` because the symbol is `#[cfg]`-removed at compile
     * time. Production code never references this method.
     */
    @JvmStatic
    private external fun panicForTest(): ByteArray

    /**
     * Called from native code via cached `JStaticMethodID`. MUST stay
     * `@JvmStatic` with this exact signature `(I, Ljava/lang/String;,
     * Ljava/lang/String;)V` — see `engine/android-jni/src/lib.rs`
     * `DISPATCH_SIG`.
     */
    @JvmStatic
    fun dispatchLog(level: Int, tag: String, msg: String) {
        val backend = installedBackend
        when (level) {
            LEVEL_ERROR -> backend.e(tag, msg)
            LEVEL_WARN -> backend.w(tag, msg)
            LEVEL_INFO -> backend.i(tag, msg)
            LEVEL_DEBUG -> backend.d(tag, msg)
            else -> backend.d(tag, msg)
        }
    }

    // endregion

    // region Private dispatch

    private fun sendPhonetics(op: PhoneticsRequest.Op, input: String): String {
        val request = Request.newBuilder()
            .setId(nextId.incrementAndGet())
            .setPhonetics(
                PhoneticsRequest.newBuilder()
                    .setOp(op)
                    .setInput(input)
                    .build()
            )
            .build()
        val response = sendRawBytes(request.toByteArray()) ?: return input
        if (response.error != ErrorCode.OK) return input
        if (!response.hasPhonetics()) return input
        return response.phonetics.output
    }

    private const val LEVEL_ERROR = 0
    private const val LEVEL_WARN = 1
    private const val LEVEL_INFO = 2
    private const val LEVEL_DEBUG = 3

    // endregion
}
