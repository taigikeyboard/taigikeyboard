package com.siansiansu.taigikeyboard.engine

import com.siansiansu.taigikeyboard.ime.settings.CandidateDisplayMode
import com.siansiansu.taigikeyboard.ime.settings.PojMarkerOptions
import com.siansiansu.taigikeyboard.ime.settings.StubEngineSettings
import com.siansiansu.taigikeyboard.ime.settings.SyllableSeparator
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import com.siansiansu.taigikeyboard.engine.proto.CandidateDisplayMode as ProtoCandidateDisplayMode
import com.siansiansu.taigikeyboard.engine.proto.SyllableSeparator as ProtoSyllableSeparator

/**
 * The wire config every request carries: what [appConfig] and the composing
 * projection [continuousAppConfig] put on each field. Mirrors iOS
 * `RustEngineBridgeAppConfigTests`; the builder only builds a proto, so no
 * engine is needed.
 */
class EngineAppConfigTest {
    // The TPS layout goes out as "tps" with the swap and Syllable Separator as stored — the engine
    // applies the TPS fold itself (`AppConfig::renders_hanji_first` / `rendered_syllable_joiner`).
    @Test
    fun continuousAppConfig_tpsLayout_sendsTpsWithTheStoredFlags() {
        val settings = StubEngineSettings(
            inputMode = "tps",
            isHanjiFirst = false,
            syllableSeparator = SyllableSeparator.NONE,
            isTpsOrMappedToER = true,
        )

        val config = continuousAppConfig(settings)

        assertEquals("tps", config.inputMode)
        assertFalse("the swap goes out unfolded; the engine reads tps as Hanji-first", config.isHanjiFirst)
        assertEquals(
            "the separator goes out as stored; the engine exempts tps",
            ProtoSyllableSeparator.SYLLABLE_SEPARATOR_NONE,
            config.syllableSeparator,
        )
        assertTrue("the or→er dialect switch rides the config", config.tpsOrMapsToEr)
    }

    @Test
    fun continuousAppConfig_romanizationModes_forwardEverySetting() {
        for (mode in listOf("tl", "poj", "english")) {
            val settings = StubEngineSettings(
                inputMode = mode,
                candidateDisplayMode = CandidateDisplayMode.COMBINED,
                isHanjiFirst = true,
                isOutputBothScripts = true,
                syllableSeparator = SyllableSeparator.SPACE,
                pojMarkerOptions = PojMarkerOptions(
                    isDoubleTapOOEnabled = true,
                    isDoubleTapNNEnabled = true,
                    isNasalMarkerUppercaseEnabled = false,
                ),
            )

            val config = continuousAppConfig(settings)

            assertEquals(mode, config.inputMode)
            assertTrue(mode, config.isHanjiFirst)
            assertTrue(mode, config.outputBothScripts)
            assertEquals(mode, ProtoCandidateDisplayMode.CANDIDATE_DISPLAY_MODE_COMBINED, config.candidateDisplayMode)
            assertEquals(mode, ProtoSyllableSeparator.SYLLABLE_SEPARATOR_SPACE, config.syllableSeparator)
            assertTrue(mode, config.ooDoubletapEnabled)
            assertTrue(mode, config.nnDoubletapEnabled)
            assertTrue("$mode: ⁿ becomes ᴺ OFF is inverted on the wire", config.forceLowercaseNasalMarker)
            assertFalse(mode, config.tpsOrMapsToEr)
        }
    }

    // Nextword and case transform pass only what they read; the rest keeps the proto defaults.
    @Test
    fun appConfig_unpassedFields_keepTheProtoDefaults() {
        val config = appConfig("tps", isHanjiFirst = true)

        assertEquals("tps", config.inputMode)
        assertTrue(config.isHanjiFirst)
        assertFalse(config.ooDoubletapEnabled)
        assertFalse(config.nnDoubletapEnabled)
        assertFalse(config.forceLowercaseNasalMarker)
        assertFalse(config.outputBothScripts)
        assertEquals(ProtoCandidateDisplayMode.CANDIDATE_DISPLAY_MODE_SIDE_BY_SIDE, config.candidateDisplayMode)
        assertEquals(ProtoSyllableSeparator.SYLLABLE_SEPARATOR_HYPHEN, config.syllableSeparator)
        assertFalse(config.tpsOrMapsToEr)
    }

    // An unknown stored mode keeps each path's historical reading: composing TL, nextword POJ.
    @Test
    fun engineInputMode_passesTheFourModes_andMapsAnyOtherValueToTheFallback() {
        for (mode in listOf("tl", "poj", "tps", "english")) {
            assertEquals(mode, engineInputMode(mode, unknownAs = "poj"))
        }
        assertEquals("tl", engineInputMode("klingon", unknownAs = "tl"))
        assertEquals("poj", engineInputMode("", unknownAs = "poj"))
        assertEquals("tl", continuousAppConfig(StubEngineSettings(inputMode = "klingon")).inputMode)
    }
}
