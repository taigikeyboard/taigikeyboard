package com.siansiansu.taigikeyboard.ime.text.smartbar

import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * Pins the strip font-sizing variants (§42): mixed content keeps the 58/42
 * title/subtitle split; subtitle-free content sizes the title for ONE line
 * (100% of the available height, same clamps).
 */
class CandidateFontSizeTest {
    // trace: scaledDensity = 3*1 = 3; verticalPadding = 12*2/3 = 8 (Int);
    // verticalMargin = 4*4 = 16; gap = 2*3 = 6; available = 100-8-16-6 = 70
    // (above the 20dp*3 = 60 floor); lineHeightFactor = 1.15.
    private fun sizes(contentHasSubtitles: Boolean) =
        computeCandidateFontSizes(
            smartbarHeightPx = 100,
            paddingPx = 12,
            marginPx = 4,
            density = 3f,
            fontScale = 1f,
            textSizeScale = 1f,
            contentHasSubtitles = contentHasSubtitles,
        )

    @Test
    fun mixedContent_keeps58_42Split() {
        val (titleSp, subtitleSp) = sizes(contentHasSubtitles = true)
        // trace: title = 70*0.58/1.15/3 = 11.768…; subtitle = 70*0.42/1.15/3 = 8.522…
        assertEquals(11.768f, titleSp, 0.01f)
        assertEquals(8.522f, subtitleSp, 0.01f)
    }

    @Test
    fun subtitleFreeContent_titleSizedForOneLine() {
        val (titleSp, subtitleSp) = sizes(contentHasSubtitles = false)
        // trace: title = 70*1.0/1.15/3 = 20.290… — inside the unchanged 10..21 clamp.
        assertEquals(20.290f, titleSp, 0.01f)
        // The subtitle slot keeps its formula (nothing renders it in a subtitle-free list).
        assertEquals(8.522f, subtitleSp, 0.01f)
    }

    @Test
    fun singleLineVariant_sharesTheTitleClamp() {
        // trace: available = 400-8-16-6 = 384 → both variants exceed the 21sp cap.
        val single = computeCandidateFontSizes(400, 12, 4, 3f, 1f, 1f, contentHasSubtitles = false)
        val mixed = computeCandidateFontSizes(400, 12, 4, 3f, 1f, 1f, contentHasSubtitles = true)
        assertEquals(21f, single.first, 0.001f)
        assertEquals(21f, mixed.first, 0.001f)
    }
}
