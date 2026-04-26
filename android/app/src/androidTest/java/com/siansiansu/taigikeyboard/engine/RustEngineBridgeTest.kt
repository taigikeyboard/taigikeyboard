package com.siansiansu.taigikeyboard.engine

import androidx.test.ext.junit.runners.AndroidJUnit4
import com.siansiansu.taigikeyboard.engine.proto.ErrorCode
import com.siansiansu.taigikeyboard.ime.core.logging.LoggerBackend
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import java.util.concurrent.CopyOnWriteArrayList

/**
 * D9.2 platform-side acceptance tests for the Rust shared-core FFI on Android.
 *
 * **Requires** the dev `.so` built by `engine/scripts/build-android-libs-dev.sh`
 * (i.e. with the `panic-injector` Cargo feature) so `panicForTest` resolves.
 * A release `.so` raises `UnsatisfiedLinkError` on the T1 test.
 *
 * Test ID map mirrors `RustEngineBridgeTests.swift` (iOS).
 */
@RunWith(AndroidJUnit4::class)
class RustEngineBridgeTest {
    private val recordingBackend = RecordingLoggerBackend()

    @Before
    fun setUp() {
        RustEngineBridge.install(recordingBackend)
        recordingBackend.clear()
    }

    // region Op tests

    @Test fun op_tlToPoj_keyboardRealWorld() {
        assertEquals("góa", RustEngineBridge.tlToPoj("guá"))
    }

    @Test fun op_pojToTl_canonical() {
        assertEquals("guá", RustEngineBridge.pojToTl("góa"))
    }

    @Test fun op_normalizeTone_canonical() {
        assertEquals("guá", RustEngineBridge.normalizeTone("gua2"))
    }

    @Test fun op_stripTone_canonical() {
        assertEquals("gua2", RustEngineBridge.stripTone("guá"))
    }

    // endregion

    // region T1 — panic at FFI

    @Test fun T1_panicForTest_returnsFailInternal_processSurvives() {
        val response = RustEngineBridge.panicForTestRaw()
        assertNotNull(response)
        assertEquals(ErrorCode.FAIL_INTERNAL, response!!.error)
    }

    // endregion

    // region T4 — malformed protobuf

    @Test fun T4_malformedBytes_returnsFailParse() {
        val garbage = byteArrayOf(0xFF.toByte(), 0xFE.toByte(), 0xFD.toByte(), 0x01, 0x02)
        val response = RustEngineBridge.sendRawBytes(garbage)
        assertNotNull(response)
        assertEquals(ErrorCode.FAIL_PARSE, response!!.error)
    }

    // endregion

    // region T5 — oversized + boundary

    @Test fun T5_overCap_returnsFailInvariant() {
        val oversized = ByteArray(2 * 1024 * 1024 + 1)
        val response = RustEngineBridge.sendRawBytes(oversized)
        assertNotNull(response)
        assertEquals(ErrorCode.FAIL_INVARIANT, response!!.error)
    }

    @Test fun T5_atCap_acceptedByGuard() {
        val atCap = ByteArray(2 * 1024 * 1024)
        val response = RustEngineBridge.sendRawBytes(atCap)
        assertNotNull(response)
        // The pre-copy guard accepts at-cap; inner decode then fails.
        assertNotEquals(ErrorCode.FAIL_INVARIANT, response!!.error)
    }

    // endregion

    // region T6 — logging round-trip

    @Test fun T6_loggerRoundTrip_warningReachesPlatformSink() {
        // Drive a warning by sending malformed bytes — `phonetics::api::run_request`
        // logs at warn level on decode failure.
        RustEngineBridge.sendRawBytes(byteArrayOf(0xFF.toByte(), 0xFE.toByte()))
        Thread.sleep(50) // give the JNI dispatch a moment to land on the JVM thread
        assertTrue(
            "expected at least one log line, got ${recordingBackend.lines.size}",
            recordingBackend.lines.isNotEmpty()
        )
    }

    // endregion

    // region T7' — empty bytes

    @Test fun T7prime_emptyBytes_returnsErrorSentinel() {
        val response = RustEngineBridge.sendRawBytes(byteArrayOf())
        assertNotNull(response)
        assertTrue(
            "expected FAIL_INVARIANT or FAIL_PARSE, got ${response!!.error}",
            response.error == ErrorCode.FAIL_INVARIANT || response.error == ErrorCode.FAIL_PARSE
        )
    }

    // endregion
}

private class RecordingLoggerBackend : LoggerBackend {
    val lines = CopyOnWriteArrayList<String>()

    override val isDebugEnabled: Boolean = true

    fun clear() {
        lines.clear()
    }

    override fun d(tag: String, msg: String) {
        lines += "[D] $tag: $msg"
    }

    override fun i(tag: String, msg: String) {
        lines += "[I] $tag: $msg"
    }

    override fun w(tag: String, msg: String, t: Throwable?) {
        lines += "[W] $tag: $msg"
    }

    override fun e(tag: String, msg: String, t: Throwable?) {
        lines += "[E] $tag: $msg"
    }
}
