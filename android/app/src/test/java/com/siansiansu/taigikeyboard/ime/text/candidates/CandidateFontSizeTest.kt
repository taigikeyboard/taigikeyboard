package com.siansiansu.taigikeyboard.ime.text.candidates

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import kotlin.math.ceil

/**
 * Pins the strip font-sizing variants (§42): mixed content keeps the 58/42
 * title/subtitle split; subtitle-free content sizes the title for ONE line
 * (100% of the available height, same cap). The stacked line boxes always fit
 * the strip (layout audit A1).
 */
class CandidateFontSizeTest {
    // trace: scaledDensity = 3*1 = 3; verticalPadding = 12/3*2 = 8;
    // verticalMargin = 4*4 = 16; gap = 4*2 = 8 with subtitles, 0 without; line rounding 2 / 1;
    // available = 100-8-16-8-2 = 66 (mixed) / 100-8-16-1 = 75 (subtitle-free);
    // lineHeightFactor = 1.15.
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
        // trace: title = 66*0.58/1.15/3 = 11.095…; subtitle = 66*0.42/1.15/3 = 8.034…
        assertEquals(11.095f, titleSp, 0.01f)
        assertEquals(8.034f, subtitleSp, 0.01f)
    }

    @Test
    fun subtitleFreeContent_titleSizedForOneLine() {
        val (titleSp, _) = sizes(contentHasSubtitles = false)
        // trace: title = 75*1.0/1.15/3 = 21.739… → the 21sp cap.
        assertEquals(21f, titleSp, 0.001f)
    }

    @Test
    fun singleLineVariant_sharesTheTitleClamp() {
        // trace: available = 400-8-16-1 = 375 / 400-8-16-8-2 = 366 → both exceed the 21sp cap.
        val single = computeCandidateFontSizes(400, 12, 4, 3f, 1f, 1f, contentHasSubtitles = false)
        val mixed = computeCandidateFontSizes(400, 12, 4, 3f, 1f, 1f, contentHasSubtitles = true)
        assertEquals(21f, single.first, 0.001f)
        assertEquals(21f, mixed.first, 0.001f)
    }

    // A1: the System font's CJK fallback made each line ~1.45× its font size while the sizes
    // assumed 1.15×, so the subtitle was clipped. Lines now render in a fixed
    // CANDIDATE_LINE_HEIGHT_FACTOR box; the stack must fit for every strip height, density,
    // font scale and theme text-size scale — no floor may push it past the strip.
    @Test
    fun stackedLineBoxes_fitTheStrip_atEveryScale() {
        for (density in listOf(2f, 2.625f, 3f, 3.5f)) {
            for (fontScale in listOf(0.85f, 1f, 1.3f, 2f)) {
                for (textSizeScale in listOf(0.85f, 1f, 1.15f)) {
                    for (heightDp in listOf(40, 48, 52, 56, 64)) {
                        val heightPx = (heightDp * density).toInt()
                        val paddingPx = (6 * density + 0.5f).toInt()
                        val marginPx = (1 * density + 0.5f).toInt()
                        val (titleSp, subtitleSp) =
                            computeCandidateFontSizes(heightPx, paddingPx, marginPx, density, fontScale, textSizeScale, true)
                        val scaledDensity = density * fontScale
                        // Each line box is rounded up to a whole pixel, as LineHeightStyleSpan does.
                        val stackPx =
                            ceil(titleSp * scaledDensity * CANDIDATE_LINE_HEIGHT_FACTOR) +
                                ceil(subtitleSp * scaledDensity * CANDIDATE_LINE_HEIGHT_FACTOR) +
                                paddingPx / 3 * 2 + marginPx * 4 + marginPx * 2
                        assertTrue(
                            "d=$density fs=$fontScale ts=$textSizeScale h=${heightDp}dp stack=$stackPx > $heightPx",
                            stackPx <= heightPx + 0.01f,
                        )
                    }
                }
            }
        }
    }
}
