package com.siansiansu.taigikeyboard.ui.components

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

/** A failed read, said in place; tapping it asks again. */
@Composable
fun RetryCard(
    text: String,
    onRetry: () -> Unit,
) {
    SettingsCard {
        Row(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .clickable(onClick = onRetry)
                    .padding(horizontal = 20.dp, vertical = 16.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                text = text,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.weight(1f),
                style = MaterialTheme.typography.bodyLarge,
            )
            Icon(
                imageVector = Icons.Default.Refresh,
                contentDescription = null,
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}
