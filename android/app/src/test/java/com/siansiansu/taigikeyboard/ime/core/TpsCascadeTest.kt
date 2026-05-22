// Unit tests for the pure [TpsCascade] state machine.
// Pure-JVM: no DataStore / Robolectric / Android dependencies.

package com.siansiansu.taigikeyboard.ime.core

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

/**
 * Mirrors iOS `SharedSettingsTests` (PR #319) at the state-machine level.
 * Verifies the cascade write plans produced by [TpsCascade] for the four
 * transition shapes (enter / exit / stable / no-op) on both directions,
 * plus the deliberate input-side ↔ layout-side asymmetry.
 */
class TpsCascadeTest {
    // ------------------------------------------------------------------ //
    // forInputMode — entering TPS
    // ------------------------------------------------------------------ //

    @Test
    fun forInputMode_enterTps_fromPojOnPhahTaigi_writesCascade() {
        val writes =
            TpsCascade.forInputMode(
                newValue = "tps",
                oldValue = "poj",
                currentLayout = "phahTaigi",
                layoutBeforeTps = "phahTaigi", // stale default
            )
        assertEquals("tps", writes[PreferenceKeys.INPUT_MODE])
        assertEquals(
            "saves current layout into LAYOUT_BEFORE_TPS",
            "phahTaigi",
            writes[PreferenceKeys.LAYOUT_BEFORE_TPS],
        )
        assertEquals("flips layout to tps", "tps", writes[PreferenceKeys.KEYBOARD_LAYOUT_TYPE])
        assertEquals(
            "PHAH_TAIGI_LAYOUT_ENABLED → false on TPS entry",
            false,
            writes[PreferenceKeys.PHAH_TAIGI_LAYOUT_ENABLED],
        )
    }

    @Test
    fun forInputMode_enterTps_layoutAlreadyTps_skipsCascade() {
        val writes =
            TpsCascade.forInputMode(
                newValue = "tps",
                oldValue = "poj",
                currentLayout = "tps", // already on TPS layout
                layoutBeforeTps = "qwerty",
            )
        assertEquals("tps", writes[PreferenceKeys.INPUT_MODE])
        assertNull(
            "no LAYOUT_BEFORE_TPS save — layout already TPS",
            writes[PreferenceKeys.LAYOUT_BEFORE_TPS],
        )
        assertNull(writes[PreferenceKeys.KEYBOARD_LAYOUT_TYPE])
        assertNull(writes[PreferenceKeys.PHAH_TAIGI_LAYOUT_ENABLED])
    }

    @Test
    fun forInputMode_enterTps_fromQwerty_savesQwertyAsBefore() {
        val writes =
            TpsCascade.forInputMode(
                newValue = "tps",
                oldValue = "tl",
                currentLayout = "qwerty",
                layoutBeforeTps = "phahTaigi",
            )
        assertEquals("qwerty", writes[PreferenceKeys.LAYOUT_BEFORE_TPS])
        assertEquals("tps", writes[PreferenceKeys.KEYBOARD_LAYOUT_TYPE])
        assertEquals(false, writes[PreferenceKeys.PHAH_TAIGI_LAYOUT_ENABLED])
    }

    // ------------------------------------------------------------------ //
    // forInputMode — leaving TPS (GUARDED restore)
    // ------------------------------------------------------------------ //

    @Test
    fun forInputMode_exitTps_layoutStillTps_restoresLayout() {
        val writes =
            TpsCascade.forInputMode(
                newValue = "poj",
                oldValue = "tps",
                currentLayout = "tps",
                layoutBeforeTps = "phahTaigi",
            )
        assertEquals("poj", writes[PreferenceKeys.INPUT_MODE])
        assertEquals(
            "restores layout from LAYOUT_BEFORE_TPS",
            "phahTaigi",
            writes[PreferenceKeys.KEYBOARD_LAYOUT_TYPE],
        )
        assertEquals(
            "PHAH_TAIGI_LAYOUT_ENABLED tracks restored layout",
            true,
            writes[PreferenceKeys.PHAH_TAIGI_LAYOUT_ENABLED],
        )
    }

    @Test
    fun forInputMode_exitTps_restoreToQwerty_phahTaigiEnabledFalse() {
        val writes =
            TpsCascade.forInputMode(
                newValue = "tl",
                oldValue = "tps",
                currentLayout = "tps",
                layoutBeforeTps = "qwerty",
            )
        assertEquals("qwerty", writes[PreferenceKeys.KEYBOARD_LAYOUT_TYPE])
        assertEquals(false, writes[PreferenceKeys.PHAH_TAIGI_LAYOUT_ENABLED])
    }

