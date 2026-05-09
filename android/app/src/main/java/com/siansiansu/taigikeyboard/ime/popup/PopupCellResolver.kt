package com.siansiansu.taigikeyboard.ime.popup

import android.content.res.Resources
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.text.key.KeyCode
import com.siansiansu.taigikeyboard.ime.text.key.KeyData
import com.siansiansu.taigikeyboard.ime.text.keyboard.computeKeyLetter

/**
 * Resolves the visual [PopupCell] list for the given [KeyData.popup] entries.
 * Direct port of the legacy `KeyPopupManager.buildPopupCell` switch — moved
 * to a free function so the keyboard-side caller can pre-resolve cells when
 * building a [KeyAnchor], leaving the popup layer a pure renderer of state.
 */
internal fun buildPopupCells(
    data: KeyData,
    inputMode: String,
    caps: Boolean,
    capsLock: Boolean,
    resources: Resources,
): List<PopupCell> =
    data.popup.map { popupKeyData ->
        when (popupKeyData.code) {
            KeyCode.SETTINGS -> PopupCell(
                label = null,
                icon = PopupIcon.Settings,
                textScale = 1.0f,
                useCustomTypeface = false,
            )
            // Legacy KeyPopupExtendedSingleView did NOT apply the custom
            // typeface on this branch — preserve that behavior.
            KeyCode.SWITCH_TO_TEXT_CONTEXT -> PopupCell(
                label = resources.getString(R.string.key__view_characters),
                icon = null,
                textScale = 1.0f,
                useCustomTypeface = false,
            )
            KeyCode.SWITCH_TO_MEDIA_CONTEXT -> PopupCell(
                label = null,
                icon = PopupIcon.SentimentSatisfied,
                textScale = 1.0f,
                useCustomTypeface = false,
            )
            else -> PopupCell(
                label = computeKeyLetter(popupKeyData, inputMode, caps, capsLock),
                icon = null,
                textScale = if (popupKeyData.code == KeyCode.URI_COMPONENT_TLD) 0.6f else 1.0f,
                useCustomTypeface = true,
            )
        }
    }
