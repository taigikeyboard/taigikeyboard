package com.siansiansu.taigikeyboard.engine

import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.engine.proto.ErrorCode
import com.siansiansu.taigikeyboard.engine.proto.Request
import com.siansiansu.taigikeyboard.engine.proto.Response
import com.siansiansu.taigikeyboard.ime.core.logging.LoggerBackend
import com.siansiansu.taigikeyboard.ime.core.logging.NullLoggerBackend
import java.util.concurrent.atomic.AtomicInteger

/**
 * Thin Kotlin wrapper around the Rust shared-core FFI exposed by
 * `engine/android-jni/src/lib.rs`.
 *
 * This object owns the JNI seam, [dispatch] and the logger state. The per-slice
 * ops are extension functions on it, one file per slice, and each slice file
 * defines its own DTOs as top-level types (mirrors iOS
 * `RustEngineBridge+<Slice>.swift`, which nests them through extensions —
 * Kotlin cannot nest a type into an object from another file;
 * `AppConfig` is built in `EngineAppConfig.kt`):
 * - `PhoneticsBridge.kt` — phonetics core + TPS
 * - `ComposingBridge.kt` — composing slice (10) + continuous-input (4)
 * - `NextWordBridge.kt` — NextWord slice (7 decide intents + filter)
 * - `LexiconBridge.kt` — lexicon read path
 * - `CaseTransformBridge.kt` — per-char/per-word case operations
 * - `UserDataBridge.kt` — the engine-owned user data (open, picks, dictionary pages, backup)
 * Callers outside this package import each extension by name
 * (`import com.siansiansu.taigikeyboard.engine.dictionarySearch`).
 *
 * Every slice sends through [dispatch] — one request-id allocator, one
 * JNI hop, one `try/Throwable` boundary (a JNI throw or a parse failure
 * never escapes into the IME keystroke path), one [recordFailure] sink.
 * Slices keep only their own payload check.
 *
 * Per Codex v2 §7: every op that needs `PojMarkerOptions` takes it as a
 * mandatory parameter — no `PojMarkerOptions(true, true, true)` silent default.
 *
 * Per Codex v2 §8 + v3 §7 + v4 §5: error visibility is hardened. Failures
 * are logged at warn and error level for logcat traceability — NEVER
 * throws (would kill IME mid-keystroke).
 */
object RustEngineBridge {
    init {
        System.loadLibrary("rust_taigi")
    }

    @Volatile
    private var installedBackend: LoggerBackend = NullLoggerBackend

    /** Sibling-bridge accessor for slice-level debug logging — same backend
     *  the JNI layer routes through. Read-only; mutation goes through
     *  [install]. */
    internal val backend: LoggerBackend
        get() = installedBackend

    @Volatile
    private var installed: Boolean = false

    private val installLock = Any()
    private val nextId = AtomicInteger(0)

    /**
     * Idempotent. Stash the backend then ask Rust to install the JNI bridge.
     *
     * In `BuildConfig.DEBUG` builds, also bumps Rust `log::max_level` to
     * `Debug` so dogfood traces are visible. Release stays at default `Warn`
     * so `log::debug!` / `log::info!` short-circuit before format — no JNI
     * cost for the no-op render path.
     */
    @JvmStatic
    fun install(backend: LoggerBackend) {
        synchronized(installLock) {
            installedBackend = backend
            if (!installed) {
                registerLogger()
                if (BuildConfig.DEBUG) {
                    setLogLevel(4) // 4 = Debug
                }
                installed = true
            }
        }
    }

    // region Test seam

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

    /**
     * Single request → JNI → parse → error-check hop for every sibling
     * bridge. `build` fills the slice payload (and any generation /
     * config snapshot) on a [Request.Builder] whose id is already
     * allocated from the process-wide counter. Both the JNI call and
     * `Response.parseFrom` catch `Exception` so an engine failure degrades
     * to "no candidates" instead of taking the keystroke path down; every
     * failure mode goes through [recordFailure] under `op`. `Error` is
     * deliberately NOT caught — `OutOfMemoryError` and the `LinkageError`
     * family mean the JVM is already unrecoverable, and swallowing them
     * would surface the real fault somewhere later and harder to read.
     * (`System.loadLibrary` runs in this object's initializer, so a
     * missing library fails at first access, never here.) Returns `null`
     * on any failure, else the envelope — callers check their own slice
     * payload.
     */
    internal fun dispatch(
        op: String,
        build: Request.Builder.() -> Unit,
    ): Response? {
        val request = Request
            .newBuilder()
            .setId(nextId.incrementAndGet())
            .apply(build)
            .build()
        val responseBytes = try {
            processRequestBytes(request.toByteArray())
        } catch (t: Exception) {
            recordFailure(op, "dispatch failed: $t")
            return null
        }
        val response = try {
            Response.parseFrom(responseBytes)
        } catch (t: Exception) {
            recordFailure(op, "response decode failed")
            return null
        }
        if (response.error != ErrorCode.OK) {
            recordFailure(op, "engine returned ${response.error}")
            return null
        }
        return response
    }

    @JvmStatic
    private external fun registerLogger()

    @JvmStatic
    private external fun setLogLevel(level: Int)

    @JvmStatic
    private external fun panicForTest(): ByteArray

    /**
     * Called from native code via cached `JStaticMethodID`. MUST stay
     * `@JvmStatic` with signature `(I, Ljava/lang/String;,
     * Ljava/lang/String;)V` — see `engine/android-jni/src/lib.rs`.
     */
    @JvmStatic
    fun dispatchLog(
        level: Int,
        tag: String,
        msg: String,
    ) {
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
    // region Failure log (shared by sibling bridges)

    private val failureLogLock = Any()

    /**
     * Logs an FFI failure at warn level. `internal` so sibling impl
     * objects route their own failures through the same sink; the lock
     * keeps each failure's warn / error pair adjacent in logcat.
     */
    internal fun recordFailure(
        op: String,
        message: String,
    ) {
        synchronized(failureLogLock) {
            installedBackend.w("RustEngineBridge", "[$op] $message")
            // Codex v3 §7 / v4 §5: also surface as error severity for logcat
            // traceability. The facade is itself DEBUG-gated (release builds
            // emit zero output per `security-rules.md` zero-logs contract),
            // so no outer `if (DEBUG)` guard is required. No throw — would
            // kill the IME mid-keystroke.
            installedBackend.e("RustEngineBridge", "[$op] $message")
        }
    }

    private const val LEVEL_ERROR = 0
    private const val LEVEL_WARN = 1
    private const val LEVEL_INFO = 2
    private const val LEVEL_DEBUG = 3

    // endregion
}
