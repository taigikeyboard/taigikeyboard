package com.siansiansu.taigikeyboard.ime.text.overlays

import android.graphics.Typeface
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.getBoundsInRoot
import androidx.compose.ui.test.hasScrollToIndexAction
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.performScrollToNode
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.test.ext.junit.runners.AndroidJUnit4
import com.siansiansu.taigikeyboard.i18n.DisplayLanguage
import com.siansiansu.taigikeyboard.i18n.ProvideDisplayLanguage
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord
import com.siansiansu.taigikeyboard.ime.settings.CandidateDisplayMode
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/** Layout audit: the expanded candidate grid stays inside the overlay and its own cells. */
@RunWith(AndroidJUnit4::class)
class CandidateOverlayLayoutTest {
    @get:Rule
    val rule = createComposeRule()

    private val overlayHeight = 300.dp
    private val navBarInset = 48.dp

    private fun setGrid(
        words: List<TaigiWord>,
        bottomInset: Dp = navBarInset,
        isHanjiFirst: Boolean = true,
    ) {
        rule.setContent {
            ProvideDisplayLanguage(DisplayLanguage.ENGLISH) {
                Box(Modifier.size(400.dp, overlayHeight)) {
                CandidateOverlayContent(
                    suggestions = words,
                    typeface = Typeface.DEFAULT,
                    isTPSLayout = false,
                    isHanjiFirst = isHanjiFirst,
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
                    bottomInset = bottomInset,
                )
                }
            }
        }
    }

    // A2: the overlay spans the keyboard's navigation-bar band; the last row sat under the bar.
    @Test
    fun lastRow_scrolledToEnd_staysAboveTheNavigationBarInset() {
        val words = (0 until 60).map { TaigiWord(it, "tsit$it", "字$it") }
        setGrid(words)
        rule.onNode(hasScrollToIndexAction()).performScrollToNode(hasText("字59"))
        rule.waitForIdle()
        val lastCell = rule.onNodeWithText("字59", useUnmergedTree = true).getBoundsInRoot()
        assertTrue(
            "last row bottom ${lastCell.bottom} must clear the ${navBarInset} inset of $overlayHeight",
            lastCell.bottom <= overlayHeight - navBarInset,
        )
    }
}
