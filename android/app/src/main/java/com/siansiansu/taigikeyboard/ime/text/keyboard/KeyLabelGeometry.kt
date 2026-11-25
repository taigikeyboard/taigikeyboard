// Pure placement rules for the text drawn on a key face (label + hints). Every margin is a
// fraction of the key height, so spacing scales with the text, which is also sized from it.

package com.siansiansu.taigikeyboard.ime.text.keyboard

internal object KeyLabelGeometry {
    /** Margin between any text and the key's edges, as a fraction of key height. */
    const val EDGE_INSET = 0.12f

    /**
     * Widest a multi-character label (`?123`, `TPS`, `……`) may be: the key width minus an
     * [EDGE_INSET] margin on each side.
     */
    fun maxLabelWidth(
        keyWidth: Float,
        keyHeight: Float,
    ): Float = insetWidth(keyWidth, keyHeight, EDGE_INSET)

    /** Margin beside a one-glyph label, as a fraction of key height. */
    const val GLYPH_SIDE_INSET = 0.06f

    /**
     * Largest font size for a one-glyph label: a full-width (1 em) glyph, such as a Bopomofo
     * letter, fits the key minus a [GLYPH_SIDE_INSET] margin on each side. It does not depend on
     * the glyph, so keys of one width keep one text size (`m` is not drawn smaller than `n`).
     */
    fun maxGlyphTextSize(
        keyWidth: Float,
        keyHeight: Float,
    ): Float = insetWidth(keyWidth, keyHeight, GLYPH_SIDE_INSET)

    /** Key width less [insetFraction] of the key height on each side; never under half the key, so a very narrow key keeps a legible label. */
    private fun insetWidth(
        keyWidth: Float,
        keyHeight: Float,
        insetFraction: Float,
    ): Float = maxOf(keyWidth - 2f * keyHeight * insetFraction, keyWidth * 0.5f)

    /** Gap between a hint's ink and the label's ink below it, as a fraction of key height. */
    const val HINT_GAP = 0.06f

    /**
     * Whether [label] is one full-width punctuation mark (CJK Symbols and Punctuation, or the
     * Fullwidth Forms such as ？，！). Its glyph sits in the left part of its full-width advance,
     * so centring the advance leaves the mark off-centre; it is centred by its ink instead.
     */
    fun centresByInk(label: String): Boolean {
        if (label.isEmpty() || label.codePointCount(0, label.length) != 1) return false
        val cp = label.codePointAt(0)
        return cp in 0x3001..0x303F ||
            cp in 0xFF01..0xFF0F ||
            cp in 0xFF1A..0xFF20 ||
            cp in 0xFF3B..0xFF40 ||
            cp in 0xFF5B..0xFF65
    }

    /** Left x at which to draw (left-aligned) ink spanning [inkLeft, inkRight] centred on [centerX]. */
    fun inkCentredX(
        centerX: Float,
        inkLeft: Float,
        inkRight: Float,
    ): Float = centerX - (inkLeft + inkRight) / 2f

    /** Gap between the two hints of a two-hint key, as a fraction of key height. */
    const val HINT_PAIR_GAP = 0.10f

    /**
     * Centres of two side-by-side hints of widths [leftWidth] / [rightWidth], [HINT_PAIR_GAP]
     * apart, the pair as a whole centred on the key (pinning them to the edges crowded a phone
     * key's corner and pushed a wide tablet key's hints into its corners).
     */
    fun hintPairCentres(
        centerX: Float,
        keyHeight: Float,
        leftWidth: Float,
        rightWidth: Float,
    ): Pair<Float, Float> {
        val gap = keyHeight * HINT_PAIR_GAP
        val left = centerX - (leftWidth + gap + rightWidth) / 2f
        return Pair(left + leftWidth / 2f, left + leftWidth + gap + rightWidth / 2f)
    }

    /** Font-size [scale] (≤ 1) and [baseline] that place a hint inside its band. */
    data class Fit(
        val scale: Float,
        val baseline: Float,
    )

    /**
     * Places a hint above the label inside the band from the top inset down to [HINT_GAP] above
     * the label's ink top. The hint's ink (`inkTop` / `inkBottom` relative to the baseline,
     * negative up, at its nominal size) shrinks to fit a shorter band. It hugs the band's top
     * (so a row of hints shares one line whatever glyph sits below), or with [hugLabel] its
     * bottom (a tone mark sits right above its digit, as an accent does).
     */
    fun hintAboveLabel(
        keyHeight: Float,
        labelInkTop: Float,
        inkTop: Float,
        inkBottom: Float,
        hugLabel: Boolean = false,
    ): Fit {
        val bandTop = keyHeight * EDGE_INSET
        val band = (labelInkTop - keyHeight * HINT_GAP - bandTop).coerceAtLeast(0f)
        val inkHeight = inkBottom - inkTop
        val scale = if (inkHeight <= 0f) 1f else minOf(1f, band / inkHeight)
        val inkTopY = if (hugLabel) bandTop + band - inkHeight * scale else bandTop
        return Fit(scale, inkTopY - inkTop * scale)
    }
}
