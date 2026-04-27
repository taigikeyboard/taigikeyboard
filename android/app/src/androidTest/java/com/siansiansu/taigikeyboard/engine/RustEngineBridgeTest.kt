package com.siansiansu.taigikeyboard.engine

import androidx.test.ext.junit.runners.AndroidJUnit4
import com.siansiansu.taigikeyboard.engine.proto.ErrorCode
import com.siansiansu.taigikeyboard.ime.core.logging.LoggerBackend
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
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
 * D9.4 expanded the bridge surface from 4 to 17 ops. This file keeps the
 * D9.2 lifecycle / FFI-safety tests (T1/T4/T5/T6/T7') intact and adds smoke
 * coverage for every new op. Branch-level fixture coverage already lives in
 * `engine/phonetics/tests/d9_4_ops.rs` (44 tests). Call-site parity coverage
 * lands in commit 8 alongside the production swap.
 */
@RunWith(AndroidJUnit4::class)
class RustEngineBridgeTest {
    private val recordingBackend = RecordingLoggerBackend()

    @Before
    fun setUp() {
        RustEngineBridge.install(recordingBackend)
        recordingBackend.clear()
        RustEngineBridge.resetDiagnosticsForTesting()
    }

    private fun togglesOff() = ToneTogglesCarrier(false, false)

    // region Phonetics core (9 ops)

    @Test fun op_normalizeTone_TL() {
        assertEquals(
            "guá",
            RustEngineBridge.normalizeTone("gua2", NormalizeMode.TL, togglesOff()),
        )
    }

    @Test fun op_stripTone_returnsBareAndTone() {
        val outcome = RustEngineBridge.stripTone("guá")
        assertEquals("gua", outcome.bare)
        assertEquals("2", outcome.tone)
    }

    @Test fun op_pojToTl_canonical() {
        assertEquals("guá", RustEngineBridge.pojToTl("góa"))
    }

    @Test fun op_tlToPoj_canonical() {
        assertEquals("góa", RustEngineBridge.tlToPoj("guá"))
    }

    @Test fun op_normalizeToTl_passthrough() {
        assertEquals("hoo", RustEngineBridge.normalizeToTl("hoo"))
    }

    @Test fun op_normalizeInput_extractsToneFromDiacritic() {
        assertEquals("ho2", RustEngineBridge.normalizeInput("hó"))
    }

    @Test fun op_restoreTone_returnsBareForToneMarked() {
        assertEquals("ho", RustEngineBridge.restoreTone("hó"))
    }

    @Test fun op_restoreTone_returnsNullForPlain() {
        assertNull(RustEngineBridge.restoreTone("ho"))
    }

    @Test fun op_hasToneMarks_trueForDiacritic() {
        assertTrue(RustEngineBridge.hasToneMarks("hó"))
    }

    @Test fun op_hasToneMarks_falseForPlain() {
        assertFalse(RustEngineBridge.hasToneMarks("ho"))
    }

    @Test fun op_toneVariations_lazyCache_returnsBothModes() {
        val cache = RustEngineBridge.toneVariations
        assertTrue("POJ map should populate", cache.poj.isNotEmpty())
        assertTrue("TL map should populate", cache.tl.isNotEmpty())
        assertNotNull(cache.tl["a"])
        assertNotNull(cache.poj["a"])
        assertNotNull(cache.tl["oo"])
        assertNotNull(cache.poj["o͘"])
    }

    // endregion
    // region Derivation (2 ops)

    @Test fun op_deriveNotone_strips() {
        assertEquals("gautsa", RustEngineBridge.deriveNotone("Gâu-tsá 2"))
    }

    @Test fun op_deriveAbbrev_returnsFirstCharPerSyllable() {
        assertEquals("gt", RustEngineBridge.deriveAbbrev("gâu-tsá"))
    }

    // endregion
    // region TPS (6 ops)

    @Test fun op_containsTps_trueForZhuyin() {
        assertTrue(RustEngineBridge.containsTps("ㄉㄧㄠ"))
    }

    @Test fun op_containsTps_falseForLatin() {
        assertFalse(RustEngineBridge.containsTps("tiau"))
    }

    @Test fun op_tpsToTl_basic() {
        val out = RustEngineBridge.tpsToTl("ㄉㄧㄠˊ")
        assertTrue("got: $out", out.contains("tiau"))
    }

    @Test fun op_tlNumericToTps_basic() {
        val out = RustEngineBridge.tlNumericToTps("tiau5", false)
        assertTrue("TL numeric → TPS should produce zhuyin", out.isNotEmpty())
    }

    @Test fun op_tlDisplayToTps_basic() {
        val out = RustEngineBridge.tlDisplayToTps("tiâu", false)
        assertTrue("TL display → TPS should produce zhuyin", out.isNotEmpty())
    }

    @Test fun op_isTpsToneMark_acuteIsToneMark() {
        assertTrue(RustEngineBridge.isTpsToneMark('ˊ'))
    }

    @Test fun op_isTpsToneMark_letterIsNotToneMark() {
        assertFalse(RustEngineBridge.isTpsToneMark('a'))
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

    @Test fun op_tpsInputAdjust_syllabicNasal() {
        val outcome = RustEngineBridge.tpsInputAdjust("ˊ", "ㄇ")
        assertEquals("ˊ", outcome.adjusted)
        assertEquals("ㆬ", outcome.replaceLast)
    }

    // endregion
    // region Diagnostics

    @Test fun diagnostics_initialState_isEmpty() {
        val snapshot = RustEngineBridge.diagnostics()
        assertEquals(0, snapshot.failureCount)
        assertTrue(snapshot.recentErrors.isEmpty())
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
        assertNotEquals(ErrorCode.FAIL_INVARIANT, response!!.error)
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
