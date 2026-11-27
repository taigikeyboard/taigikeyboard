// Pure custom-layout selection — computes the DataStore write batch for applying, saving and
// deleting a custom layout, and for choosing a built-in one.

package com.siansiansu.taigikeyboard.ime.settings

import androidx.datastore.preferences.core.Preferences
import com.siansiansu.taigikeyboard.ime.text.layout.CustomLayout

/**
 * Selection = `KEYBOARD_LAYOUT_TYPE` (the base) + `SELECTED_CUSTOM_LAYOUT_ID` (a romanization layout) or,
 * while the layout is TPS, `TPS_CUSTOM_LAYOUT_ID`. Choosing a built-in layout clears the id, even on the
 * same base (inside TPS the TPS card clears only the TPS slot); the input-mode TPS cascade
 * ([TpsCascade.forInputMode]) leaves both, so leaving TPS resumes the romanization layout. Pure: takes
 * the current state, returns the plan [PrefHelper] commits in one transaction. Mirrors iOS
 * `SharedSettings` § Custom layouts.
 */
internal object CustomLayoutSelection {
    /**
     * [layoutWrites] (a built-in layout's cascade, leaving [oldLayout]) plus clearing the custom
     * selection: entering or staying in TPS ends a TPS one; anything but staying in TPS ends a
     * romanization one (inside TPS it is suspended and resumes on leaving).
     */
    fun forBuiltIn(
        layoutWrites: Map<Preferences.Key<*>, Any>,
        oldLayout: String,
    ): Map<Preferences.Key<*>, Any> =
        buildMap {
            putAll(layoutWrites)
            val newLayout = layoutWrites[PreferenceKeys.KEYBOARD_LAYOUT_TYPE] as? String ?: oldLayout
            val entersTps = newLayout == CustomLayout.TPS_BASE
            val staysInTps = entersTps && oldLayout == CustomLayout.TPS_BASE
            if (entersTps) put(PreferenceKeys.TPS_CUSTOM_LAYOUT_ID, "")
            if (!staysInTps) put(PreferenceKeys.SELECTED_CUSTOM_LAYOUT_ID, "")
        }

    /**
     * [modeWrites] (an input-mode write plan, or empty), then [layout] through the TPS-aware layout
     * cascade against the state they leave, in one plan; later writes win. Mode first, so [layout]
     * lands in the entered script's memory ([LayoutScriptMemory]). Mirrors iOS `LayoutChoice.apply(to:)`.
     */
    fun forLayoutAfterMode(
        layout: String,
        modeWrites: Map<Preferences.Key<*>, Any>,
        currentLayout: String,
        currentInputMode: String,
        inputModeBeforeTps: String,
    ): Map<Preferences.Key<*>, Any> {
        val layoutNow = modeWrites[PreferenceKeys.KEYBOARD_LAYOUT_TYPE] as? String ?: currentLayout
        val modeNow = modeWrites[PreferenceKeys.INPUT_MODE] as? String ?: currentInputMode
        return modeWrites + TpsCascade.forKeyboardLayoutType(layout, layoutNow, modeNow, inputModeBeforeTps)
    }

    /**
     * The input mode applying [layout] enters: TPS for a TPS layout, its script for a Tâi-lô /
     * Pe̍h-ōe-jī one; null for a universal one, which keeps the mode.
     */
    fun inputModeToApply(layout: CustomLayout): String? =
        when {
            layout.isTps -> CustomLayout.TPS_BASE
            layout.isUniversal -> null
            else -> layout.script.inputMode
        }

    /**
     * Selects [layout]. A TPS one: the mode to leave to saved (`INPUT_MODE_BEFORE_TPS`, unless already
     * TPS), [modeWrites] (entering TPS as the input mode does, so the romanization selection stays),
     * then the TPS slot. Otherwise [modeWrites] (entering its script, or empty) and its base
     * ([forLayoutAfterMode]), then its id.
     */
    fun forApply(
        layout: CustomLayout,
        modeWrites: Map<Preferences.Key<*>, Any>,
        currentLayout: String,
        currentInputMode: String,
        inputModeBeforeTps: String,
    ): Map<Preferences.Key<*>, Any> =
        buildMap {
            if (layout.isTps) {
                if (currentInputMode != CustomLayout.TPS_BASE) put(PreferenceKeys.INPUT_MODE_BEFORE_TPS, currentInputMode)
                putAll(modeWrites)
                put(PreferenceKeys.TPS_CUSTOM_LAYOUT_ID, layout.id)
            } else {
                putAll(forLayoutAfterMode(layout.base, modeWrites, currentLayout, currentInputMode, inputModeBeforeTps))
                put(PreferenceKeys.SELECTED_CUSTOM_LAYOUT_ID, layout.id)
            }
        }

    /**
     * Saves [layout] into [layouts] (replacing its saved version) and applies it; null when a new
     * layout would pass [CustomLayout.MAX_CUSTOM_LAYOUTS].
     */
    fun forSave(
        layouts: List<CustomLayout>,
        layout: CustomLayout,
        modeWrites: Map<Preferences.Key<*>, Any>,
        currentLayout: String,
        currentInputMode: String,
        inputModeBeforeTps: String,
    ): Map<Preferences.Key<*>, Any>? {
        val saved = CustomLayout.upserted(layouts, layout) ?: return null
        return buildMap {
            putAll(forApply(layout, modeWrites, currentLayout, currentInputMode, inputModeBeforeTps))
            put(PreferenceKeys.USER_LAYOUTS, CustomLayout.encodeList(saved))
        }
    }

    /**
     * Saves a copy of [layout] under a new id and [name] right after it, leaving the selection
     * alone; null when it would pass [CustomLayout.MAX_CUSTOM_LAYOUTS].
     */
    fun forDuplicate(
        layouts: List<CustomLayout>,
        layout: CustomLayout,
        name: String,
    ): Map<Preferences.Key<*>, Any>? {
        val saved = CustomLayout.upserted(layouts, layout.duplicated(name), anchorId = layout.id) ?: return null
        return buildMap { put(PreferenceKeys.USER_LAYOUTS, CustomLayout.encodeList(saved)) }
    }

    /**
     * Deletes the layout with [id]; deleting a selected one ([selectedId], or the TPS slot's [tpsId])
     * clears its slot and leaves its base selected as a built-in layout.
     */
    fun forDelete(
        layouts: List<CustomLayout>,
        id: String,
        selectedId: String,
        tpsId: String,
    ): Map<Preferences.Key<*>, Any> =
        buildMap {
            put(PreferenceKeys.USER_LAYOUTS, CustomLayout.encodeList(layouts.filterNot { it.id == id }))
            if (selectedId == id) put(PreferenceKeys.SELECTED_CUSTOM_LAYOUT_ID, "")
            if (tpsId == id) put(PreferenceKeys.TPS_CUSTOM_LAYOUT_ID, "")
        }
}
