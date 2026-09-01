package com.siansiansu.taigikeyboard.ime.core.settings

import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * Pin for `docs/architecture/behavioral-invariants.md` §11 — engine
 * settings MUST be live-read. Each property access on an
 * [EngineSettingsProvider]-returned value re-reads the underlying store.
 *
 * Uses an in-memory [MutableEngineSettingsProvider] (test-only) rather
 * than `PrefHelper` because `PrefHelper` hard-wires the DataStore
 * delegate to a real Android `Context`, which pure-JVM tests cannot
 * construct. The invariant is about engine read semantics — not
 * DataStore correctness — so a fake provider is the right harness.
 */
class EngineSettingsLiveReadTest {
    /**
     * A fresh `.current` view reflects the underlying store state at
     * the time of the call — NOT the state captured at provider
     * construction. Mutating the backing source between calls changes
     * what engine code sees without rebuilding the provider.
     */
    @Test
    fun test_INVARIANT_engine_settings_are_live_read() {
        val backing = MutableBacking(inputMode = "poj")
        val settings = MutableEngineSettings(backing)
        val provider = MutableEngineSettingsProvider(settings)

        // Initial read reflects the constructor value.
        assertEquals("initial inputMode", "poj", provider.current.inputMode)

        // Mutate the backing source AND re-read through the same
        // provider instance. Live-read contract says the new value is
        // visible without re-creating `provider`.
        backing.inputMode = "tl"
        assertEquals("inputMode reflects live mutation", "tl", provider.current.inputMode)

        backing.isAutoCap = true
        assertEquals("isAutoCap reflects live mutation", true, provider.current.isAutoCap)

        backing.isCustomDictEnabled = true
        assertEquals(
            "isCustomDictEnabled reflects live mutation",
            true,
            provider.current.isCustomDictEnabled,
        )

        // Repeated reads without mutation return the same value — the
        // reads are re-issued, not memoized at the engine boundary.
        backing.inputMode = "tps"
        assertEquals("tps", provider.current.inputMode)
        assertEquals("tps", provider.current.inputMode)

        // Dictionary toggles also honor the live-read contract.
        backing.isMoeDictEnabled = true
        assertEquals(true, provider.current.isMoeDictEnabled)
        backing.isMoeDictEnabled = false
        assertEquals(false, provider.current.isMoeDictEnabled)

        // kautian subcollection toggles honor the same live-read contract.
        backing.isKautianAccentLukangEnabled = true
        assertEquals(true, provider.current.isKautianAccentLukangEnabled)
        backing.isKautianAccentLukangEnabled = false
        assertEquals(false, provider.current.isKautianAccentLukangEnabled)
        backing.isKautianNameAppendixEnabled = true
        assertEquals(true, provider.current.isKautianNameAppendixEnabled)
    }

    /**
     * Plain backing bag of mutable fields — stands in for whatever the
     * production provider reads (`PrefHelper.cachedPrefs`, `UserDefaults`,
     * etc.).
     */
    private class MutableBacking(
        var inputMode: String = "tl",
        var isAutoCap: Boolean = false,
        var candidateDisplayMode: CandidateDisplayMode = CandidateDisplayMode.SIDE_BY_SIDE,
        var isTranslateSwapped: Boolean = false,
        var isOutputBothScripts: Boolean = false,
        var isLiteralRomanCandidateEnabled: Boolean = true,
        var isAssociationRecordingEnabled: Boolean = false,
        var toneToggles: ToneToggles =
            ToneToggles(isDoubleTapOOEnabled = false, isDoubleTapNNEnabled = false),
        var isCustomDictEnabled: Boolean = false,
        var isTpsOrMappedToER: Boolean = false,
        var isMoeDictEnabled: Boolean = false,
        var isNewwordDictEnabled: Boolean = false,
        var isKunggeDictEnabled: Boolean = false,
        var isITaigiDictEnabled: Boolean = false,
        var isTaiwanJapanDictEnabled: Boolean = false,
        var isTaiHuaDictEnabled: Boolean = false,
        var isTaiwanPlantDictEnabled: Boolean = false,
        var isSttiDictEnabled: Boolean = false,
        var isKhpooDictEnabled: Boolean = false,
        var isVariantEnabled: Boolean = false,
        var isKhiinEnabled: Boolean = false,
        var isLkkDictEnabled: Boolean = false,
        var isDevDictEnabled: Boolean = false,
        var isKautianAccentLukangEnabled: Boolean = false,
        var isKautianAccentSansiaEnabled: Boolean = false,
        var isKautianAccentTaipakEnabled: Boolean = false,
        var isKautianAccentGilanEnabled: Boolean = false,
        var isKautianAccentTainanEnabled: Boolean = false,
        var isKautianAccentKaohsiungEnabled: Boolean = false,
        var isKautianAccentKinmenEnabled: Boolean = false,
        var isKautianAccentMakungEnabled: Boolean = false,
        var isKautianAccentSintikEnabled: Boolean = false,
        var isKautianAccentTaichungEnabled: Boolean = false,
        var isKautianNameAppendixEnabled: Boolean = false,
    )

