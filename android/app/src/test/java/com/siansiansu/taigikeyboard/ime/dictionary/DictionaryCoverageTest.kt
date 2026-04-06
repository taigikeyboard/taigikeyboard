package com.siansiansu.taigikeyboard.ime.dictionary

import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels.InputMode
import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * Dictionary coverage test: verifies all dictionary syllables
 * are reachable via TPS and POJ input modes.
 *
 * Reference data generated from taigi-converter (independent implementation).
 * Regenerate: node scripts/generate-syllable-test-data.mjs
 */
class DictionaryCoverageTest {

    private data class TestRow(
        val tlNumeric: String,
        val tps: String,
        val pojDisplay: String,
        val pojNumeric: String,
    )

    private val testData: List<TestRow> by lazy {
        val stream = javaClass.classLoader!!.getResourceAsStream("syllable-test-data.csv")
            ?: error("syllable-test-data.csv not found in test resources")
        stream.bufferedReader().useLines { lines ->
            lines.drop(1) // skip header
                .filter { it.isNotBlank() }
                .map { line ->
                    val cols = line.split(",", limit = 4)
                    TestRow(cols[0], cols[1], cols[2], cols[3])
                }
                .toList()
        }
    }

    @Test
    fun testTPSPipeline_allDictionarySyllables() {
        val failures = mutableListOf<String>()
        for (row in testData) {
            val tlFromTPS = TPSConverter.toTL(row.tps)
            val normalized = InputNormalizer.normalize(tlFromTPS, InputMode.TL)
            if (normalized != row.tlNumeric) {
                failures.add(
                    "TPS '${row.tps}' -> toTL='$tlFromTPS' -> normalize='$normalized', expected '${row.tlNumeric}'"
                )
            }
        }
        if (failures.isNotEmpty()) {
            assertEquals(
                "TPS pipeline: ${failures.size}/${testData.size} failures:\n${failures.joinToString("\n")}",
                0, failures.size
            )
        }
    }

    @Test
    fun testPOJPipeline_allDictionarySyllables() {
        val failures = mutableListOf<String>()
        for (row in testData) {
            // Skip tone 4 (unmarked in POJ, works via prefix search)
            if (row.pojNumeric.isEmpty()) continue
            val normalized = InputNormalizer.normalize(row.pojDisplay, InputMode.POJ)
            if (normalized != row.pojNumeric) {
                failures.add(
                    "POJ '${row.pojDisplay}' -> normalize='$normalized', expected '${row.pojNumeric}'"
                )
            }
        }
        if (failures.isNotEmpty()) {
            assertEquals(
                "POJ pipeline: ${failures.size}/${testData.size} failures:\n${failures.joinToString("\n")}",
                0, failures.size
            )
        }
    }
}
