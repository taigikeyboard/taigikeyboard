package com.siansiansu.taigikeyboard.ime.text.overlays

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.test.getBoundsInRoot
import androidx.compose.ui.test.hasScrollToIndexAction
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performScrollToNode
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.test.ext.junit.runners.AndroidJUnit4
import com.siansiansu.taigikeyboard.i18n.DisplayLanguage
import com.siansiansu.taigikeyboard.i18n.ProvideDisplayLanguage
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/** Layout audit: the symbol picker stays inside the overlay. */
@RunWith(AndroidJUnit4::class)
class SymbolOverlayLayoutTest {
    @get:Rule
    val rule = createComposeRule()

    private val overlayHeight = 260.dp
    private val navBarInset = 48.dp

    private val appearance =
        KeyboardOverlayAppearance(
            solidBackground = Color.White,
            surface = null,
            foreground = Color.Black,
            accent = Color.Blue,
            calloutFill = Color.White,
            calloutForeground = Color.Black,
            keyFill = null,
            lineColor = null,
        )

    private fun setPicker(
        language: DisplayLanguage = DisplayLanguage.ENGLISH,
        width: Dp = 400.dp,
    ) {
        rule.setContent {
            ProvideDisplayLanguage(language) {
                Box(Modifier.size(width, overlayHeight)) {
                    SymbolOverlayContent(
                        appearance = appearance,
                        resetKey = 0,
                        onSymbolSelected = {},
                        bottomInset = navBarInset,
                    )
                }
            }
        }
    }

    // A3: the overlay spans the keyboard's navigation-bar band; the last row sat under the bar.
    @Test
    fun lastSymbolRow_scrolledToEnd_staysAboveTheNavigationBarInset() {
        setPicker()
        val last = SymbolData.rows(SymbolCategory.FULL_WIDTH).flatten().last()
        rule.onNode(hasScrollToIndexAction()).performScrollToNode(hasText(last))
        rule.waitForIdle()
        val bounds = rule.onNodeWithText(last, useUnmergedTree = true).getBoundsInRoot()
        assertTrue(
            "last symbol bottom ${bounds.bottom} must clear the $navBarInset inset of $overlayHeight",
            bounds.bottom <= overlayHeight - navBarInset,
        )
    }
}