    @Test
    fun forInputMode_exitTps_layoutAlreadyChanged_guardSkipsRestore() {
        // GUARD: manual layout change earlier in the same flow is preserved.
        val writes =
            TpsCascade.forInputMode(
                newValue = "poj",
                oldValue = "tps",
                currentLayout = "qwerty", // user manually moved off TPS
                layoutBeforeTps = "phahTaigi",
            )
        assertEquals("poj", writes[PreferenceKeys.INPUT_MODE])
        assertNull(
            "GUARDED — no layout restore because layout already left TPS",
            writes[PreferenceKeys.KEYBOARD_LAYOUT_TYPE],
        )
        assertNull(writes[PreferenceKeys.PHAH_TAIGI_LAYOUT_ENABLED])
        assertNull(writes[PreferenceKeys.LAYOUT_BEFORE_TPS])
    }

    // ------------------------------------------------------------------ //
    // forInputMode — stable / non-TPS transitions
    // ------------------------------------------------------------------ //

    @Test
    fun forInputMode_pojToTl_noTpsActivity_onlyInputModeWritten() {
        val writes =
            TpsCascade.forInputMode(
                newValue = "tl",
                oldValue = "poj",
                currentLayout = "phahTaigi",
                layoutBeforeTps = "phahTaigi",
            )
        assertEquals("tl", writes[PreferenceKeys.INPUT_MODE])
        assertNull(writes[PreferenceKeys.LAYOUT_BEFORE_TPS])
        assertNull(writes[PreferenceKeys.KEYBOARD_LAYOUT_TYPE])
        assertNull(writes[PreferenceKeys.PHAH_TAIGI_LAYOUT_ENABLED])
    }

    @Test
    fun forInputMode_idempotentTpsToTps_noLayoutCascade() {
        // Same-value writes: TPS → TPS does not fire the entry branch
        // (oldValue == newValue), so no cascade.
        val writes =
            TpsCascade.forInputMode(
                newValue = "tps",
                oldValue = "tps",
                currentLayout = "qwerty", // sanity: irrelevant, no cascade
                layoutBeforeTps = "phahTaigi",
            )
        assertEquals("tps", writes[PreferenceKeys.INPUT_MODE])
        assertNull(
            "idempotent TPS write — no LAYOUT_BEFORE_TPS save fires",
            writes[PreferenceKeys.LAYOUT_BEFORE_TPS],
        )
        assertNull(writes[PreferenceKeys.KEYBOARD_LAYOUT_TYPE])
        assertNull(writes[PreferenceKeys.PHAH_TAIGI_LAYOUT_ENABLED])
    }

    // ------------------------------------------------------------------ //
    // forKeyboardLayoutType — entering TPS
    // ------------------------------------------------------------------ //

    @Test
    fun forKeyboardLayoutType_enterTps_fromPhahTaigi_writesCascade() {
        val writes =
            TpsCascade.forKeyboardLayoutType(
                newValue = "tps",
                oldValue = "phahTaigi",
                currentInputMode = "tl",
                inputModeBeforeTps = "tl", // stale default
            )
        assertEquals("tps", writes[PreferenceKeys.KEYBOARD_LAYOUT_TYPE])
        assertEquals(
            "PHAH_TAIGI_LAYOUT_ENABLED → false on TPS layout entry",
            false,
            writes[PreferenceKeys.PHAH_TAIGI_LAYOUT_ENABLED],
        )
        assertEquals(
            "saves current inputMode into INPUT_MODE_BEFORE_TPS",
            "tl",
            writes[PreferenceKeys.INPUT_MODE_BEFORE_TPS],
        )
        assertEquals("forces INPUT_MODE = tps", "tps", writes[PreferenceKeys.INPUT_MODE])
    }

    @Test
    fun forKeyboardLayoutType_enterTps_inputModeAlreadyTps_noInputModeBeforeSave() {
        val writes =
            TpsCascade.forKeyboardLayoutType(
                newValue = "tps",
                oldValue = "phahTaigi",
                currentInputMode = "tps", // already TPS
                inputModeBeforeTps = "poj",
            )
        assertEquals("tps", writes[PreferenceKeys.KEYBOARD_LAYOUT_TYPE])
        assertEquals(false, writes[PreferenceKeys.PHAH_TAIGI_LAYOUT_ENABLED])
        assertNull(
            "no INPUT_MODE_BEFORE_TPS save when inputMode already TPS",
            writes[PreferenceKeys.INPUT_MODE_BEFORE_TPS],
        )
        // Still rewrites INPUT_MODE = "tps" (idempotent — matches HEAD~1).
        assertEquals("tps", writes[PreferenceKeys.INPUT_MODE])
    }

