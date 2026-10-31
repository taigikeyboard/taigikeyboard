package com.siansiansu.taigikeyboard.ui.tabs.layout

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInWindow
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.siansiansu.taigikeyboard.i18n.generated.L10n
import com.siansiansu.taigikeyboard.ime.text.layout.CustomLayout
import com.siansiansu.taigikeyboard.ime.text.layout.dashedOutline
import com.siansiansu.taigikeyboard.ui.theme.AppStyle

// The selected key's long-press list, drawn as the keyboard's callout bubble in a fixed slot below the
// board, its tail under the key. Mirrors iOS App/Tabs/Layout/LayoutCalloutStrip.swift.

/**
 * Always [CustomLayout.MAX_CALLOUTS] slots: [entries], then empty dashed slots (all dashed while
 * [entries] is null — no character key selected). Slots are edited like board keys, by tapping: the
 * editor decides what a tap does. [tailX] is the selected key's centre in window coordinates.
 */
@Composable
fun LayoutCalloutStrip(
    entries: List<String>?,
    selectedSlot: Int?,
    canReset: Boolean,
    tailX: Float?,
    onTapSlot: (Int) -> Unit,
    onReset: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Row(modifier, verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(4.dp)) {
        Bubble(entries, selectedSlot, tailX, onTapSlot, Modifier.weight(1f))
        IconButton(onClick = onReset, enabled = canReset) {
            Icon(Icons.Filled.Refresh, contentDescription = L10n.themeEditorResetAll, modifier = Modifier.size(AppStyle.selectionIconSize))
        }
    }
}

@Composable
private fun Bubble(
    entries: List<String>?,
    selectedSlot: Int?,
    tailX: Float?,
    onTapSlot: (Int) -> Unit,
    modifier: Modifier,
) {
    val color = MaterialTheme.colorScheme.surface
    var bubbleX by remember { mutableFloatStateOf(0f) }
    val title = L10n.layoutCalloutsTitle
    Row(
        modifier
            .onGloballyPositioned { bubbleX = it.positionInWindow().x }
            // A triangle on the top edge, pointing up at the selected key.
            .drawBehind {
                if (tailX != null && entries != null) {
                    val half = 8.dp.toPx()
                    val x = (tailX - bubbleX).coerceIn(2 * half, size.width - 2 * half)
                    val tail =
                        Path().apply {
                            moveTo(x - half, 0f)
                            lineTo(x, -half)
                            lineTo(x + half, 0f)
                            close()
                        }
                    drawPath(tail, color)
                }
            }.shadow(4.dp, RoundedCornerShape(12.dp), clip = false)
            .background(color, RoundedCornerShape(12.dp))
            .padding(6.dp)
            .semantics { contentDescription = title },
        horizontalArrangement = Arrangement.spacedBy(2.dp),
    ) {
        repeat(CustomLayout.MAX_CALLOUTS) { index ->
            Slot(
                entry = entries?.getOrNull(index),
                isSelected = index == selectedSlot,
                enabled = entries != null,
                onTap = { onTapSlot(index) },
                modifier = Modifier.weight(1f),
            )
        }
    }
}

/** An entry, or an empty dashed slot past the list's end; drawn selected in the board's colour. */
@Composable
private fun Slot(
    entry: String?,
    isSelected: Boolean,
    enabled: Boolean,
    onTap: () -> Unit,
    modifier: Modifier,
) {
    val shape = RoundedCornerShape(8.dp)
    val dashColor = MaterialTheme.colorScheme.outline
    val emptyLabel = L10n.settingsSyllableSeparatorNone
    Box(
        modifier
            .height(44.dp)
            .background(if (isSelected) MaterialTheme.colorScheme.primary else Color.Transparent, shape)
            .dashedOutline(dashColor, 8.dp, isVisible = entry == null && !isSelected)
            .clickable(enabled = enabled, onClick = onTap)
            .semantics {
                contentDescription = entry ?: emptyLabel
                role = Role.Button
                selected = isSelected
            },
        contentAlignment = Alignment.Center,
    ) {
        Text(
            entry.orEmpty(),
            fontSize = 20.sp,
            maxLines = 1,
            color = if (isSelected) MaterialTheme.colorScheme.onPrimary else MaterialTheme.colorScheme.onSurface,
        )
    }
}
