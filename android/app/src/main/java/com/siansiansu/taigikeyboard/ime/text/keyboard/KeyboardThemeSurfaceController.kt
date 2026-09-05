package com.siansiansu.taigikeyboard.ime.text.keyboard

import android.graphics.drawable.GradientDrawable
import android.view.ViewGroup
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.InputView
import com.siansiansu.taigikeyboard.ime.core.KeyboardColorSettings
import com.siansiansu.taigikeyboard.ime.text.smartbar.SmartbarView

/**
 * Applies the View-layer side of a resolved theme. The background gradient is
 * painted ONCE on the common parent (`text_input_content`, which spans the
 * candidate bar through the keyboard body and is scoped to text input — the
 * media/emoji panel is a `ViewFlipper` sibling, so it is unaffected). The smartbar
 * chrome is then made transparent so the gradient shows continuously.
 *
 * A flat/legacy theme clears the gradient (back to the parent's `?keyboard_bgColor`)
 * and restores the attr-backed chrome, so the default path is visually unchanged.
 * The Compose keyboard body handles its own transparency separately, by reading
 * `hasBackgroundGradient`.
 */
internal class KeyboardThemeSurfaceController(
    private val inputView: InputView,
) {
    fun apply(colors: KeyboardColorSettings) {
        val content = inputView.findViewById<ViewGroup>(R.id.text_input_content)
        content?.background = colors.gradientStops()?.let { stops ->
            GradientDrawable(GradientDrawable.Orientation.TOP_BOTTOM, stops)
        }
        inputView.findViewById<SmartbarView>(R.id.smartbar)?.applyThemeSurface(colors)
    }
}
