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
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.siansiansu.taigikeyboard.localization.CommonTexts
import com.siansiansu.taigikeyboard.localization.DictionaryTexts
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

// Frequency data management — view, import/export, and clear word frequency records
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun FrequencyDataScreen(
    viewModel: FrequencyDataViewModel,
    onNavigateBack: () -> Unit,
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()

    val allData by viewModel.allData.collectAsStateWithLifecycle()
    val isImporting by viewModel.isImporting.collectAsStateWithLifecycle()
    val isRecordingEnabled by viewModel.isFrequencyRecordingEnabled.collectAsStateWithLifecycle()

    var showClearDialog by remember { mutableStateOf(false) }
    var showResultDialog by remember { mutableStateOf(false) }
    var resultMessage by remember { mutableStateOf("") }
    var filterText by remember { mutableStateOf("") }

    val filteredData =
        if (filterText.isEmpty()) {
            allData.take(DISPLAY_LIMIT)
        } else {
            val query = filterText.lowercase()
            allData.filter { it.first.lowercase().contains(query) }
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
                    resultMessage = DictionaryTexts.exportSuccess
                    showResultDialog = true
                } catch (e: Exception) {
                    resultMessage = e.localizedMessage ?: CommonTexts.exportFailed
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
                        String.format(
                            DictionaryTexts.frequencyImportResult,
                            outcome.imported,
                            outcome.skipped,
                        )
                    showResultDialog = true
                } catch (e: Exception) {
                    resultMessage = e.localizedMessage ?: CommonTexts.importFailed
                    showResultDialog = true
                }
            }
        }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Text(
                        text = DictionaryTexts.frequencyManagement,
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
                            label = DictionaryTexts.frequencyRecordingEnabled,
                            checked = isRecordingEnabled,
                            infoText = DictionaryTexts.frequencyRecordingEnabledInfo,
                            onCheckedChange = { viewModel.setRecordingEnabled(it) },
                        )
                    }
                }

                // Import/Export
                item {
                    Spacer(Modifier.height(16.dp))
                    Text(
                        text = DictionaryTexts.importExportTitle,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(start = 16.dp, bottom = 8.dp),
                        style = MaterialTheme.typography.titleMedium,
                    )
                    SettingsCard {
                        Text(
                            text = DictionaryTexts.frequencyDescription,
                            color = MaterialTheme.colorScheme.onSurface,
                            style = MaterialTheme.typography.bodyLarge,
                            modifier = Modifier.padding(horizontal = 20.dp, vertical = 16.dp),
                        )
                        SettingsDivider()
                        ActionRow(
                            label = DictionaryTexts.frequencyExportCSV,
                            onClick = {
                                if (!isImporting) {
                                    val dateStr = SimpleDateFormat("yyyy-MM-dd", Locale.US).format(Date())
                                    exportLauncher.launch("詞頻紀錄_$dateStr.csv")
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
                                label = DictionaryTexts.frequencyImportCSV,
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
                            label = DictionaryTexts.clearAllFrequency,
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
                            text = DictionaryTexts.frequencyPrivacyWarning,
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
                            text = DictionaryTexts.frequencyManagement,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            style = MaterialTheme.typography.titleMedium,
                        )
                        Spacer(Modifier.width(6.dp))
                        SettingInfoButton(description = DictionaryTexts.filterHint)
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
                                    text = DictionaryTexts.noData,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                        }
                    }
                } else if (filterText.isNotEmpty() && filteredData.isEmpty()) {
                    item {
                        SettingsCard {
                            Text(
                                text = DictionaryTexts.noResults,
                                style = MaterialTheme.typography.bodyLarge,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                modifier = Modifier.padding(horizontal = 20.dp, vertical = 16.dp),
                            )
                        }
                    }
                } else {
                    itemsIndexed(
                        items = filteredData,
                        key = { _, (word, _) -> word },
                    ) { index, (word, count) ->
                        Row(
                            modifier =
                                Modifier
                                    .fillMaxWidth()
                                    .background(MaterialTheme.colorScheme.surface)
                                    .padding(start = 20.dp, end = 4.dp),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Text(
                                text = word,
                                modifier = Modifier.weight(1f),
                                style = MaterialTheme.typography.bodyLarge,
                            )
                            Text(
                                text = "$count",
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                style = MaterialTheme.typography.labelLarge,
                            )
                            IconButton(
                                onClick = { viewModel.deleteWord(word) },
                            ) {
                                Icon(
                                    imageVector = Icons.Default.Delete,
                                    contentDescription = DictionaryTexts.delete,
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
                placeholder = DictionaryTexts.searchPlaceholder,
            )
        }
    }

    if (showClearDialog) {
        ConfirmationDialog(
            title = DictionaryTexts.clearAllFrequency,
            message = DictionaryTexts.clearFrequencyMessage,
            confirmLabel = DictionaryTexts.clear,
            dismissLabel = CommonTexts.cancel,
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
            confirmLabel = CommonTexts.ok,
            onDismiss = { showResultDialog = false },
        )
    }
}
