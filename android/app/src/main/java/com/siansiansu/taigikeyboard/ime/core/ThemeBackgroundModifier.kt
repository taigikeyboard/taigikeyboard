// Compose painting of a ThemeSurface (solid / gradient / photo) as a modifier + brush.

package com.siansiansu.taigikeyboard.ime.core

import android.graphics.Bitmap
import androidx.compose.foundation.background
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawWithCache
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.graphics.ColorMatrix
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.graphics.drawscope.clipRect
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.IntSize
import kotlin.math.roundToInt

/**
 * The gradient as a Compose brush over a surface of [size] that sits [topInsetPx] below the
 * keyboard top: the unit points are taken over the full keyboard (`size.height + topInsetPx`)
 * and shifted up by the inset, so a panel mounted below the smartbar shows exactly its slice
 * and stays continuous with the keyboard above it. A surface spanning the whole keyboard
 * passes the default inset 0.
 */
fun ThemeGradient.brush(
    size: Size,
    topInsetPx: Float = 0f,
): Brush {
    val (start, end) = unitPoints()
    val fullHeight = size.height + topInsetPx
    return Brush.linearGradient(
        colors = stops.map { Color(it) },
        start = Offset(start.x * size.width, start.y * fullHeight - topInsetPx),
        end = Offset(end.x * size.width, end.y * fullHeight - topInsetPx),
    )
}

/**
 * Paints [surface] as the keyboard surface: a solid color, the gradient, or the desaturated +
 * dimmed photo (aspect-fill over the whole keyboard, then this surface's slice — see
 * [ThemeGradient.brush] for [topInsetPx]); null (adaptive) paints [fallback]. The modifier is
 * remembered on its inputs so the draw cache survives the host's recompositions and the brush
 * is rebuilt only when the draw size changes. A missing photo file paints the seed grey.
 */
@Composable
fun Modifier.themeBackground(
    surface: ThemeSurface?,
    fallback: Color,
    topInsetPx: Float = 0f,
): Modifier {
    val context = LocalContext.current
    return then(
        remember(surface, fallback, topInsetPx) {
            when (val background = surface?.background) {
                is ThemeBackground.Solid -> Modifier.background(Color(background.color))
                is ThemeBackground.Gradient ->
                    Modifier.drawWithCache {
                        val brush = background.gradient.brush(size, topInsetPx)
                        onDrawBehind { drawRect(brush) }
                    }
                is ThemeBackground.Image ->
                    CompositionRoot
                        .shared(context)
                        .themeImages
                        .bitmap(background.image.file)
                        ?.let { Modifier.themePhoto(it, background.image, surface.dimsTowardWhite, topInsetPx) }
                        // A missing photo file paints the seed grey so the keyboard never renders see-through.
                        ?: Modifier.background(Color(UserThemeSeed.SOLID_COLOR))
                null -> Modifier.background(fallback)
            }
        },
    )
}

private val desaturate = ColorFilter.colorMatrix(ColorMatrix().apply { setToSaturation(ThemeImageBackground.SATURATION) })

/**
 * The desaturated, dimmed photo aspect-filled over the whole keyboard at its focus (`coverRect`, shifted
 * up by [topInsetPx]) and clipped to this surface; the image wrapper and destination rect are
 * computed once per draw size, only `drawImage` + the tone fill run per frame.
 */
private fun Modifier.themePhoto(
    bitmap: Bitmap,
    photo: ThemeImageBackground,
    dimsTowardWhite: Boolean,
    topInsetPx: Float,
): Modifier {
    val image = bitmap.asImageBitmap()
    val tone = (if (dimsTowardWhite) Color.White else Color.Black).copy(alpha = photo.dim)
    return drawWithCache {
        val rect =
            ThemeImageBackground.coverRect(
                imageWidth = bitmap.width.toFloat(),
                imageHeight = bitmap.height.toFloat(),
                bounds = SurfaceRect(left = 0f, top = -topInsetPx, width = size.width, height = size.height + topInsetPx),
                focusX = photo.focusX,
                focusY = photo.focusY,
            )
        val dstOffset = IntOffset(rect.left.roundToInt(), rect.top.roundToInt())
        val dstSize = IntSize(rect.width.roundToInt(), rect.height.roundToInt())
        onDrawBehind {
            clipRect {
                drawImage(image, srcOffset = IntOffset.Zero, srcSize = IntSize(bitmap.width, bitmap.height), dstOffset = dstOffset, dstSize = dstSize, colorFilter = desaturate)
                drawRect(tone)
            }
        }
    }
}