    // ------------------------------------------------------------------ //
    // forKeyboardLayoutType — leaving TPS (UNCONDITIONAL restore)
    // ------------------------------------------------------------------ //

    @Test
    fun forKeyboardLayoutType_exitTps_inputModeStillTps_restoresInputMode() {
        val writes =
            TpsCascade.forKeyboardLayoutType(
                newValue = "phahTaigi",
                oldValue = "tps",
                currentInputMode = "tps",
                inputModeBeforeTps = "poj",
            )
        assertEquals("phahTaigi", writes[PreferenceKeys.KEYBOARD_LAYOUT_TYPE])
        assertEquals(true, writes[PreferenceKeys.PHAH_TAIGI_LAYOUT_ENABLED])
        assertEquals(
            "restores INPUT_MODE from INPUT_MODE_BEFORE_TPS",
            "poj",
            writes[PreferenceKeys.INPUT_MODE],
        )
    }

    @Test
    fun forKeyboardLayoutType_exitTps_inputModeAlreadyChanged_unconditionalRestoreOverwrites() {
        // ASYMMETRIC: layout-side exit is UNCONDITIONAL — it restores
        // inputMode from INPUT_MODE_BEFORE_TPS even if the live inputMode
        // is no longer "tps". Mirrors iOS PR-1 + HEAD~1 byte-for-byte.
        val writes =
            TpsCascade.forKeyboardLayoutType(
                newValue = "qwerty",
                oldValue = "tps",
                currentInputMode = "poj", // user manually moved off TPS
                inputModeBeforeTps = "tl",
            )
        assertEquals("qwerty", writes[PreferenceKeys.KEYBOARD_LAYOUT_TYPE])
        assertEquals(false, writes[PreferenceKeys.PHAH_TAIGI_LAYOUT_ENABLED])
        assertEquals(
            "UNCONDITIONAL — restore fires despite live inputMode = poj",
            "tl",
            writes[PreferenceKeys.INPUT_MODE],
        )
    }

    // ------------------------------------------------------------------ //
    // forKeyboardLayoutType — non-TPS transitions
    // ------------------------------------------------------------------ //

    @Test
    fun forKeyboardLayoutType_phahTaigiToQwerty_noCascade() {
        val writes =
            TpsCascade.forKeyboardLayoutType(
                newValue = "qwerty",
                oldValue = "phahTaigi",
                currentInputMode = "tl",
                inputModeBeforeTps = "tl",
            )
        assertEquals("qwerty", writes[PreferenceKeys.KEYBOARD_LAYOUT_TYPE])
        assertEquals(false, writes[PreferenceKeys.PHAH_TAIGI_LAYOUT_ENABLED])
        assertNull(writes[PreferenceKeys.INPUT_MODE])
        assertNull(writes[PreferenceKeys.INPUT_MODE_BEFORE_TPS])
    }

    @Test
    fun forKeyboardLayoutType_idempotentTpsToTps_noCascade() {
        val writes =
            TpsCascade.forKeyboardLayoutType(
                newValue = "tps",
                oldValue = "tps",
                currentInputMode = "tl",
                inputModeBeforeTps = "tl",
            )
        assertEquals("tps", writes[PreferenceKeys.KEYBOARD_LAYOUT_TYPE])
        assertEquals(false, writes[PreferenceKeys.PHAH_TAIGI_LAYOUT_ENABLED])
        assertNull("idempotent TPS layout write — no inputMode cascade", writes[PreferenceKeys.INPUT_MODE])
        assertNull(writes[PreferenceKeys.INPUT_MODE_BEFORE_TPS])
    }

    // ------------------------------------------------------------------ //
    // PHAH_TAIGI_LAYOUT_ENABLED tracking — paired field invariant
    // ------------------------------------------------------------------ //

    @Test
    fun forKeyboardLayoutType_phahTaigiEnabledTrue_onlyWhenLayoutIsPhahTaigi() {
        val layouts = listOf("phahTaigi", "qwerty", "moe1", "moe2", "tps")
        for (layout in layouts) {
            val writes =
                TpsCascade.forKeyboardLayoutType(
                    newValue = layout,
                    oldValue = "phahTaigi",
                    currentInputMode = "tl",
                    inputModeBeforeTps = "tl",
                )
            assertEquals(
                "PHAH_TAIGI_LAYOUT_ENABLED tracks (newValue == \"phahTaigi\") — layout=$layout",
                layout == "phahTaigi",
                writes[PreferenceKeys.PHAH_TAIGI_LAYOUT_ENABLED],
            )
        }
    }
}
