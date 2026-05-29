package com.siansiansu.taigikeyboard.engine

import com.siansiansu.taigikeyboard.engine.LexiconBridge.DictionaryToggles
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Pins the Android kautian-subcollection wire ENCODE
 * ([LexiconBridge.encodeKautianSubcollWire]) against the same expectations as
 * the Rust golden tests in `engine/lexicon/src/dictionary_filters.rs`
 * (`subcoll_*`). This is the binary-skew fallback path; if it drifts from the
 * engine, a Kotlin-newer-than-`.so` session would silently mis-gate the
 * subcollections.
 *
 * Wire high region: bit 13 = active sentinel, bits 14..=25 = 12-bit enable mask
 * (main bit 0 | accent[10] bits 1-10 | name bit 11). The encode emits ONLY the
 * high region — source bit 0 is added separately by `platformFallbackFilters`.
 */
class KautianSubcollWireEncodeTest {
    private val activeBit: UInt = 1u shl 13
    private val lowSourceRegion: UInt = 0x1FFFu // bits 0-12 (source/variant)

    /** Extract the 12-bit subtag enable mask from the wire value. */
    private fun maskRegion(wire: UInt): UInt = (wire shr 14) and 0xFFFu

    private fun subcoll(
        lukang: Boolean = false,
        sansia: Boolean = false,
        taipak: Boolean = false,
        gilan: Boolean = false,
        tainan: Boolean = false,
        kaohsiung: Boolean = false,
        kinmen: Boolean = false,
        makung: Boolean = false,
        sintik: Boolean = false,
        taichung: Boolean = false,
        nameAppendix: Boolean = false,
    ) = DictionaryToggles.KautianSubcoll(
        lukang, sansia, taipak, gilan, tainan,
        kaohsiung, kinmen, makung, sintik, taichung, nameAppendix,
    )

    private fun toggles(
        kautian: Boolean,
        sub: DictionaryToggles.KautianSubcoll,
    ) = DictionaryToggles(
        kautian = kautian,
        taigitv = false,
        itaigi = false,
        sitbut = false,
        taihoa = false,
        taijit = false,
        kungge = false,
        stti = false,
        khpoo = false,
        variant = false,
        khiin = false,
        lkk = false,
        kautianSubcoll = sub,
    )

    private fun allSubcollOn() = subcoll(
        lukang = true, sansia = true, taipak = true, gilan = true, tainan = true,
        kaohsiung = true, kinmen = true, makung = true, sintik = true, taichung = true,
        nameAppendix = true,
    )

    /** master off ⇒ no high bits (kautian rows drop via the source-OR anyway). */
    @Test
    fun masterOff_emitsZero() {
        val wire = LexiconBridge.encodeKautianSubcollWire(toggles(kautian = false, sub = allSubcollOn()))
        assertEquals(0u, wire)
    }

    /** master on + everything on ⇒ active bit + full 12-bit mask, no low bits. */
    @Test
    fun allOn_setsActiveAndFullMask() {
        val wire = LexiconBridge.encodeKautianSubcollWire(toggles(kautian = true, sub = allSubcollOn()))
        assertTrue("active sentinel set", wire and activeBit != 0u)
        assertEquals("full 12-bit mask", 0xFFFu, maskRegion(wire))
        assertEquals("no source/variant low bits", 0u, wire and lowSourceRegion)
        // Never collides with the all-enabled sentinel.
        assertNotEquals(UInt.MAX_VALUE, wire)
    }

    /** master on + all nested off ⇒ main bit only (主條目 is not a user toggle). */
    @Test
    fun allNestedOff_keepsMainBitOnly() {
        val wire = LexiconBridge.encodeKautianSubcollWire(toggles(kautian = true, sub = subcoll()))
        assertTrue("active sentinel set", wire and activeBit != 0u)
        assertEquals("main bit only", 0x1u, maskRegion(wire))
    }

    /**
     * Each accent maps to subtag bit (1 + config.yaml dialect_columns index);
     * name_appendix → bit 11. main (bit 0) is always present.
     */
    @Test
    fun perSubcollBitPositions_matchDialectOrder() {
        val cases: List<Pair<DictionaryToggles.KautianSubcoll, Int>> = listOf(
            subcoll(lukang = true) to 1,
            subcoll(sansia = true) to 2,
            subcoll(taipak = true) to 3,
            subcoll(gilan = true) to 4,
            subcoll(tainan = true) to 5,
            subcoll(kaohsiung = true) to 6,
            subcoll(kinmen = true) to 7,
            subcoll(makung = true) to 8,
            subcoll(sintik = true) to 9,
            subcoll(taichung = true) to 10,
            subcoll(nameAppendix = true) to 11,
        )
        for ((sub, bit) in cases) {
            val wire = LexiconBridge.encodeKautianSubcollWire(toggles(kautian = true, sub = sub))
            val expectedMask = 0x1u or (1u shl bit) // main + the one toggled bit
            assertEquals("subtag bit $bit", expectedMask, maskRegion(wire))
        }
    }
}
