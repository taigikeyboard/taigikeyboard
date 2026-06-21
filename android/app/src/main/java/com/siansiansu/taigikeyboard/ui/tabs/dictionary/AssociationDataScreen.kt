package com.siansiansu.taigikeyboard.ui.tabs.dictionary

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.siansiansu.taigikeyboard.i18n.LocalStringResolver
import com.siansiansu.taigikeyboard.i18n.generated.L10n
import com.siansiansu.taigikeyboard.i18n.generated.StringKey
import com.siansiansu.taigikeyboard.i18n.generated.dictionaryImportResult
import com.siansiansu.taigikeyboard.ui.components.ActionRow
import com.siansiansu.taigikeyboard.ui.components.ConfirmationDialog
import com.siansiansu.taigikeyboard.ui.components.FileDownload
import com.siansiansu.taigikeyboard.ui.components.FileUpload
import com.siansiansu.taigikeyboard.ui.components.FilterSearchBar
import com.siansiansu.taigikeyboard.ui.components.LoadingRow
import com.siansiansu.taigikeyboard.ui.components.ResultDialog
import com.siansiansu.taigikeyboard.ui.components.SettingInfoButton
import com.siansiansu.taigikeyboard.ui.components.SettingsCard
import com.siansiansu.taigikeyboard.ui.components.SettingsDivider
import com.siansiansu.taigikeyboard.ui.components.SwitchRow
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

private const val DISPLAY_LIMIT = 100

