// Compose content for the keyboard symbol selection overlay — M3 PrimaryTabRow +
// LazyVerticalGrid. Honors the user-customizable keyboard overlay appearance
// (gradient backdrop + role-first foreground + chrome accent).

package com.siansiansu.taigikeyboard.ime.text.smartbar

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.material3.PrimaryTabRow
import androidx.compose.material3.Tab
import androidx.compose.material3.TabRowDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.siansiansu.taigikeyboard.i18n.stringRes

private val SymbolCategoryValues = SymbolCategory.entries
private val SymbolCellMinHeight = 40.dp
private val GridContentPadding = 8.dp

/**
 * Symbol picker: a fixed tab row over the 5 [SymbolCategory] tabs and a lazy grid of the
 * selected category's symbols.
 *
 * @param appearance user keyboard overlay colors — gradient backdrop + role-first foreground +
 *   chrome accent (tab/grid honor these, not the M3 brand palette).
 * @param resetKey bumped on each overlay show() — resets the active tab to FULL_WIDTH,
 *   preserving the legacy View behavior where reopening the overlay starts on the first tab.
 * @param onSymbolSelected invoked with the tapped symbol string.
 */
@Composable
fun SymbolOverlayContent(
    appearance: KeyboardOverlayAppearance,
    resetKey: Int,
    onSymbolSelected: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
    // remember(resetKey) re-initializes to FULL_WIDTH whenever the overlay is reopened
    // (show() bumps the key) — a synchronous reset, no one-frame stale-tab render.
    var selectedCategory by remember(resetKey) { mutableStateOf(SymbolCategory.FULL_WIDTH) }

    val selectedIndex = SymbolCategoryValues.indexOf(selectedCategory)

    // Panel sits below the smartbar; offset the gradient by it so the slice stays continuous.
    val topInsetPx = rememberSmartbarInsetPx()

    Column(
        modifier = modifier
            .fillMaxSize()
            .keyboardOverlayBackdrop(appearance.gradientStops, appearance.solidBackground, topInsetPx),
    ) {
        PrimaryTabRow(
            selectedTabIndex = selectedIndex,
            // Transparent so the column's gradient/solid backdrop shows through uniformly.
            containerColor = Color.Transparent,
            contentColor = appearance.foreground,
            indicator = {
                TabRowDefaults.PrimaryIndicator(
                    modifier = Modifier.tabIndicatorOffset(selectedIndex, matchContentSize = false),
                    color = appearance.accent,
                )
            },
        ) {
            SymbolCategoryValues.forEachIndexed { index, category ->
                Tab(
                    selected = index == selectedIndex,
                    onClick = { selectedCategory = category },
                    selectedContentColor = appearance.accent,
                    unselectedContentColor = appearance.foreground,
                    text = { Text(stringRes(category.labelKey)) },
                )
            }
        }

        val symbols = remember(selectedCategory) { SymbolData.rows(selectedCategory).flatten() }
        LazyVerticalGrid(
            // weight(1f), not fillMaxSize: fill only the space left under the tab row so the
            // grid stays bounded inside the overlay and scrolls internally (fillMaxSize would
            // size it to the full Column height and overflow below the overlay).
            modifier = Modifier
                .fillMaxWidth()
                .weight(1f),
            columns = GridCells.Fixed(selectedCategory.columnCount),
            contentPadding = PaddingValues(GridContentPadding),
        ) {
            items(symbols) { symbol ->
                SymbolCell(
                    symbol = symbol,
                    fontSizeSp = selectedCategory.fontSize,
                    color = appearance.foreground,
                    onClick = { onSymbolSelected(symbol) },
                )
            }
        }
    }
}

@Composable
private fun SymbolCell(
    symbol: String,
    fontSizeSp: Float,
    color: Color,
    onClick: () -> Unit,
) {
    Box(
        modifier = Modifier
            .fillMaxWidth()
            .heightIn(min = SymbolCellMinHeight)
            .clickable(onClick = onClick),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text = symbol,
            color = color,
            fontSize = fontSizeSp.sp,
            textAlign = TextAlign.Center,
        )
    }
}
