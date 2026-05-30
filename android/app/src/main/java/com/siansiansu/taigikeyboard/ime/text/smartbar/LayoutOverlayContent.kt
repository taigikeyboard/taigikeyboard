// Compose content for the keyboard layout selection overlay — M3 Surface preview cards in
// horizontally-scrolling sections. Honors user-customizable keyboard chrome colors (KeyboardChromeColors).

package com.siansiansu.taigikeyboard.ime.text.smartbar

import androidx.annotation.DrawableRes
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Check
import androidx.compose.material3.Icon
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.localization.LayoutTexts

private data class LayoutOption(
    val key: String,
    val label: String,
    @DrawableRes val previewRes: Int,
)

// Section 1 — romanization keyboards (matches iOS LayoutSelectionOverlay).
private val RomanizationLayouts =
    listOf(
        LayoutOption("phahTaigi", LayoutTexts.phahTaigiLayout, R.drawable.layout_phahtaigi_preview),
        LayoutOption("qwerty", LayoutTexts.standardLayout, R.drawable.layout_standard_preview),
        LayoutOption("moe1", LayoutTexts.moe1Layout, R.drawable.layout_moe1_preview),
        LayoutOption("moe2", LayoutTexts.moe2Layout, R.drawable.layout_moe2_preview),
    )

// Section 2 — Taigi phonetic keyboards.
private val PhoneticLayouts =
    listOf(
        LayoutOption("tps", LayoutTexts.tpsLayout, R.drawable.layout_tps_preview),
    )

private val CardWidth = 120.dp
private val CardSpacing = 12.dp
private val CardCornerRadius = 10.dp
private val CheckmarkSize = 36.dp
private val CheckmarkIconSize = 16.dp
private val SelectedBorderWidth = 2.5.dp
private val SectionHorizontalPadding = 12.dp
private const val SectionHeaderAlpha = 0.6f
private const val SelectedScrimAlpha = 0.25f

/**
 * Layout picker: two horizontally-scrolling sections (romanization + phonetic) of preview cards.
 * The selected card shows an accent border + checkmark; tapping a card reports its layout key.
 *
 * @param chromeColors user keyboard colors (cards honor these, not the M3 brand palette).
 * @param selectedKey the currently active layout key (re-read from prefs on each overlay show()).
 * @param resetKey bumped on each overlay show() — re-seeds the local selection to [selectedKey].
 * @param onLayoutSelected invoked with the tapped layout key.
 */
@Composable
fun LayoutOverlayContent(
    chromeColors: KeyboardChromeColors,
    selectedKey: String,
    resetKey: Int,
    onLayoutSelected: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
    // Key on resetKey (monotonic per show()), NOT selectedKey: the composition survives hide/show,
    // and selectedKey can return to a PRIOR value (e.g. TPS cascade restoring the pre-TPS layout),
    // so value-keying would skip the re-seed and leave a stale checkmark. resetKey forces re-seed
    // on every reopen without a DataStore round-trip. Mirrors SymbolOverlayContent.
    var activeKey by remember(resetKey) { mutableStateOf(selectedKey) }

    val onSelect: (String) -> Unit = { key ->
        activeKey = key
        onLayoutSelected(key)
    }

    Column(
        modifier =
            modifier
                .fillMaxSize()
                .background(chromeColors.background)
                .verticalScroll(rememberScrollState())
                .padding(top = 10.dp, bottom = 4.dp),
    ) {
        SectionHeader(LayoutTexts.romanizationKeyboard, chromeColors.foreground, topPadding = 0.dp)
        LayoutCardRow(RomanizationLayouts, activeKey, chromeColors, onSelect)

        SectionHeader(LayoutTexts.taigiPhonetic, chromeColors.foreground, topPadding = 12.dp)
        LayoutCardRow(PhoneticLayouts, activeKey, chromeColors, onSelect)
    }
}

@Composable
private fun SectionHeader(
    text: String,
    color: Color,
    topPadding: Dp,
) {
    Text(
        text = text,
        color = color.copy(alpha = SectionHeaderAlpha),
        fontSize = 11.sp,
        fontWeight = FontWeight.Bold,
        modifier =
            Modifier.padding(
                start = SectionHorizontalPadding,
                top = topPadding,
                bottom = 6.dp,
            ),
    )
}

@Composable
private fun LayoutCardRow(
    options: List<LayoutOption>,
    activeKey: String,
    chromeColors: KeyboardChromeColors,
    onSelect: (String) -> Unit,
) {
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .horizontalScroll(rememberScrollState())
                .padding(horizontal = SectionHorizontalPadding),
        horizontalArrangement = Arrangement.spacedBy(CardSpacing),
    ) {
        options.forEach { option ->
            LayoutCard(
                option = option,
                isSelected = option.key == activeKey,
                chromeColors = chromeColors,
                onClick = { onSelect(option.key) },
            )
        }
    }
}

@Composable
private fun LayoutCard(
    option: LayoutOption,
    isSelected: Boolean,
    chromeColors: KeyboardChromeColors,
    onClick: () -> Unit,
) {
    Column(
        modifier =
            Modifier
                .width(CardWidth)
                .clickable(onClick = onClick),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        // Surface clips children to its shape, so the scrim/checkmark inherit the rounded corners.
        Surface(
            shape = RoundedCornerShape(CardCornerRadius),
            color = Color.Transparent,
            border = if (isSelected) BorderStroke(SelectedBorderWidth, chromeColors.accent) else null,
        ) {
            Box {
                Image(
                    painter = painterResource(option.previewRes),
                    contentDescription = option.label,
                    // FillWidth mirrors the legacy ImageView FIT_CENTER + adjustViewBounds: pin
                    // width to the card, let height follow the preview's aspect ratio.
                    modifier = Modifier.fillMaxWidth(),
                    contentScale = ContentScale.FillWidth,
                )
                if (isSelected) {
                    Box(
                        modifier =
                            Modifier
                                .matchParentSize()
                                .background(Color.Black.copy(alpha = SelectedScrimAlpha)),
                    )
                    Box(
                        modifier =
                            Modifier
                                .align(Alignment.Center)
                                .size(CheckmarkSize)
                                .clip(CircleShape)
                                .background(chromeColors.accent),
                        contentAlignment = Alignment.Center,
                    ) {
                        Icon(
                            imageVector = Icons.Default.Check,
                            contentDescription = null,
                            tint = Color.White,
                            modifier = Modifier.size(CheckmarkIconSize),
                        )
                    }
                }
            }
        }

        Spacer(Modifier.height(6.dp))

        Text(
            text = option.label,
            color = chromeColors.foreground,
            fontSize = 12.sp,
            fontWeight = FontWeight.Bold,
            maxLines = 1,
            textAlign = TextAlign.Center,
        )
    }
}
