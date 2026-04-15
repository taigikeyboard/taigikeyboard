package com.siansiansu.taigikeyboard.ui.tabs.tab3

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
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Clear
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.text.composing.UserFrequencyService
import com.siansiansu.taigikeyboard.localization.CommonTexts
import com.siansiansu.taigikeyboard.localization.Tab3Texts
import com.siansiansu.taigikeyboard.ui.components.ActionRow
import com.siansiansu.taigikeyboard.ui.components.ConfirmationDialog
import com.siansiansu.taigikeyboard.ui.components.FileDownload
import com.siansiansu.taigikeyboard.ui.components.FileUpload
import com.siansiansu.taigikeyboard.ui.components.LoadingRow
import com.siansiansu.taigikeyboard.ui.components.ResultDialog
import com.siansiansu.taigikeyboard.ui.components.SettingInfoButton
import com.siansiansu.taigikeyboard.ui.components.SettingsCard
import com.siansiansu.taigikeyboard.ui.components.SettingsDivider
import com.siansiansu.taigikeyboard.ui.components.SwitchRow
import com.siansiansu.taigikeyboard.util.CsvUtils
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
    prefs: PrefHelper,
    onNavigateBack: () -> Unit,
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()

    var allData by remember { mutableStateOf<List<Pair<String, Int>>>(emptyList()) }
    var showClearDialog by remember { mutableStateOf(false) }
    var isImporting by remember { mutableStateOf(false) }
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

    LaunchedEffect(Unit) {
        withContext(Dispatchers.IO) {
            allData = UserFrequencyService.getAllFrequencies(context)
        }
    }

    val exportLauncher =
        rememberLauncherForActivityResult(
            contract = ActivityResultContracts.CreateDocument("text/csv"),
        ) { uri: Uri? ->
            uri ?: return@rememberLauncherForActivityResult
            scope.launch {
                try {
                    val exportData =
                        withContext(Dispatchers.IO) {
                            UserFrequencyService.getAllFrequencies(context)
                        }
                    val csv =
                        buildString {
                            for ((word, count) in exportData) {
                                append("${CsvUtils.escape(word)},$count\n")
                            }
                        }
                    withContext(Dispatchers.IO) {
                        context.contentResolver.openOutputStream(uri)?.use {
                            it.write(csv.toByteArray(Charsets.UTF_8))
                        }
                    }
                    resultMessage = Tab3Texts.exportSuccess
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
            isImporting = true
            scope.launch {
                try {
                    val csvString =
                        withContext(Dispatchers.IO) {
                            context.contentResolver.openInputStream(uri)?.use {
                                it.bufferedReader(Charsets.UTF_8).readText()
                            } ?: throw Exception("Cannot read file")
                        }
                    val entries = parseFrequencyCSV(csvString)
                    val imported =
                        withContext(Dispatchers.IO) {
                            UserFrequencyService.batchImportMerge(context, entries)
                        }
                    val skipped = entries.size - imported
                    resultMessage =
                        String.format(
                            Tab3Texts.frequencyImportResult,
                            imported,
                            skipped,
                        )
                    showResultDialog = true
                    // Reload data
                    withContext(Dispatchers.IO) {
                        allData = UserFrequencyService.getAllFrequencies(context)
                    }
                } catch (e: Exception) {
                    resultMessage = e.localizedMessage ?: CommonTexts.importFailed
                    showResultDialog = true
                } finally {
                    isImporting = false
                }
            }
        }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Text(
                        text = Tab3Texts.frequencyManagement,
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
                            label = Tab3Texts.frequencyRecordingEnabled,
                            checked = prefs.frequencyRecordingEnabled,
                            infoText = Tab3Texts.frequencyRecordingEnabledInfo,
                            onCheckedChange = { prefs.frequencyRecordingEnabled = it },
                        )
                    }
                }

                // Import/Export
                item {
                    Spacer(Modifier.height(16.dp))
                    Text(
                        text = Tab3Texts.importExportTitle,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(start = 16.dp, bottom = 8.dp),
                        style = MaterialTheme.typography.titleMedium,
                    )
                    SettingsCard {
                        Text(
                            text = Tab3Texts.frequencyDescription,
                            color = MaterialTheme.colorScheme.onSurface,
                            style = MaterialTheme.typography.bodyLarge,
                            modifier = Modifier.padding(horizontal = 20.dp, vertical = 16.dp),
                        )
                        SettingsDivider()
                        ActionRow(
                            label = Tab3Texts.frequencyExportCSV,
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
                                label = Tab3Texts.frequencyImportCSV,
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
                            label = Tab3Texts.clearAllFrequency,
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
                            text = Tab3Texts.frequencyPrivacyWarning,
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
                            text = Tab3Texts.frequencyManagement,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            style = MaterialTheme.typography.titleMedium,
                        )
                        Spacer(Modifier.width(6.dp))
                        SettingInfoButton(description = Tab3Texts.filterHint)
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
                                    text = Tab3Texts.noData,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                        }
                    }
                } else if (filterText.isNotEmpty() && filteredData.isEmpty()) {
                    item {
                        SettingsCard {
                            Text(
                                text = Tab3Texts.noResults,
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
                                onClick = {
                                    scope.launch {
                                        withContext(Dispatchers.IO) {
                                            UserFrequencyService.deleteWord(context, word)
                                        }
                                        allData = allData.filter { it.first != word }
                                    }
                                },
                            ) {
                                Icon(
                                    imageVector = Icons.Default.Delete,
                                    contentDescription = Tab3Texts.delete,
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
            SettingsCard(
                modifier =
                    Modifier
                        .padding(horizontal = 20.dp)
                        .padding(top = 8.dp, bottom = 8.dp),
            ) {
                OutlinedTextField(
                    value = filterText,
                    onValueChange = { filterText = it },
                    modifier = Modifier.fillMaxWidth(),
                    placeholder = {
                        Text(Tab3Texts.searchPlaceholder)
                    },
                    leadingIcon = {
                        Icon(
                            Icons.Default.Search,
                            contentDescription = null,
                            tint = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    },
                    trailingIcon = {
                        if (filterText.isNotEmpty()) {
                            IconButton(onClick = { filterText = "" }) {
                                Icon(
                                    Icons.Default.Clear,
                                    contentDescription = null,
                                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                        }
                    },
                    colors =
                        OutlinedTextFieldDefaults.colors(
                            focusedBorderColor = MaterialTheme.colorScheme.primary,
                            unfocusedBorderColor = Color.Transparent,
                            focusedContainerColor = MaterialTheme.colorScheme.surface,
                            unfocusedContainerColor = MaterialTheme.colorScheme.surface,
                        ),
                    shape = RoundedCornerShape(16.dp),
                    singleLine = true,
                )
            }
        }
    }

    if (showClearDialog) {
        ConfirmationDialog(
            title = Tab3Texts.clearAllFrequency,
            message = Tab3Texts.clearFrequencyMessage,
            confirmLabel = Tab3Texts.clear,
            dismissLabel = CommonTexts.cancel,
            onConfirm = {
                showClearDialog = false
                scope.launch {
                    withContext(Dispatchers.IO) { UserFrequencyService.deleteDatabase() }
                    allData = emptyList()
                }
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

private fun parseFrequencyCSV(csv: String): List<Pair<String, Int>> {
    val entries = mutableListOf<Pair<String, Int>>()
    for (line in csv.split("\n")) {
        val trimmed = line.trim()
        if (trimmed.isEmpty()) continue
        val columns = CsvUtils.parseLine(trimmed)
        if (columns.size < 2) continue
        val word = columns[0].trim()
        val count = columns[1].trim().toIntOrNull() ?: continue
        if (word.isEmpty() || count <= 0) continue
        entries.add(word to count)
    }
    return entries
}
