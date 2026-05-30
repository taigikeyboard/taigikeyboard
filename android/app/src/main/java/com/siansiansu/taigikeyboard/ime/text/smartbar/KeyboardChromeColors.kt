// Snapshots the user-customizable keyboard chrome colors (foreground / accent / background)
// for Compose overlays that must honor KeyboardColorSettings rather than the fixed M3 brand palette.

package com.siansiansu.taigikeyboard.ime.text.smartbar

import android.content.Context
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.theme.getColorFromAttr

/**
 * User-customizable keyboard chrome colors, resolved from the keyboard theme attributes
 * (`smartbar_fgColor` / `smartbar_accentColor` / `keyboard_bgColor`).
 *
 * Keyboard overlays must use THESE colors, not [androidx.compose.material3.MaterialTheme]
 * `colorScheme`, so the user's ColorPickerDialog customization keeps applying. The fixed
 * M3 brand palette is reserved for app-style chrome (settings/emoji overlays).
 */
data class KeyboardChromeColors(
    val foreground: Color,
    val accent: Color,
    val background: Color,
) {
    companion object {
        fun from(context: Context): KeyboardChromeColors =
            KeyboardChromeColors(
                foreground = Color(getColorFromAttr(context, R.attr.smartbar_fgColor)),
                accent = Color(getColorFromAttr(context, R.attr.smartbar_accentColor)),
                background = Color(getColorFromAttr(context, R.attr.keyboard_bgColor)),
            )
    }
}

/**
 * Resolves [KeyboardChromeColors] from the current Compose context theme.
 *
 * @param refreshKey bump to force re-resolution (e.g. on overlay show()), so a live
 *   keyboard-color change is picked up without recreating the composition.
 */
@Composable
fun rememberKeyboardChromeColors(refreshKey: Int = 0): KeyboardChromeColors {
    val context = LocalContext.current
    return remember(refreshKey, context) { KeyboardChromeColors.from(context) }
}
