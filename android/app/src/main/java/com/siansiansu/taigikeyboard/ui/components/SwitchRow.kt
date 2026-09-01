package com.siansiansu.taigikeyboard.ui.components

import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.selection.toggleable
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp

@Composable
fun SwitchRow(
    label: String,
    checked: Boolean,
    onCheckedChange: (Boolean) -> Unit,
    modifier: Modifier = Modifier,
    icon: ImageVector? = null,
    iconTint: Color = MaterialTheme.colorScheme.primary,
    labelColor: Color = MaterialTheme.colorScheme.onSurface,
    fontFamily: FontFamily? = null,
    infoText: String? = null,
    enabled: Boolean = true,
) {
    // Disabled rows dim label + icon (M3 disabled-content alpha) and drop the row's toggle target.
    val contentAlpha = if (enabled) 1f else DISABLED_CONTENT_ALPHA
    // Whole row is the toggle target (standard Android Settings / Material3 + iOS Form parity).
    // `toggleable` placed before `padding` so the ripple + touch target fill the full row, and it
    // merges descendants into one TalkBack node; `role = Role.Switch` makes that node read as a
    // switch ("label, switch, on/off").
    Row(
        modifier =
            modifier
                .fillMaxWidth()
                .heightIn(min = 48.dp)
                .toggleable(
                    value = checked,
                    enabled = enabled,
                    onValueChange = onCheckedChange,
                    role = Role.Switch,
                ).padding(horizontal = 20.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (icon != null) {
            Icon(
                imageVector = icon,
                contentDescription = null,
                modifier = Modifier.size(24.dp),
                tint = iconTint.copy(alpha = iconTint.alpha * contentAlpha),
            )
            Spacer(Modifier.width(12.dp))
        }
        // Measure the trailing switch first; localized labels wrap within the remaining width.
        Row(
            modifier = Modifier.weight(1f),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                text = label,
                modifier = Modifier.weight(1f, fill = false),
                color = labelColor.copy(alpha = labelColor.alpha * contentAlpha),
                fontFamily = fontFamily,
                style = MaterialTheme.typography.bodyLarge,
            )
            if (infoText != null) {
                Spacer(Modifier.width(6.dp))
                SettingInfoButton(description = infoText)
            }
        }
        // onCheckedChange = null: the row's toggleable owns the toggle, so the Switch is
        // display-only (no duplicate toggle semantics / role conflict with the row).
        Switch(
            checked = checked,
            onCheckedChange = null,
            enabled = enabled,
        )
    }
}

private const val DISABLED_CONTENT_ALPHA = 0.38f
