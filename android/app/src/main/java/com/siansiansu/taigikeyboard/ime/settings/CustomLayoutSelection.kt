// Pure custom-layout selection — computes the DataStore write batch for applying, saving and
// deleting a custom layout, and for choosing a built-in one.

package com.siansiansu.taigikeyboard.ime.settings

import androidx.datastore.preferences.core.Preferences
import com.siansiansu.taigikeyboard.ime.text.layout.CustomLayout

/**
 * Selection = `KEYBOARD_LAYOUT_TYPE` (the base) + `SELECTED_CUSTOM_LAYOUT_ID`. Choosing a built-in
 * layout clears the id, even on the same base; the input-mode TPS cascade ([TpsCascade.forInputMode])
 * leaves it, so leaving TPS resumes the layout. Pure: takes the current state, returns the plan
 * [PrefHelper] commits in one transaction. Mirrors iOS `SharedSettings` § Custom layouts.
 */
internal object CustomLayoutSelection {
    /** [layoutWrites] (a built-in layout's cascade) plus clearing the custom selection. */
    fun forBuiltIn(layoutWrites: Map<Preferences.Key<*>, Any>): Map<Preferences.Key<*>, Any> =
        buildMap {
            putAll(layoutWrites)
            put(PreferenceKeys.SELECTED_CUSTOM_LAYOUT_ID, "")
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

    /** Selects [layout]: [modeWrites] (entering its script, or empty) and its base ([forLayoutAfterMode]), then its id. */
    fun forApply(
        layout: CustomLayout,
        modeWrites: Map<Preferences.Key<*>, Any>,
        currentLayout: String,
        currentInputMode: String,
        inputModeBeforeTps: String,
    ): Map<Preferences.Key<*>, Any> =
        buildMap {
            putAll(forLayoutAfterMode(layout.base, modeWrites, currentLayout, currentInputMode, inputModeBeforeTps))
            put(PreferenceKeys.SELECTED_CUSTOM_LAYOUT_ID, layout.id)
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

    /** Deletes the layout with [id]; deleting the selected one leaves its base selected as a built-in layout. */
    fun forDelete(
        layouts: List<CustomLayout>,
        id: String,
        selectedId: String,
    ): Map<Preferences.Key<*>, Any> =
        buildMap {
            put(PreferenceKeys.USER_LAYOUTS, CustomLayout.encodeList(layouts.filterNot { it.id == id }))
            if (selectedId == id) put(PreferenceKeys.SELECTED_CUSTOM_LAYOUT_ID, "")
        }
}
