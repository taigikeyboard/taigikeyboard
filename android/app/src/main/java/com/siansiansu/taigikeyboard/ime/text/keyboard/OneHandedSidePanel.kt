// Buttons in the gap beside the one-handed docked keys: move the keys to the other edge, or restore full width.

package com.siansiansu.taigikeyboard.ime.text.keyboard

import androidx.annotation.DrawableRes
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.unit.dp
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.i18n.generated.L10n
import com.siansiansu.taigikeyboard.ime.core.settings.OneHandedMode

/** Mirrors iOS `OneHandedSidePanel` (`Overlays/OneHandedModeViews.swift`). */
@Composable
internal fun OneHandedSidePanel(
    mode: OneHandedMode,
    tint: Color,
    onModeSelected: (OneHandedMode) -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(
        modifier = modifier,
        verticalArrangement = Arrangement.spacedBy(24.dp, Alignment.CenterVertically),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        // The chevron points at the gap: that is where the keys move to.
        SidePanelButton(
            iconRes = if (mode == OneHandedMode.LEFT) R.drawable.ic_chevron_right else R.drawable.ic_chevron_left,
            contentDescription = L10n.keyboardOneHandedSwapSide,
            tint = tint,
            onClick = { onModeSelected(mode.flipped) },
        )
        SidePanelButton(
            iconRes = R.drawable.ic_zoom_out_map,
            contentDescription = L10n.keyboardOneHandedOff,
            tint = tint,
            onClick = { onModeSelected(OneHandedMode.OFF) },
        )
    }
}

@Composable
private fun SidePanelButton(
    @DrawableRes iconRes: Int,
    contentDescription: String,
    tint: Color,
    onClick: () -> Unit,
) {
    IconButton(onClick = onClick, modifier = Modifier.size(48.dp)) {
        Icon(painter = painterResource(iconRes), contentDescription = contentDescription, tint = tint)
    }
}
