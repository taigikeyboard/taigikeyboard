package com.siansiansu.taigikeyboard.ime.text.keyboard

import android.content.res.ColorStateList
import android.graphics.LinearGradient
import android.graphics.Shader
import android.graphics.drawable.ColorDrawable
import android.graphics.drawable.Drawable
import android.graphics.drawable.PaintDrawable
import android.graphics.drawable.ShapeDrawable
import android.view.ViewGroup
import android.widget.Button
import android.widget.ImageButton
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.CompositionRoot
import com.siansiansu.taigikeyboard.ime.core.InputView
import com.siansiansu.taigikeyboard.ime.core.KeyboardColorSettings
import com.siansiansu.taigikeyboard.ime.core.ThemeBackground
import com.siansiansu.taigikeyboard.ime.core.ThemeGradient
import com.siansiansu.taigikeyboard.ime.core.ThemeImageBackground
import com.siansiansu.taigikeyboard.ime.core.ThemeImageVariant
import com.siansiansu.taigikeyboard.ime.core.ThemeSurface
import com.siansiansu.taigikeyboard.ime.core.UserThemeSeed
import com.siansiansu.taigikeyboard.ime.text.smartbar.SmartbarView
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch

/**
 * Applies the View-layer side of a resolved theme. The theme background (solid,
 * gradient or photo) is painted on each [SURFACE_TARGETS] view: `text_input_content`
 * (spans the candidate bar through the keyboard body, one continuous fill) and
 * `media_input` (the emoji panel, a `ViewFlipper` sibling — USER 2026-09-26: the emoji
 * panel applies the theme too). The smartbar chrome is then made transparent so the
 * surface shows continuously, and the emoji bottom bar takes the key text color.
 *
 * The adaptive default clears the drawable (back to the parent's `?keyboard_bgColor`)
 * and restores the attr-backed chrome, so the default path is visually unchanged.
 * The Compose keyboard body handles its own transparency separately, by reading
 * `background != null`.
 *
 * A photo not yet decoded paints the seed grey and is decoded off the main thread on
 * [scope] (the IME-lifetime scope); it is swapped in only if no newer surface was applied
 * meanwhile ([generation]), so a stale decode never overwrites the current theme.
 */
internal class KeyboardThemeSurfaceController(
    private val inputView: InputView,
    private val scope: CoroutineScope,
) {
    private var applied: ThemeSurface? = null
    private var hasApplied = false
    private var generation = 0
    private var photoJob: Job? = null
    private var photoJobGeneration = -1
    private var mediaBarDefaults: Pair<ColorStateList, ColorStateList?>? = null

    fun apply(colors: KeyboardColorSettings) {
        // apply() runs on every keyboard show; only re-allocate the drawable when the
        // resolved surface actually changed.
        val surface = colors.surface
        if (!hasApplied || applied != surface) {
            hasApplied = true
            applied = surface
            generation++
            photoJob?.cancel()
            setSurfaceDrawable { surface?.let { drawable(it) } }
        }
        inputView.findViewById<SmartbarView>(R.id.smartbar)?.applyThemeSurface(colors)
        applyMediaBarForeground(colors)
    }

    /** Paints each surface target with its own drawable instance (bounds are per view). */
    private fun setSurfaceDrawable(drawable: () -> Drawable?) {
        for (id in SURFACE_TARGETS) inputView.findViewById<ViewGroup>(id)?.background = drawable()
    }

    /**
     * The emoji panel's bottom bar (ABC + backspace) takes the key text color on a themed surface,
     * else the colors its layout declares (captured on first apply).
     */
    private fun applyMediaBarForeground(colors: KeyboardColorSettings) {
        val abc = inputView.findViewById<Button>(R.id.media_input_switch_to_text_input_button) ?: return
        val backspace = inputView.findViewById<ImageButton>(R.id.media_input_backspace_button) ?: return
        val defaults = mediaBarDefaults ?: (abc.textColors to backspace.imageTintList).also { mediaBarDefaults = it }
        val foreground = colors.keyTextColor?.takeIf { colors.surface != null }?.let { ColorStateList.valueOf(it) }
        abc.setTextColor(foreground ?: defaults.first)
        backspace.imageTintList = foreground ?: defaults.second
    }

    private fun drawable(surface: ThemeSurface): Drawable =
        when (val background = surface.background) {
            is ThemeBackground.Solid -> ColorDrawable(background.color)
            is ThemeBackground.Gradient -> gradientDrawable(background.gradient)
            is ThemeBackground.Image -> photoDrawable(background.image, surface.dimsTowardWhite)
        }

    /**
     * The photo drawable on a cache hit; otherwise the seed grey now (also the final look when
     * the file is missing, so the keyboard never renders see-through) and the photo once decoded.
     */
    private fun photoDrawable(
        photo: ThemeImageBackground,
        dimsTowardWhite: Boolean,
    ): Drawable {
        val images = CompositionRoot.shared(inputView.context).themeImages
        images.cached(photo.file, ThemeImageVariant.FULL)?.let { return ThemeImageDrawable(it, photo, dimsTowardWhite) }
        val requested = generation
        // One decode per surface change, though every surface target asks for a drawable.
        if (photoJobGeneration == requested) return ColorDrawable(UserThemeSeed.SOLID_COLOR)
        photoJobGeneration = requested
        photoJob =
            scope.launch {
                val bitmap = images.load(photo.file, ThemeImageVariant.FULL) ?: return@launch
                if (requested == generation) setSurfaceDrawable { ThemeImageDrawable(bitmap, photo, dimsTowardWhite) }
            }
        return ColorDrawable(UserThemeSeed.SOLID_COLOR)
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

    private companion object {
        /** Views that paint the theme surface (text input + emoji panel). */
        val SURFACE_TARGETS = intArrayOf(R.id.text_input_content, R.id.media_input)
    }
}