    /**
     * [EngineSettings] implementation whose `get()` properties delegate to
     * a mutable backing bag — each access reads the latest value. Matches
     * the contract `PrefHelper` honors in production.
     */
    private class MutableEngineSettings(
        private val backing: MutableBacking,
    ) : EngineSettings {
        override val inputMode: String get() = backing.inputMode
        override val isAutoCap: Boolean get() = backing.isAutoCap
        override val candidateDisplayMode: CandidateDisplayMode get() = backing.candidateDisplayMode
        override val isTranslateSwapped: Boolean get() = backing.isTranslateSwapped
        override val isOutputBothScripts: Boolean get() = backing.isOutputBothScripts
        override val isLiteralRomanCandidateEnabled: Boolean get() = backing.isLiteralRomanCandidateEnabled
        override val isAssociationRecordingEnabled: Boolean get() = backing.isAssociationRecordingEnabled
        override val toneToggles: ToneToggles get() = backing.toneToggles
        override val isCustomDictEnabled: Boolean get() = backing.isCustomDictEnabled
        override val isTpsOrMappedToER: Boolean get() = backing.isTpsOrMappedToER
        override val isMoeDictEnabled: Boolean get() = backing.isMoeDictEnabled
        override val isNewwordDictEnabled: Boolean get() = backing.isNewwordDictEnabled
        override val isKunggeDictEnabled: Boolean get() = backing.isKunggeDictEnabled
        override val isITaigiDictEnabled: Boolean get() = backing.isITaigiDictEnabled
        override val isTaiwanJapanDictEnabled: Boolean get() = backing.isTaiwanJapanDictEnabled
        override val isTaiHuaDictEnabled: Boolean get() = backing.isTaiHuaDictEnabled
        override val isTaiwanPlantDictEnabled: Boolean get() = backing.isTaiwanPlantDictEnabled
        override val isSttiDictEnabled: Boolean get() = backing.isSttiDictEnabled
        override val isKhpooDictEnabled: Boolean get() = backing.isKhpooDictEnabled
        override val isVariantEnabled: Boolean get() = backing.isVariantEnabled
        override val isKhiinEnabled: Boolean get() = backing.isKhiinEnabled
        override val isLkkDictEnabled: Boolean get() = backing.isLkkDictEnabled
        override val isDevDictEnabled: Boolean get() = backing.isDevDictEnabled
        override val isKautianAccentLukangEnabled: Boolean get() = backing.isKautianAccentLukangEnabled
        override val isKautianAccentSansiaEnabled: Boolean get() = backing.isKautianAccentSansiaEnabled
        override val isKautianAccentTaipakEnabled: Boolean get() = backing.isKautianAccentTaipakEnabled
        override val isKautianAccentGilanEnabled: Boolean get() = backing.isKautianAccentGilanEnabled
        override val isKautianAccentTainanEnabled: Boolean get() = backing.isKautianAccentTainanEnabled
        override val isKautianAccentKaohsiungEnabled: Boolean get() = backing.isKautianAccentKaohsiungEnabled
        override val isKautianAccentKinmenEnabled: Boolean get() = backing.isKautianAccentKinmenEnabled
        override val isKautianAccentMakungEnabled: Boolean get() = backing.isKautianAccentMakungEnabled
        override val isKautianAccentSintikEnabled: Boolean get() = backing.isKautianAccentSintikEnabled
        override val isKautianAccentTaichungEnabled: Boolean get() = backing.isKautianAccentTaichungEnabled
        override val isKautianNameAppendixEnabled: Boolean get() = backing.isKautianNameAppendixEnabled
    }

    /**
     * Provider whose `current` returns the same [MutableEngineSettings]
     * instance — minimal stand-in for the `PrefHelper`-backed live provider
     * in production. Matches the `current = this` (property, not snapshot)
     * pattern used by `PrefHelper`.
     */
    private class MutableEngineSettingsProvider(
        private val settings: MutableEngineSettings,
    ) : EngineSettingsProvider {
        override val current: EngineSettings get() = settings
    }
}