// Association data management — view, import/export, and clear word association records
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AssociationDataScreen(
    viewModel: AssociationDataViewModel,
    onNavigateBack: () -> Unit,
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    // Captured for the non-Composable export/import callbacks; tracks the live display language.
    val stringResolver by rememberUpdatedState(LocalStringResolver.current)

    val allData by viewModel.allData.collectAsStateWithLifecycle()
    val isImporting by viewModel.isImporting.collectAsStateWithLifecycle()
    val isRecordingEnabled by viewModel.isAssociationRecordingEnabled.collectAsStateWithLifecycle()

    var showClearDialog by remember { mutableStateOf(false) }
    var showResultDialog by remember { mutableStateOf(false) }
    var resultMessage by remember { mutableStateOf("") }
    var filterText by remember { mutableStateOf("") }

    val filteredData =
        if (filterText.isEmpty()) {
            allData.take(DISPLAY_LIMIT)
        } else {
            val query = filterText.lowercase()
            allData.filter {
                it.prevWord.lowercase().contains(query) ||
                    it.prevTl.lowercase().contains(query) ||
                    it.nextWord.lowercase().contains(query) ||
                    it.nextTl.lowercase().contains(query)
            }
        }

    LaunchedEffect(Unit) { viewModel.load() }

    val exportLauncher =
        rememberLauncherForActivityResult(
            contract = ActivityResultContracts.CreateDocument("text/csv"),
        ) { uri: Uri? ->
            uri ?: return@rememberLauncherForActivityResult
            scope.launch {
                try {
                    val csv = viewModel.exportCSV()
                    withContext(Dispatchers.IO) {
                        context.contentResolver.openOutputStream(uri)?.use {
                            it.write(csv.toByteArray(Charsets.UTF_8))
                        }
                    }
                    resultMessage = stringResolver.resolve(StringKey.DICTIONARY_EXPORT_SUCCESS)
                    showResultDialog = true
                } catch (e: Exception) {
                    resultMessage = e.localizedMessage ?: stringResolver.resolve(StringKey.COMMON_EXPORT_FAILED)
                    showResultDialog = true
                }
            }
        }

    val importLauncher =
        rememberLauncherForActivityResult(
            contract = ActivityResultContracts.OpenDocument(),
        ) { uri: Uri? ->
            uri ?: return@rememberLauncherForActivityResult
            scope.launch {
                try {
                    val outcome = viewModel.importCSV(uri)
                    resultMessage =
                        stringResolver.dictionaryImportResult(
                            imported = outcome.imported,
                            skipped = outcome.skipped,
                        )
                    showResultDialog = true
                } catch (e: Exception) {
                    resultMessage = e.localizedMessage ?: stringResolver.resolve(StringKey.COMMON_IMPORT_FAILED)
                    showResultDialog = true
                }
            }
        }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Text(
                        text = L10n.dictionaryAssociationManagement,
                        fontWeight = FontWeight.Bold,
                    )
                },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = L10n.commonBack)
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
        Column(
            modifier =
                Modifier
                    .fillMaxSize()
                    .padding(padding),
        ) {
            LazyColumn(
                modifier =
                    Modifier
                        .weight(1f)
                        .padding(horizontal = 20.dp),
            ) {
                // Toggle
                item {
                    Spacer(Modifier.height(8.dp))
                    SettingsCard {
                        SwitchRow(
                            label = L10n.dictionaryAssociationRecordingEnabled,
                            checked = isRecordingEnabled,
                            infoText = L10n.dictionaryAssociationRecordingEnabledInfo,
                            onCheckedChange = { viewModel.setRecordingEnabled(it) },
                        )
                    }
                }

                // Import/Export
                item {
                    Spacer(Modifier.height(16.dp))
                    Text(
                        text = L10n.dictionaryImportExportTitle,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(start = 16.dp, bottom = 8.dp),
                        style = MaterialTheme.typography.titleMedium,
                    )
                    SettingsCard {
                        Text(
                            text = L10n.dictionaryAssociationDescription,
                            color = MaterialTheme.colorScheme.onSurface,
                            style = MaterialTheme.typography.bodyLarge,
                            modifier = Modifier.padding(horizontal = 20.dp, vertical = 16.dp),
                        )
                        SettingsDivider()
                        ActionRow(
                            label = L10n.dictionaryAssociationExportCSV,
                            onClick = {
                                if (!isImporting) {
                                    val dateStr = SimpleDateFormat("yyyy-MM-dd", Locale.US).format(Date())
                                    exportLauncher.launch("詞關聯紀錄_$dateStr.csv")
                                }
                            },
                            icon = Icons.Outlined.FileUpload,
                            textColor = MaterialTheme.colorScheme.primary,
                        )
                        SettingsDivider()
                        if (isImporting) {
                            LoadingRow()
                        } else {
                            ActionRow(
                                label = L10n.dictionaryAssociationImportCSV,
                                onClick = { importLauncher.launch(arrayOf("text/*")) },
                                icon = Icons.Outlined.FileDownload,
                                textColor = MaterialTheme.colorScheme.primary,
                            )
                        }
                    }
                }

                // Clear button
                item {
                    Spacer(Modifier.height(16.dp))
                    SettingsCard {
                        ActionRow(
                            label = L10n.dictionaryClearAllAssociation,
                            onClick = { if (!isImporting) showClearDialog = true },
                            textColor = MaterialTheme.colorScheme.error,
                        )
                    }
                }

                // Privacy warning
                item {
                    Spacer(Modifier.height(16.dp))
                    SettingsCard {
                        Text(
                            text = L10n.dictionaryAssociationPrivacyWarning,
                            style = MaterialTheme.typography.bodyLarge,
                            color = MaterialTheme.colorScheme.onSurface,
                            modifier = Modifier.padding(horizontal = 20.dp, vertical = 16.dp),
                        )
                    }
                }

                // Data list
                item {
                    Spacer(Modifier.height(16.dp))
                    Row(
                        modifier = Modifier.padding(start = 16.dp, bottom = 8.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(
                            text = L10n.dictionaryAssociationManagement,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            style = MaterialTheme.typography.titleMedium,
                        )
                        Spacer(Modifier.width(6.dp))
                        SettingInfoButton(description = L10n.dictionaryFilterHint)
                    }
                }
                if (allData.isEmpty()) {
                    item {
                        SettingsCard {
                            Row(
                                modifier =
                                    Modifier
                                        .fillMaxWidth()
                                        .padding(horizontal = 20.dp, vertical = 16.dp),
                            ) {
                                Text(
                                    text = L10n.dictionaryNoData,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                        }
                    }
                } else if (filterText.isNotEmpty() && filteredData.isEmpty()) {
                    item {
                        SettingsCard {
                            Text(
                                text = L10n.dictionaryNoResults,
                                style = MaterialTheme.typography.bodyLarge,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                modifier = Modifier.padding(horizontal = 20.dp, vertical = 16.dp),
                            )
                        }
                    }
                } else {
                    itemsIndexed(
                        items = filteredData,
                        key = { _, entry -> "${entry.prevWord}\t${entry.prevTl}\t${entry.nextWord}\t${entry.nextTl}" },
                    ) { index, entry ->
                        Row(
                            modifier =
                                Modifier
                                    .fillMaxWidth()
                                    .background(MaterialTheme.colorScheme.surface)
                                    .padding(start = 20.dp, end = 4.dp),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            val prev = if (entry.prevTl.isEmpty()) entry.prevWord else "(${entry.prevTl}, ${entry.prevWord})"
                            val next = if (entry.nextTl.isEmpty()) entry.nextWord else "(${entry.nextTl}, ${entry.nextWord})"
                            Text(
                                text = "$prev → $next",
                                modifier = Modifier.weight(1f),
                                style = MaterialTheme.typography.bodyLarge,
                            )
                            Text(
                                text = "${entry.count}",
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                style = MaterialTheme.typography.labelLarge,
                            )
                            IconButton(
                                onClick = { viewModel.delete(entry) },
                            ) {
                                Icon(
                                    imageVector = Icons.Default.Delete,
                                    contentDescription = L10n.commonDelete,
                                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                        }
                        if (index < filteredData.lastIndex) {
                            HorizontalDivider(
                                modifier = Modifier.padding(horizontal = 20.dp),
                                color = MaterialTheme.colorScheme.outlineVariant,
                            )
                        }
                    }
                }

                item { Spacer(Modifier.height(40.dp)) }
            }

            // Filter (anchored at bottom)
            FilterSearchBar(
                value = filterText,
                onValueChange = { filterText = it },
                placeholder = L10n.dictionarySearchPlaceholder,
            )
        }
    }

    if (showClearDialog) {
        ConfirmationDialog(
            title = L10n.dictionaryClearAllAssociation,
            message = L10n.dictionaryClearAssociationMessage,
            confirmLabel = L10n.dictionaryClear,
            dismissLabel = L10n.commonCancel,
            onConfirm = {
                showClearDialog = false
                viewModel.clearAll()
            },
            onDismiss = { showClearDialog = false },
        )
    }

    if (showResultDialog) {
        ResultDialog(
            message = resultMessage,
            confirmLabel = L10n.commonOk,
            onDismiss = { showResultDialog = false },
        )
    }
}
