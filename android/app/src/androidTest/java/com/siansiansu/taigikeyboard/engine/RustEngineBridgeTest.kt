package com.siansiansu.taigikeyboard.engine

import androidx.test.ext.junit.runners.AndroidJUnit4
import com.siansiansu.taigikeyboard.engine.proto.ErrorCode
import com.siansiansu.taigikeyboard.ime.core.logging.LoggerBackend
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import java.util.concurrent.CopyOnWriteArrayList

/**
 * D9.2 + D9.4 platform-side acceptance tests for the Rust shared-core FFI on Android.
 *
 * **Requires** the dev `.so` built by `engine/scripts/build-android-libs-dev.sh`
 * (with the `panic-injector` Cargo feature) so `panicForTest` resolves.
 *
 * Keeps the D9.2 lifecycle / FFI-safety tests (T1/T4/T5/T6/T7') plus one
 * result-decode pin per phonetics op shape the Kotlin bridge maps itself
 * (both `tpsInputAdjust` `replaceLast` branches). Op behaviour lives in
 * `engine/phonetics/tests/op_coverage.rs`; the 2 MB cap boundary in
 * `engine/dispatch/src/lib.rs` (`request_cap_accepts_exactly_the_cap_…`).
 */
@RunWith(AndroidJUnit4::class)
class RustEngineBridgeTest {
    private val recordingBackend = RecordingLoggerBackend()

    @Before
    fun setUp() {
        RustEngineBridge.install(recordingBackend)
        recordingBackend.clear()
    }

    // region Phonetics core

    @Test fun op_tlToPoj_canonical() {
        assertEquals("góa", RustEngineBridge.tlToPoj("guá"))
    }

    // endregion
    // region TPS

    @Test fun op_isTpsToneMark_acuteIsToneMark() {
        assertTrue(RustEngineBridge.isTpsToneMark('ˊ'))
    }

    @Test fun op_isTpsToneMark_letterIsNotToneMark() {
        assertFalse(RustEngineBridge.isTpsToneMark('a'))
    }

    // The engine's attaching set (`phonetics::punctuation`, full roster pinned
    // there) through the bridge `TextInputKeyHandler.commitNonComposingCharacter` reads.
    @Test fun op_isAttachingPunctuation_closingAttaches_openingDoesNot() {
        assertTrue(RustEngineBridge.isAttachingPunctuation("？"))
        assertTrue(RustEngineBridge.isAttachingPunctuation("」"))
        assertFalse(RustEngineBridge.isAttachingPunctuation("「"))
        assertFalse(RustEngineBridge.isAttachingPunctuation("?!"))
    }

    @Test fun op_isJoinerPunctuation_onlyTheKhinsiannDot() {
        assertTrue(RustEngineBridge.isJoinerPunctuation("\u00B7"))
        assertFalse(RustEngineBridge.isJoinerPunctuation("?"))
        assertFalse(RustEngineBridge.isJoinerPunctuation("-"))
    }

    // The engine's digit-tone fold (`phonetics::external_lookup`, full table
    // pinned there) through the bridge `ExternalLookupURLBuilder` reads; the
    // `o͘` row is the parity-correction (the Kotlin copy left `ho͘2`).
    @Test fun op_externalLookupDigitForm_foldsTonesAndODot() {
        assertEquals("tai5-gi2", RustEngineBridge.externalLookupDigitForm("Tâi-gí"))
        assertEquals("hoo2", RustEngineBridge.externalLookupDigitForm("ho͘" + "2"))
        assertEquals("sann", RustEngineBridge.externalLookupDigitForm("sann1"))
    }

    @Test fun op_tpsInputAdjust_dualForm() {
        val outcome = RustEngineBridge.tpsInputAdjust("ㄇ", "ㄚ")
        assertEquals("ㆬ", outcome.adjusted)
        assertNull(outcome.replaceLast)
    }

    @Test fun op_tpsInputAdjust_palatalization() {
        val outcome = RustEngineBridge.tpsInputAdjust("ㄧ", "ㄗ")
        assertEquals("ㄧ", outcome.adjusted)
        assertEquals("ㄐ", outcome.replaceLast)
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
    // region T5 — oversized

    @Test fun T5_overCap_returnsFailInvariant() {
        val oversized = ByteArray(2 * 1024 * 1024 + 1)
        val response = RustEngineBridge.sendRawBytes(oversized)
        assertNotNull(response)
        assertEquals(ErrorCode.FAIL_INVARIANT, response!!.error)
    }

    // endregion
    // region T6 — logging round-trip

    @Test fun T6_loggerRoundTrip_warningReachesPlatformSink() {
        RustEngineBridge.sendRawBytes(byteArrayOf(0xFF.toByte(), 0xFE.toByte()))
        Thread.sleep(50)
        assertTrue(
            "expected at least one log line, got ${recordingBackend.lines.size}",
            recordingBackend.lines.isNotEmpty(),
        )
    }

    // endregion
    // region T7' — empty bytes

    @Test fun T7prime_emptyBytes_returnsErrorSentinel() {
        val response = RustEngineBridge.sendRawBytes(byteArrayOf())
        assertNotNull(response)
        assertTrue(
            "expected FAIL_INVARIANT or FAIL_PARSE, got ${response!!.error}",
            response.error == ErrorCode.FAIL_INVARIANT || response.error == ErrorCode.FAIL_PARSE,
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

    override fun d(
        tag: String,
        msg: String,
    ) {
        lines += "[D] $tag: $msg"
    }

    override fun i(
        tag: String,
        msg: String,
    ) {
        lines += "[I] $tag: $msg"
    }

    override fun w(
        tag: String,
        msg: String,
        t: Throwable?,
    ) {
        lines += "[W] $tag: $msg"
    }

    override fun e(
        tag: String,
        msg: String,
        t: Throwable?,
    ) {
        lines += "[E] $tag: $msg"
    }
}
