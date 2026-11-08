package com.siansiansu.taigikeyboard.engine

import com.siansiansu.taigikeyboard.ime.settings.StubEngineSettings
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * The dictionary toggles `continuousAppConfig` puts in `AppConfig.dictionarySources`
 * (and `dictionaryFilters` / `nextwordPredictNext` send): the engine resolves them into
 * its source filter, so every dictionary off offers no dictionary candidates
 * (`behavioral-invariants.md` §57). JVM tests cannot load the engine; the builder
 * only builds a proto. Engine side: `golden_fetch_at_pos.rs`
 * `fetch_at_pos_filters_by_the_config_dictionary_sources`.
 */
class DictionaryTogglesProtoTest {
    // INVARIANT_DICTIONARIES_ALL_OFF_OFFERS_NO_DICTIONARY_CANDIDATES (behavioral-invariants.md §57)
    @Test
    fun everyDictionaryOff_goesOutAsEveryToggleOff() {
        val settings = StubEngineSettings(
            isMoeDictEnabled = false,
            isNewwordDictEnabled = false,
            isITaigiDictEnabled = false,
            isTaiwanPlantDictEnabled = false,
            isTaiHuaDictEnabled = false,
            isTaiwanJapanDictEnabled = false,
            isKunggeDictEnabled = false,
            isSttiDictEnabled = false,
            isKhpooDictEnabled = false,
            isVariantEnabled = false,
            isKhiinEnabled = false,
            isLkkDictEnabled = false,
            isDevDictEnabled = false,
        )

        val proto = dictionaryTogglesProto(RustEngineBridge.DictionaryToggles.from(settings))

        val sources = listOf(
            proto.kautian,
            proto.taigitv,
            proto.itaigi,
            proto.sitbut,
            proto.taihoa,
            proto.taijit,
            proto.kungge,
            proto.stti,
            proto.khpoo,
            proto.variant,
            proto.khiin,
            proto.lkk,
            proto.dev,
        )
        assertEquals("every source toggle off", List(sources.size) { false }, sources)
    }

    @Test
    fun kautianSubcollections_alwaysRideAlong() {
        val settings = StubEngineSettings(isKautianAccentGilanEnabled = false)

        val proto = dictionaryTogglesProto(RustEngineBridge.DictionaryToggles.from(settings))

        assertTrue("an absent message would switch the subcollection gate off", proto.hasKautianSubcollections())
        assertFalse(proto.kautianSubcollections.accentGilan)
        assertTrue(proto.kautianSubcollections.accentLukang)
        assertTrue(proto.kautian)
    }
}
