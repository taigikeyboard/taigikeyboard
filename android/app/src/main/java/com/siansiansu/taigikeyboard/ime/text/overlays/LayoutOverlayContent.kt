// Compose content for the keyboard layout selection overlay — M3 Surface preview cards in
// horizontally-scrolling sections. Honors the user-customizable keyboard overlay appearance
// (gradient backdrop + role-first foreground + chrome accent).

package com.siansiansu.taigikeyboard.ime.text.overlays

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
import com.siansiansu.taigikeyboard.i18n.generated.L10n
import com.siansiansu.taigikeyboard.i18n.stringRes
import com.siansiansu.taigikeyboard.ime.text.layout.KeyboardLayoutOption
import com.siansiansu.taigikeyboard.ime.text.layout.KeyboardLayoutOptions
import com.siansiansu.taigikeyboard.ime.theme.themeBackground

private val CardWidth = 120.dp
private val CardSpacing = 12.dp
private val CardCornerRadius = 10.dp
private val CheckmarkSize = 36.dp
private val CheckmarkIconSize = 16.dp
private val SelectedBorderWidth = 2.5.dp
private val SectionHorizontalPadding = 12.dp
private const val SelectedScrimAlpha = 0.25f

/**
 * Layout picker: two horizontally-scrolling sections (romanization + phonetic) of preview cards.
 * The selected card shows an accent border + checkmark; tapping a card reports its layout key.
 *
 * @param appearance user keyboard overlay colors — gradient backdrop + role-first foreground +
 *   chrome accent (cards honor these, not the M3 brand palette).
 * @param selectedKey the currently active layout key (re-read from prefs on each overlay show()).
 * @param resetKey bumped on each overlay show() — re-seeds the local selection to [selectedKey].
 * @param onLayoutSelected invoked with the tapped layout key.
 */
@Composable
fun LayoutOverlayContent(
    appearance: KeyboardOverlayAppearance,
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

    // Panel sits below the smartbar; offset the gradient by it so the slice stays continuous.
    val topInsetPx = rememberSmartbarInsetPx()

    Column(
        modifier =
            modifier
                .fillMaxSize()
                .themeBackground(appearance.surface, appearance.solidBackground, topInsetPx)
                .verticalScroll(rememberScrollState())
                .padding(top = 10.dp, bottom = 4.dp),
    ) {
        SectionHeader(L10n.layoutRomanizationKeyboard, appearance.foreground, topPadding = 0.dp)
        LayoutCardRow(KeyboardLayoutOptions.romanization, activeKey, appearance, onSelect)

        SectionHeader(L10n.settingsTpsMode, appearance.foreground, topPadding = 12.dp)
        LayoutCardRow(KeyboardLayoutOptions.phonetic, activeKey, appearance, onSelect)
    }
}

@Composable
private fun SectionHeader(
    text: String,
    color: Color,
    topPadding: Dp,
) {
    KeyboardOverlaySectionHeader(
        text = text,
        color = color,
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
    options: List<KeyboardLayoutOption>,
    activeKey: String,
    appearance: KeyboardOverlayAppearance,
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
                appearance = appearance,
                onClick = { onSelect(option.key) },
            )
        }
    }
}

@Composable
private fun LayoutCard(
    option: KeyboardLayoutOption,
    isSelected: Boolean,
    appearance: KeyboardOverlayAppearance,
    onClick: () -> Unit,
) {
    val label = stringRes(option.labelKey)
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
            border = if (isSelected) BorderStroke(SelectedBorderWidth, appearance.accent) else null,
        ) {
            Box {
                Image(
                    painter = painterResource(option.previewRes),
                    contentDescription = label,
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
                                .background(appearance.accent),
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
            text = label,
            color = appearance.foreground,
            fontSize = 12.sp,
            fontWeight = FontWeight.Bold,
            maxLines = 1,
            textAlign = TextAlign.Center,
        )
    }
}
