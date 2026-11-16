package com.siansiansu.taigikeyboard.ime.settings

import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * Pin for `docs/architecture/behavioral-invariants.md` §11 — engine
 * settings MUST be live-read. Each property access on an
 * [EngineSettingsProvider]-returned value re-reads the underlying store.
 *
 * Uses the in-memory [StubEngineSettingsProvider] (test-only) rather
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
        val backing =
            StubEngineSettings(
                inputMode = "poj",
                isCustomDictEnabled = false,
                isMoeDictEnabled = false,
                isKautianAccentLukangEnabled = false,
                isKautianNameAppendixEnabled = false,
            )
        val provider = StubEngineSettingsProvider(backing)

        // Initial read reflects the constructor value.
        assertEquals("initial inputMode", "poj", provider.current.inputMode)

        // Mutate the backing source AND re-read through the same
        // provider instance. Live-read contract says the new value is
        // visible without re-creating `provider`.
        backing.inputMode = "tl"
        assertEquals("inputMode reflects live mutation", "tl", provider.current.inputMode)

        backing.isCustomDictEnabled = true
        assertEquals(
            "isCustomDictEnabled reflects live mutation",
            true,
            provider.current.isCustomDictEnabled,
        )

        backing.isKhinsiannInHanjiEnabled = true
        assertEquals(
            "isKhinsiannInHanjiEnabled reflects live mutation",
            true,
            provider.current.isKhinsiannInHanjiEnabled,
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
}
