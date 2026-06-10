package com.siansiansu.taigikeyboard.ime.dictionary

import org.junit.Assert.assertEquals
import org.junit.Test

// Frequency CSV carries the (漢字, 羅馬字) pair.
//
// Pins the CSV side of INVARIANT_USER_FREQ_PAIR_KEY
// (docs/architecture/behavioral-invariants.md §28): the hand-editable frequency
// CSV is word,tl,count (3 columns), a legacy 2-column word,count file decodes
// with tl="", and any other column count is skipped. The format must match iOS
// CSVParsers.swift encode/decodeFrequencyCSV byte-for-byte (cross-platform CSV
// contract). The codec is pure (no SQLite), so unlike the §24/§25 user-data
// paths it runs on the JVM without the .so / native DB.
class DictionaryCsvCodecFrequencyTest {
    // MARK: encode

    @Test
    fun `INVARIANT_USER_FREQ_PAIR_KEY csv encode writes three columns`() {
        val csv =
            DictionaryCsvCodec.encodeFrequencyCSV(
                listOf(Triple("重", "tāng", 5), Triple("食", "tsia̍h", 8)),
            )
        assertEquals("重,tāng,5\n食,tsia̍h,8\n", csv)
    }

    @Test
    fun `INVARIANT_USER_FREQ_PAIR_KEY csv legacy empty tl encodes as empty middle field`() {
        val csv = DictionaryCsvCodec.encodeFrequencyCSV(listOf(Triple("食", "", 8)))
        assertEquals("食,,8\n", csv)
    }

    @Test
    fun `INVARIANT_USER_FREQ_PAIR_KEY csv homograph readings stay distinct rows`() {
        val csv =
            DictionaryCsvCodec.encodeFrequencyCSV(
                listOf(Triple("重", "tāng", 5), Triple("重", "tîng", 3)),
            )
        assertEquals("重,tāng,5\n重,tîng,3\n", csv)
    }

    // MARK: decode

    @Test
    fun `INVARIANT_USER_FREQ_PAIR_KEY csv decode three columns`() {
        val rows = DictionaryCsvCodec.decodeFrequencyCSV("重,tāng,5\n重,tîng,3\n食,tsia̍h,8\n")
        assertEquals(
            listOf(Triple("重", "tāng", 5), Triple("重", "tîng", 3), Triple("食", "tsia̍h", 8)),
            rows,
        )
    }

    @Test
    fun `INVARIANT_USER_FREQ_PAIR_KEY csv decode legacy two columns to empty tl`() {
        val rows = DictionaryCsvCodec.decodeFrequencyCSV("食,8\n重,5\n")
        assertEquals(listOf(Triple("食", "", 8), Triple("重", "", 5)), rows)
    }

    @Test
    fun `INVARIANT_USER_FREQ_PAIR_KEY csv decode empty middle field is empty tl`() {
        val rows = DictionaryCsvCodec.decodeFrequencyCSV("食,,8\n")
        assertEquals(listOf(Triple("食", "", 8)), rows)
    }

    @Test
    fun `INVARIANT_USER_FREQ_PAIR_KEY csv decode skips other column counts`() {
        // Exact discriminator: 1 col and 4 cols are skipped, not eaten as legacy.
        val rows = DictionaryCsvCodec.decodeFrequencyCSV("solo\n食,tsia̍h,8\na,b,c,d\n")
        assertEquals(listOf(Triple("食", "tsia̍h", 8)), rows)
    }

    @Test
    fun `INVARIANT_USER_FREQ_PAIR_KEY csv decode skips invalid rows`() {
        // Non-positive / non-numeric count, empty word → skipped.
        val rows = DictionaryCsvCodec.decodeFrequencyCSV("食,tsia̍h,0\n食,tsia̍h,abc\n,tsia̍h,5\n重,tāng,5\n")
        assertEquals(listOf(Triple("重", "tāng", 5)), rows)
    }

    @Test
    fun `INVARIANT_USER_FREQ_PAIR_KEY csv decode tolerates blank lines`() {
        val rows = DictionaryCsvCodec.decodeFrequencyCSV("\n食,tsia̍h,8\n\n重,tāng,5\n\n")
        assertEquals(listOf(Triple("食", "tsia̍h", 8), Triple("重", "tāng", 5)), rows)
    }

    // MARK: round-trip

    @Test
    fun `INVARIANT_USER_FREQ_PAIR_KEY csv round trip preserves pair`() {
        val original =
            listOf(
                Triple("重", "tāng", 5),
                Triple("重", "tîng", 3),
                Triple("食", "tsia̍h", 8),
                Triple("舊", "", 2), // legacy bucket survives the round-trip
            )
        assertEquals(original, DictionaryCsvCodec.decodeFrequencyCSV(DictionaryCsvCodec.encodeFrequencyCSV(original)))
    }

    @Test
    fun `INVARIANT_USER_FREQ_PAIR_KEY csv round trip escapes comma field`() {
        // A field with a comma is quote-wrapped on encode and recovered on decode
        // — pins escape↔parse consistency with iOS CSVDocument.
        val original = listOf(Triple("a,b", "tāng", 5))
        assertEquals(original, DictionaryCsvCodec.decodeFrequencyCSV(DictionaryCsvCodec.encodeFrequencyCSV(original)))
    }

    @Test
    fun `INVARIANT_USER_FREQ_PAIR_KEY csv round trip escapes quote field`() {
        // A field with a literal " survives the RFC 4180 "" round-trip — pins
        // parity with iOS CSVDocument.parseLine.
        val original = listOf(Triple("a\"b", "tāng", 5))
        assertEquals(original, DictionaryCsvCodec.decodeFrequencyCSV(DictionaryCsvCodec.encodeFrequencyCSV(original)))
    }
}
