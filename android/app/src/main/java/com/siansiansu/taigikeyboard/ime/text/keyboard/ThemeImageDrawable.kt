// View-layer photo surface for text_input_content: cover-fit, desaturated, tone-dimmed.

package com.siansiansu.taigikeyboard.ime.text.keyboard

import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.ColorFilter
import android.graphics.ColorMatrix
import android.graphics.ColorMatrixColorFilter
import android.graphics.Paint
import android.graphics.PixelFormat
import android.graphics.Rect
import android.graphics.RectF
import android.graphics.drawable.Drawable
import com.siansiansu.taigikeyboard.ime.core.SurfaceRect
import com.siansiansu.taigikeyboard.ime.core.ThemeImageBackground
import kotlin.math.roundToInt

/**
 * Draws the theme photo aspect-filled over the drawable's bounds (the whole keyboard
 * parent) at the photo's focus, at [ThemeImageBackground.SATURATION], then the tone overlay
 * at the photo's dim — the same look `Modifier.themeBackground` gives the Compose overlays.
 */
internal class ThemeImageDrawable(
    private val bitmap: Bitmap,
    private val photo: ThemeImageBackground,
    dimsTowardWhite: Boolean,
) : Drawable() {
    private val photoPaint =
        Paint(Paint.FILTER_BITMAP_FLAG).apply {
            colorFilter = ColorMatrixColorFilter(ColorMatrix().apply { setSaturation(ThemeImageBackground.SATURATION) })
        }
    private val tonePaint =
        Paint().apply {
            color = if (dimsTowardWhite) android.graphics.Color.WHITE else android.graphics.Color.BLACK
            alpha = (photo.dim * 255).roundToInt()
        }
    private val source = Rect(0, 0, bitmap.width, bitmap.height)
    private val destination = RectF()

    override fun onBoundsChange(bounds: Rect) {
        val cover =
            ThemeImageBackground.coverRect(
                imageWidth = bitmap.width.toFloat(),
                imageHeight = bitmap.height.toFloat(),
                bounds = SurfaceRect(bounds.left.toFloat(), bounds.top.toFloat(), bounds.width().toFloat(), bounds.height().toFloat()),
                focusX = photo.focusX,
                focusY = photo.focusY,
            )
        destination.set(cover.left, cover.top, cover.left + cover.width, cover.top + cover.height)
    }

    override fun draw(canvas: Canvas) {
        canvas.save()
        canvas.clipRect(bounds)
        canvas.drawBitmap(bitmap, source, destination, photoPaint)
        canvas.drawRect(bounds, tonePaint)
        canvas.restore()
    }

    override fun setAlpha(alpha: Int) = Unit

    override fun setColorFilter(colorFilter: ColorFilter?) = Unit

    @Deprecated("Deprecated in Java")
    override fun getOpacity(): Int = PixelFormat.OPAQUE
}
