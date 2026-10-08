// Pure TPS sync state machine — computes the DataStore write batch for inputMode ↔ keyboardLayoutType cascade.

package com.siansiansu.taigikeyboard.ime.settings

import androidx.datastore.preferences.core.Preferences

/**
 * Computes the [Preferences.Key] → value map that [PrefHelper.applyInputMode] and
 * [PrefHelper.applyKeyboardLayoutType] commit in a single DataStore transaction.
 *
 * Pure: takes raw current-state inputs, returns the write plan. No DataStore /
 * cache touched. Mirrors iOS `SharedSettings.setInputMode(_:)` /
 * `SharedSettings.setKeyboardLayoutType(_:)` state machine introduced in iOS
 * PR-1 (#319, commit 1cd6cbfd).
 *
 * ### Asymmetry — deliberate, preserved from HEAD~1 + iOS PR-1
 * - **Input-side exit** (leaving TPS via inputMode) GUARDS on
 *   `currentLayout == "tps"` before restoring layout. A manual layout change
 *   earlier in the same flow is preserved.
 * - **Layout-side exit** (leaving TPS via keyboardLayoutType) UNCONDITIONALLY
 *   restores `inputMode` from `inputModeBeforeTps` — even if `inputMode` has
 *   already changed via another path.
 *
 * Symmetrization belongs in a separate slice; do NOT collapse the two sides.
 */
internal object TpsCascade {
    /**
     * Computes the write plan for [PrefHelper.applyInputMode]. Includes the
     * `INPUT_MODE` write itself plus, on TPS entry/exit, the paired
     * `LAYOUT_BEFORE_TPS` save and `KEYBOARD_LAYOUT_TYPE` cascade write.
     */
    fun forInputMode(
        newValue: String,
        oldValue: String,
        currentLayout: String,
        layoutBeforeTps: String,
    ): Map<Preferences.Key<*>, Any> =
        buildMap {
            if (newValue == "tps" && oldValue != "tps") {
                // Entry: only cascade if the layout is not already on TPS — preserves
                // user's prior layout in `LAYOUT_BEFORE_TPS` for later restore.
                if (currentLayout != "tps") {
                    put(PreferenceKeys.LAYOUT_BEFORE_TPS, currentLayout)
                    put(PreferenceKeys.KEYBOARD_LAYOUT_TYPE, "tps")
                }
            } else if (newValue != "tps" && oldValue == "tps") {
                // Exit: guarded restore — only restore layout if the live layout is
                // still TPS. A manual layout change earlier in the same flow is preserved.
                if (currentLayout == "tps") {
                    put(PreferenceKeys.KEYBOARD_LAYOUT_TYPE, layoutBeforeTps)
                }
            }
            put(PreferenceKeys.INPUT_MODE, newValue)
        }

    /**
     * Computes the write plan for [PrefHelper.applyKeyboardLayoutType]: the
     * `KEYBOARD_LAYOUT_TYPE` write plus, on TPS entry/exit, the
     * `INPUT_MODE_BEFORE_TPS` save and `INPUT_MODE` cascade write.
     *
     * Exit branch UNCONDITIONALLY restores `INPUT_MODE` from
     * `inputModeBeforeTps` — see [TpsCascade] KDoc for asymmetry rationale.
     */
    fun forKeyboardLayoutType(
        newValue: String,
        oldValue: String,
        currentInputMode: String,
        inputModeBeforeTps: String,
    ): Map<Preferences.Key<*>, Any> =
        buildMap {
            if (newValue == "tps" && oldValue != "tps") {
                // Entry: save the current inputMode unless it is already TPS, then
                // force inputMode = "tps".
                if (currentInputMode != "tps") {
                    put(PreferenceKeys.INPUT_MODE_BEFORE_TPS, currentInputMode)
                }
                put(PreferenceKeys.INPUT_MODE, "tps")
            } else if (newValue != "tps" && oldValue == "tps") {
                // Exit: unconditional restore — mirrors HEAD~1 + iOS PR-1 deliberate asymmetry.
                put(PreferenceKeys.INPUT_MODE, inputModeAfterRomanizationLayout(oldValue, currentInputMode, inputModeBeforeTps))
            }
            put(PreferenceKeys.KEYBOARD_LAYOUT_TYPE, newValue)
        }

    /**
     * The input mode after writing any non-TPS layout: the backup when leaving TPS (the value
     * [forKeyboardLayoutType] restores, unconditionally), else the live mode.
     */
    fun inputModeAfterRomanizationLayout(
        oldLayout: String,
        currentInputMode: String,
        inputModeBeforeTps: String,
    ): String = if (oldLayout == "tps") inputModeBeforeTps else currentInputMode

    /**
     * Computes ONE write plan for a layout write followed by an input-mode write (a Layout-tab
     * Tâi-lô / Pe̍h-ōe-jī card). Two setters would launch two DataStore commits in no fixed
     * order, so leaving TPS the layout cascade's restored mode could land after the card's mode.
     * Runs [forKeyboardLayoutType], then [forInputMode] against the post-layout state; later
     * writes win — the same end state as iOS `keyboardLayoutType = …; inputMode = …`.
     */
    fun forLayoutAndInputMode(
        newLayout: String,
        newInputMode: String,
        oldLayout: String,
        oldInputMode: String,
        inputModeBeforeTps: String,
        layoutBeforeTps: String,
    ): Map<Preferences.Key<*>, Any> {
        val layoutWrites = forKeyboardLayoutType(newLayout, oldLayout, oldInputMode, inputModeBeforeTps)
        val modeAfterLayout = layoutWrites[PreferenceKeys.INPUT_MODE] as? String ?: oldInputMode
        return layoutWrites + forInputMode(newInputMode, modeAfterLayout, newLayout, layoutBeforeTps)
    }
}
