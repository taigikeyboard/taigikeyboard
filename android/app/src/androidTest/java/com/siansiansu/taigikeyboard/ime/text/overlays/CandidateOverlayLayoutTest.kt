package com.siansiansu.taigikeyboard.ime.text.overlays

import android.graphics.Typeface
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.SemanticsMatcher
import androidx.compose.ui.test.getBoundsInRoot
import androidx.compose.ui.test.hasScrollToIndexAction
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performScrollToNode
import androidx.compose.ui.text.TextLayoutResult
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.test.ext.junit.runners.AndroidJUnit4
import com.siansiansu.taigikeyboard.i18n.DisplayLanguage
import com.siansiansu.taigikeyboard.i18n.ProvideDisplayLanguage
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord
import com.siansiansu.taigikeyboard.ime.settings.CandidateDisplayMode
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/** Layout audit: the expanded candidate grid stays inside the overlay and its own cells. */
@RunWith(AndroidJUnit4::class)
class CandidateOverlayLayoutTest {
    @get:Rule
    val rule = createComposeRule()

    private val navBarInset = 48.dp

    private fun setGrid(
        words: List<TaigiWord>,
        height: Dp = 300.dp,
        fontScale: Float? = null,
        width: Dp? = null,
    ) {
        rule.setContent {
            val density = LocalDensity.current
            CompositionLocalProvider(LocalDensity provides Density(density.density, fontScale ?: density.fontScale)) {
                ProvideDisplayLanguage(DisplayLanguage.ENGLISH) {
                    Box(Modifier.then(if (width != null) Modifier.width(width) else Modifier.fillMaxWidth()).height(height)) {
                        CandidateOverlayContent(
                            suggestions = words,
                            typeface = Typeface.DEFAULT,
                            isTPSLayout = false,
                            isHanjiFirst = true,
                            isFullWidthPunctuation = false,
                            candidateDisplayMode = CandidateDisplayMode.SIDE_BY_SIDE,
                            resetKey = 0,
                            surface = null,
                            candidateTextColor = null,
                            firstCandidateColor = 0,
                            pressedColor = 0,
                            controlPressedColor = null,
                            onSuggestionSelected = { _, _ -> },
                            onCollapse = {},
                            onTranslateToggle = {},
                            bottomInset = navBarInset,
                        )
                    }
                }
            }
        }
    }

    private fun visibleTextLayouts(): List<Pair<String, TextLayoutResult>> =
        rule
            .onAllNodes(SemanticsMatcher.keyIsDefined(SemanticsProperties.Text), useUnmergedTree = true)
            .fetchSemanticsNodes()
            .mapNotNull { node ->
                val layouts = mutableListOf<TextLayoutResult>()
                node.config
                    .getOrElseNullable(SemanticsActions.GetTextLayoutResult) { null }
                    ?.action
                    ?.invoke(layouts)
                layouts.singleOrNull()?.let { it.layoutInput.text.text to it }
            }

    // A2: the overlay spans the keyboard's navigation-bar band; the last row sat under the bar.
    @Test
    fun lastRow_scrolledToEnd_staysAboveTheNavigationBarInset() {
        val height = 300.dp
        val words = (0 until 60).map { TaigiWord(it, "tsit$it", "字$it") }
        setGrid(words, height)
        rule.onNode(hasScrollToIndexAction()).performScrollToNode(hasText("字59"))
        rule.waitForIdle()
        val lastCell = rule.onNodeWithText("字59", useUnmergedTree = true).getBoundsInRoot()
        assertTrue(
            "last row bottom ${lastCell.bottom} must clear the $navBarInset inset of $height",
            lastCell.bottom <= height - navBarInset,
        )
    }

    // A5: Hanji titles were measured at the subtitle size by a Paint (no M3 letter spacing), so
    // a packed row rendered them wider than their cell and ellipsized "子仔" to "子…".
    @Test
    fun hanjiTitles_fitTheirCells_atFontScale2() {
        val hanji = listOf("子仔", "阿里山", "火金蛄", "十一點", "在野黨", "囡仔", "鉸刀", "菜頭粿")
        val words = (0 until 48).map { TaigiWord(it, "a", hanji[it % hanji.size] + "字".repeat(it % 3)) }
        setGrid(words, height = 2000.dp, fontScale = 2f)
        val layouts = visibleTextLayouts()
        assertTrue("grid rendered no text", layouts.size > 8)
        for ((text, layout) in layouts) {
            assertFalse("\"$text\" is ellipsized", layout.isLineEllipsized(0))
        }
    }

    // A5: rows were packed for the screen width; beside a landscape navigation bar the IME
    // window is narrower, so a row overran and its last cell collapsed to "…".
    @Test
    fun cells_fitAnOverlayNarrowerThanTheScreen() {
        val words = (0 until 40).map { TaigiWord(it, "tai$it", "台$it") }
        setGrid(words, height = 600.dp, width = 320.dp)
        val layouts = visibleTextLayouts()
        assertTrue("grid rendered no text", layouts.size > 8)
        for ((text, layout) in layouts) {
            assertFalse("\"$text\" is ellipsized", layout.isLineEllipsized(0))
        }
    }
}
