package com.siansiansu.taigikeyboard.ime.text.keyboard

import android.graphics.LinearGradient
import android.graphics.Shader
import android.graphics.drawable.ColorDrawable
import android.graphics.drawable.Drawable
import android.graphics.drawable.PaintDrawable
import android.graphics.drawable.ShapeDrawable
import android.view.ViewGroup
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.CompositionRoot
import com.siansiansu.taigikeyboard.ime.core.InputView
import com.siansiansu.taigikeyboard.ime.core.KeyboardColorSettings
import com.siansiansu.taigikeyboard.ime.core.ThemeBackground
import com.siansiansu.taigikeyboard.ime.core.ThemeGradient
import com.siansiansu.taigikeyboard.ime.core.ThemeSurface
import com.siansiansu.taigikeyboard.ime.core.UserThemeSeed
import com.siansiansu.taigikeyboard.ime.text.smartbar.SmartbarView

/**
 * Applies the View-layer side of a resolved theme. The theme background (solid or
 * gradient) is painted ONCE on the common parent (`text_input_content`, which spans
 * the candidate bar through the keyboard body and is scoped to text input — the
 * media/emoji panel is a `ViewFlipper` sibling, so it is unaffected). The smartbar
 * chrome is then made transparent so the surface shows continuously.
 *
 * The adaptive default clears the drawable (back to the parent's `?keyboard_bgColor`)
 * and restores the attr-backed chrome, so the default path is visually unchanged.
 * The Compose keyboard body handles its own transparency separately, by reading
 * `background != null`.
 */
internal class KeyboardThemeSurfaceController(
    private val inputView: InputView,
) {
    private var applied: ThemeSurface? = null
    private var hasApplied = false

    fun apply(colors: KeyboardColorSettings) {
        // apply() runs on every keyboard show; only re-allocate the drawable when the
        // resolved surface actually changed.
        val surface = colors.surface
        if (!hasApplied || applied != surface) {
            hasApplied = true
            applied = surface
            inputView.findViewById<ViewGroup>(R.id.text_input_content)?.background = surface?.let { drawable(it) }
        }
        inputView.findViewById<SmartbarView>(R.id.smartbar)?.applyThemeSurface(colors)
    }

    private fun drawable(surface: ThemeSurface): Drawable =
        when (val background = surface.background) {
            is ThemeBackground.Solid -> ColorDrawable(background.color)
            is ThemeBackground.Gradient -> gradientDrawable(background.gradient)
            is ThemeBackground.Image ->
                // A missing photo file paints the seed grey so the keyboard never renders see-through.
                CompositionRoot
                    .shared(inputView.context)
                    .themeImages
                    .bitmap(background.image.file)
                    ?.let { ThemeImageDrawable(it, background.image, surface.dimsTowardWhite) }
                    ?: ColorDrawable(UserThemeSeed.SOLID_COLOR)
        }

    /**
     * The same direction math as the Compose brush (`ThemeGradient.unitPoints`), so the View
     * parent and the Compose overlays / preview render one gradient for any angle. The shader
     * factory is re-run only when the drawable's bounds change.
     */
    private fun gradientDrawable(gradient: ThemeGradient): Drawable =
        PaintDrawable().apply {
            shape = android.graphics.drawable.shapes
                .RectShape()
            shaderFactory =
                object : ShapeDrawable.ShaderFactory() {
                    override fun resize(
                        width: Int,
                        height: Int,
                    ): Shader {
                        val (start, end) = gradient.unitPoints()
                        return LinearGradient(
                            start.x * width,
                            start.y * height,
                            end.x * width,
                            end.y * height,
                            gradient.stops.toIntArray(),
                            null,
                            Shader.TileMode.CLAMP,
                        )
                    }
                }
        }
}
