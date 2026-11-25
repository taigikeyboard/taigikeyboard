// Pure placement rules for the text drawn on a key face (label + hints). Every margin is a
// fraction of the key height, so spacing scales with the text, which is also sized from it.

package com.siansiansu.taigikeyboard.ime.text.keyboard

internal object KeyLabelGeometry {
    /** Margin between any text ink and the key's top edge, as a fraction of key height. */
    const val EDGE_INSET = 0.12f

    /** Gap between a hint's ink and the label's ink below it, as a fraction of key height. */
    const val HINT_GAP = 0.06f

    /** Font-size [scale] (≤ 1) and [baseline] that place a hint inside its band. */
    data class Fit(
        val scale: Float,
        val baseline: Float,
    )

    /**
     * Places a hint above the label inside the band from the top inset down to [HINT_GAP] above
     * the label's ink top. The hint's ink (`inkTop` / `inkBottom` relative to the baseline,
     * negative up, at its nominal size) hugs the band's top, so a row of hints shares one line
     * whatever glyph sits below, and shrinks to fit a shorter band.
     */
    fun hintAboveLabel(
        keyHeight: Float,
        labelInkTop: Float,
        inkTop: Float,
        inkBottom: Float,
    ): Fit {
        val bandTop = keyHeight * EDGE_INSET
        val band = (labelInkTop - keyHeight * HINT_GAP - bandTop).coerceAtLeast(0f)
        val inkHeight = inkBottom - inkTop
        val scale = if (inkHeight <= 0f) 1f else minOf(1f, band / inkHeight)
        return Fit(scale, bandTop - inkTop * scale)
    }
}
