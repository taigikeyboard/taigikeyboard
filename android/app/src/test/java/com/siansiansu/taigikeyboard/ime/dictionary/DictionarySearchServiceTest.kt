package com.siansiansu.taigikeyboard.ime.dictionary

import com.siansiansu.taigikeyboard.engine.DictionaryToggles
import com.siansiansu.taigikeyboard.engine.proto.DictionarySearchRow
import com.siansiansu.taigikeyboard.engine.proto.DictionarySourceCode
import com.siansiansu.taigikeyboard.ime.settings.StubEngineSettings
import com.siansiansu.taigikeyboard.ime.settings.StubEngineSettingsProvider
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * JVM tests for dictionary search, mirrored by iOS `DictionarySearchServiceTests`. The engine's
 * `DictionarySearch` op owns the search (golden: `engine/dispatch/tests/dictionary_search.rs`);
 * the platform side is the settings snapshot the service sends and the row decoding.
 */
class DictionarySearchServiceTest {
    private data class Sent(
        val query: String,
        val inputMode: String,
        val toggles: DictionaryToggles,
        val customDictionaryEnabled: Boolean,
        val limit: Int,
    )

    private class FakeLexicon(
        private val rows: List<DictionarySearchResult> = emptyList(),
    ) : LexiconClient {
        val sent = mutableListOf<Sent>()

        override suspend fun dictionarySearch(
            query: String,
            inputMode: String,
            toggles: DictionaryToggles,
            customDictionaryEnabled: Boolean,
            limit: Int,
        ): List<DictionarySearchResult> {
            sent += Sent(query, inputMode, toggles, customDictionaryEnabled, limit)
            return rows
        }
    }

    private fun service(
        settings: StubEngineSettings = StubEngineSettings(),
        lexicon: FakeLexicon,
    ) = DictionarySearchService(lexicon, StubEngineSettingsProvider(settings))

    // CROSS-PLATFORM INVARIANT — mirrors iOS `testAnEmptyQuery_asksTheEngineNothing`.
    @Test
    fun `an empty query asks the engine nothing`() =
        runTest {
            val lexicon = FakeLexicon()

            val results = service(lexicon = lexicon).search("")

            assertTrue(results.isEmpty())
            assertTrue(lexicon.sent.isEmpty())
        }

    // CROSS-PLATFORM INVARIANT — mirrors iOS `testOneRequest_carriesTheSettingsSnapshot`.
    @Test
    fun `one request carries the settings snapshot`() =
        runTest {
            val settings = StubEngineSettings(inputMode = "tps", isCustomDictEnabled = false, isTaiHuaDictEnabled = false)
            val lexicon = FakeLexicon()

            service(settings, lexicon).search("ㄉㄞ")

            assertEquals(
                listOf(Sent("ㄉㄞ", "tps", DictionaryToggles.from(settings), customDictionaryEnabled = false, limit = 20)),
                lexicon.sent,
            )
            assertFalse(
                lexicon.sent
                    .single()
                    .toggles.taihoa,
            )
        }

    // CROSS-PLATFORM INVARIANT — mirrors iOS `testRows_keepTheEngineOrder`:
    // the engine puts custom entries first and owns the dictionary order.
    @Test
    fun `rows keep the engine order`() =
        runTest {
            val rows =
                listOf(
                    DictionarySearchResult("taigi", "taigi", "我的台語", listOf(DictionarySource.CUSTOM)),
                    DictionarySearchResult("tâi-gí", "tâi-gí", "台語", listOf(DictionarySource.KAUTIAN)),
                )

            assertEquals(rows, service(lexicon = FakeLexicon(rows)).search("taigi"))
        }

    // CROSS-PLATFORM INVARIANT — mirrors iOS `testARow_decodesLookupTlAbsentHanjiAndKnownSources`.
    @Test
    fun `a row decodes lookup TL, absent Hanji and known sources`() {
        val custom =
            DictionarySearchRow
                .newBuilder()
                .setCustomId("c1")
                .setRoman("Tâi-gí")
                .setLookupTl("Tâi-gí")
                .addSources(DictionarySourceCode.DICT_SOURCE_CUSTOM)
                .addSources(DictionarySourceCode.DICT_SOURCE_UNSPECIFIED)
                .build()
        assertEquals(
            DictionarySearchResult("Tâi-gí", "Tâi-gí", null, listOf(DictionarySource.CUSTOM)),
            custom.toSearchResult(),
        )

        val system =
            DictionarySearchRow
                .newBuilder()
                .setSystemId(7)
                .setRoman("tâi-gí")
                .setLookupTl("tâi-gí")
                .setHanji("台語")
                .addSources(DictionarySourceCode.DICT_SOURCE_KAUTIAN)
                .addSources(DictionarySourceCode.DICT_SOURCE_TAIGITV)
                .build()
        assertEquals(
            DictionarySearchResult("tâi-gí", "tâi-gí", "台語", listOf(DictionarySource.KAUTIAN, DictionarySource.TAIGITV)),
            system.toSearchResult(),
        )
    }
}
