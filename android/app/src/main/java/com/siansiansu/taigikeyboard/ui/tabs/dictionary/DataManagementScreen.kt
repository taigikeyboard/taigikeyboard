package com.siansiansu.taigikeyboard.ui.tabs.dictionary

import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.siansiansu.taigikeyboard.localization.DictionaryTexts
import com.siansiansu.taigikeyboard.ui.components.ActionRow
import com.siansiansu.taigikeyboard.ui.components.FileDownload
import com.siansiansu.taigikeyboard.ui.components.FileUpload
import com.siansiansu.taigikeyboard.ui.components.LoadingRow
import com.siansiansu.taigikeyboard.ui.components.SettingsCard
import com.siansiansu.taigikeyboard.ui.components.SettingsDivider

// Backup and restore screen for user data
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DataManagementScreen(
    isProcessing: Boolean = false,
    onNavigateBack: () -> Unit,
    onExportBackup: () -> Unit,
    onImportBackup: () -> Unit,
) {
    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Text(
                        text = DictionaryTexts.backupRestore,
                        fontWeight = FontWeight.Bold,
                    )
                },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = null)
                    }
                },
                colors =
                    TopAppBarDefaults.topAppBarColors(
                        containerColor = MaterialTheme.colorScheme.surfaceContainer,
                    ),
            )
        },
        containerColor = MaterialTheme.colorScheme.surfaceContainer,
    ) { padding ->
        LazyColumn(
            modifier =
                Modifier
                    .fillMaxSize()
                    .padding(padding)
                    .padding(horizontal = 20.dp),
        ) {
            // Privacy warning
            item {
                Spacer(Modifier.height(8.dp))
                SettingsCard {
                    Text(
                        text = DictionaryTexts.backupPrivacyWarning,
                        color = MaterialTheme.colorScheme.onSurface,
                        modifier = Modifier.padding(horizontal = 20.dp, vertical = 16.dp),
                        style = MaterialTheme.typography.bodyLarge,
                    )
                }
            }

            // Backup/Restore
            item {
                Spacer(Modifier.height(16.dp))

                Text(
                    text = DictionaryTexts.backupRestore,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(start = 16.dp, bottom = 8.dp),
                    style = MaterialTheme.typography.titleMedium,
                )

                SettingsCard {
                    ActionRow(
                        label = DictionaryTexts.exportBackup,
                        onClick = { if (!isProcessing) onExportBackup() },
                        icon = Icons.Outlined.FileUpload,
                        textColor = MaterialTheme.colorScheme.primary,
                    )
                    SettingsDivider()
                    if (isProcessing) {
                        LoadingRow()
                    } else {
                        ActionRow(
                            label = DictionaryTexts.importBackup,
                            onClick = onImportBackup,
                            icon = Icons.Outlined.FileDownload,
                            textColor = MaterialTheme.colorScheme.primary,
                        )
                    }
                }
            }

            item { Spacer(Modifier.height(40.dp)) }
        }
    }
}
