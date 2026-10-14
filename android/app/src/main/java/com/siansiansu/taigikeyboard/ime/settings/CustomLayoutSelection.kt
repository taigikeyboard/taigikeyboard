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

    /** Selects [layout]: its base through the TPS-aware layout cascade, then its id. */
    fun forApply(
        layout: CustomLayout,
        currentLayout: String,
        currentInputMode: String,
        inputModeBeforeTps: String,
    ): Map<Preferences.Key<*>, Any> =
        buildMap {
            putAll(TpsCascade.forKeyboardLayoutType(layout.base, currentLayout, currentInputMode, inputModeBeforeTps))
            put(PreferenceKeys.SELECTED_CUSTOM_LAYOUT_ID, layout.id)
        }

    /**
     * Saves [layout] into [layouts] (replacing its saved version) and applies it; null when a new
     * layout would pass [CustomLayout.MAX_CUSTOM_LAYOUTS].
     */
    fun forSave(
        layouts: List<CustomLayout>,
        layout: CustomLayout,
        currentLayout: String,
        currentInputMode: String,
        inputModeBeforeTps: String,
    ): Map<Preferences.Key<*>, Any>? {
        val saved = CustomLayout.upserted(layouts, layout) ?: return null
        return buildMap {
            putAll(forApply(layout, currentLayout, currentInputMode, inputModeBeforeTps))
            put(PreferenceKeys.USER_LAYOUTS, CustomLayout.encodeList(saved))
        }
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
