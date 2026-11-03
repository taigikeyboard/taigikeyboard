// The layout each romanization script last used, so switching Tâi-lô ↔ Pe̍h-ōe-jī brings it back.
// Mirrors iOS Layout/LayoutScriptMemory.swift and SharedSettings.enterLayoutScript.

package com.siansiansu.taigikeyboard.ime.settings

import androidx.datastore.preferences.core.Preferences
import com.siansiansu.taigikeyboard.ime.text.layout.CustomLayout
import com.siansiansu.taigikeyboard.ime.text.layout.KeyboardLayoutOptions
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutScript

/** A layout selection: a base and, while a custom layout is selected, its id ("" for none). */
internal data class LayoutSelection(
    val base: String,
    val customId: String,
) {
    /** `"<base>"` or `"<base>|<custom id>"`; the same spelling on iOS. */
    val storage: String
        get() = if (customId.isEmpty()) base else "$base|$customId"

    companion object {
        fun fromStorage(value: String): LayoutSelection? {
            val parts = value.split("|")
            if (parts.size > 2 || KeyboardLayoutOptions.romanization.none { it.key == parts[0] }) return null
            return LayoutSelection(parts[0], parts.getOrElse(1) { "" })
        }
    }
}

internal object LayoutScriptMemory {
    /**
     * The write plan for entering [newMode] from [oldMode] with [liveLayout] showing (after the TPS
     * cascade): entering the other romanization script swaps the live selection with that script's
     * remembered one ([selection]); English and TPS keep none, so TL → English → POJ swaps too. An
     * empty [owner] (first switch after an upgrade) is [oldMode]'s script.
     */
    fun forInputMode(
        newMode: String,
        oldMode: String,
        owner: String,
        liveLayout: String,
        liveCustomId: String,
        remembered: (LayoutScript) -> String,
        layouts: () -> List<CustomLayout>,
    ): Map<Preferences.Key<*>, Any> {
        val script = LayoutScript.fromStorage(newMode) ?: return emptyMap()
        val ownerScript = LayoutScript.fromStorage(owner) ?: LayoutScript.fromStorage(oldMode) ?: LayoutScript.TL
        return buildMap {
            if (owner != script.inputMode) put(PreferenceKeys.LAYOUT_SCRIPT_OWNER, script.inputMode)
            if (script == ownerScript) return@buildMap
            val live = LayoutSelection(liveLayout, liveCustomId)
            val next = selection(script, LayoutSelection.fromStorage(remembered(script)), live, layouts())
            put(selectionKey(ownerScript), live.storage)
            put(PreferenceKeys.KEYBOARD_LAYOUT_TYPE, next.base)
            put(PreferenceKeys.SELECTED_CUSTOM_LAYOUT_ID, next.customId)
        }
    }

    /**
     * The selection to show after entering [script]: the one [remembered] for it, else [live] —
     * either one's custom layout only while it is still live over that base ([CustomLayout.active])
     * and shows in [script] ([CustomLayout.isUniversal], or made for it), else its base.
     */
    fun selection(
        script: LayoutScript,
        remembered: LayoutSelection?,
        live: LayoutSelection,
        layouts: List<CustomLayout>,
    ): LayoutSelection {
        val selection = remembered ?: live
        val layout = CustomLayout.active(layouts, selection.customId, selection.base)
        val fits = layout != null && (layout.isUniversal || layout.script == script)
        return if (fits) selection else selection.copy(customId = "")
    }

    fun selectionKey(script: LayoutScript): Preferences.Key<String> =
        when (script) {
            LayoutScript.TL -> PreferenceKeys.TL_LAYOUT_SELECTION
            LayoutScript.POJ -> PreferenceKeys.POJ_LAYOUT_SELECTION
        }
}
