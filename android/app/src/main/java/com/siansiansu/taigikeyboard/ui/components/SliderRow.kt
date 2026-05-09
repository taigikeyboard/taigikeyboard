package com.siansiansu.taigikeyboard.ui.components

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.size
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Slider
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import com.siansiansu.taigikeyboard.ui.theme.AppStyle

@Composable
fun SliderRow(
    label: String,
    value: Float,
    valueFrom: Float,
    valueTo: Float,
    stepSize: Float,
    onValueChange: (Float) -> Unit,
    modifier: Modifier = Modifier,
    defaultValue: Float? = null,
    onValueChangeFinished: (() -> Unit)? = null,
) {
    Column(modifier = modifier.fillMaxWidth()) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                text = label,
                color = MaterialTheme.colorScheme.onSurface,
                modifier = Modifier.weight(1f),
                style = MaterialTheme.typography.bodyLarge,
            )
            if (defaultValue != null && value != defaultValue) {
                Icon(
                    imageVector = Icons.Default.Refresh,
                    contentDescription = "Reset",
                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier =
                        Modifier
                            .size(AppStyle.selectionIconSize)
                            .clickable { onValueChange(defaultValue) },
                )
            }
        }
        Slider(
            value = value.coerceIn(valueFrom, valueTo),
            onValueChange = onValueChange,
            onValueChangeFinished = onValueChangeFinished,
            valueRange = valueFrom..valueTo,
            steps = ((valueTo - valueFrom) / stepSize).toInt() - 1,
            modifier = Modifier.fillMaxWidth(),
        )
    }
}
