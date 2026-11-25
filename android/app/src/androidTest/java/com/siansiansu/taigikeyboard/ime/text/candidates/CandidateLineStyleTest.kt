package com.siansiansu.taigikeyboard.ime.text.candidates

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextMeasurer
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.createFontFamilyResolver
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.sp
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.assertEquals
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Layout audit A1: with the System font, a Hanji line falls back to the CJK font, whose
 * metrics made the line ~1.45× the font size while the strip sized text for 1.15× — the
 * subtitle was clipped. [candidateLineStyle] pins every line to the factor the sizing uses.
 */
@RunWith(AndroidJUnit4::class)
class CandidateLineStyleTest {
    private val context = InstrumentationRegistry.getInstrumentation().targetContext
    private val density = Density(context)
    private val measurer = TextMeasurer(createFontFamilyResolver(context), density, LayoutDirection.Ltr)

    private fun lineHeightPx(
        text: String,
        fontSp: Float,
    ): Int = measurer.measure(text, candidateLineStyle(fontSp, Color.Black, FontFamily.Default)).size.height

    @Test
    fun hanjiAndRomanLines_takeTheSizedLineBox_withTheSystemFont() {
        val fontSp = 20f
        val expectedPx = with(density) { (fontSp * CANDIDATE_LINE_HEIGHT_FACTOR).sp.toPx() }
        for (text in listOf("一", "這", "tsit", "Tsi̍t", "ǹg")) {
            assertEquals("line box of \"$text\"", expectedPx, lineHeightPx(text, fontSp).toFloat(), 1f)
        }
    }
}
