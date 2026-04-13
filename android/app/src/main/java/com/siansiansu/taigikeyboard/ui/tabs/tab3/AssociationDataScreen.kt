package com.siansiansu.taigikeyboard.ui.tabs.tab3

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Clear
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.CircularProgressIndicator
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
import com.siansiansu.taigikeyboard.ime.dictionary.NextWordService
import com.siansiansu.taigikeyboard.localization.CommonTexts
import com.siansiansu.taigikeyboard.localization.LanguageManager
import com.siansiansu.taigikeyboard.localization.Tab3Texts
import com.siansiansu.taigikeyboard.ui.components.ActionRow
import com.siansiansu.taigikeyboard.ui.components.ConfirmationDialog
import com.siansiansu.taigikeyboard.ui.components.FileDownload
import com.siansiansu.taigikeyboard.ui.components.FileUpload
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

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AssociationDataScreen(
    languageManager: LanguageManager,
    prefs: PrefHelper,
    onNavigateBack: () -> Unit,
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val displayLimit = 100

    var allData by remember { mutableStateOf<List<NextWordService.AssociationEntry>>(emptyList()) }
    var showClearDialog by remember { mutableStateOf(false) }
    var isImporting by remember { mutableStateOf(false) }
    var showResultDialog by remember { mutableStateOf(false) }
    var resultMessage by remember { mutableStateOf("") }
    var filterText by remember { mutableStateOf("") }

    val filteredData =
        if (filterText.isEmpty()) {
            allData.take(displayLimit)
        } else {
            val query = filterText.lowercase()
            allData.filter {
                it.prevWord.lowercase().contains(query) ||
                    it.prevTl.lowercase().contains(query) ||
                    it.nextWord.lowercase().contains(query) ||
                    it.nextTl.lowercase().contains(query)
            }
        }

    LaunchedEffect(Unit) {
        withContext(Dispatchers.IO) {
            val assoc = NextWordService.allAssociations(context)
            allData = assoc
        }
    }

    val exportLauncher =
        rememberLauncherForActivityResult(
            contract = ActivityResultContracts.CreateDocument("text/csv"),
        ) { uri: Uri? ->
            uri ?: return@rememberLauncherForActivityResult
            scope.launch {
                try {
                    val allData =
                        withContext(Dispatchers.IO) {
                            NextWordService.allAssociations(context)
                        }
                    val csv =
                        buildString {
                            for (entry in allData) {
                                append(
                                    "${CsvUtils.escape(
                                        entry.prevWord,
                                    )},${CsvUtils.escape(
                                        entry.prevTl,
                                    )},${CsvUtils.escape(entry.nextWord)},${CsvUtils.escape(entry.nextTl)},${entry.count}\n",
                                )
                            }
                        }
                    withContext(Dispatchers.IO) {
                        context.contentResolver.openOutputStream(uri)?.use {
                            it.write(csv.toByteArray(Charsets.UTF_8))
                        }
                    }
                    resultMessage = languageManager.text(Tab3Texts.exportSuccess)
                    showResultDialog = true
                } catch (e: Exception) {
                    resultMessage = e.localizedMessage ?: languageManager.text(CommonTexts.exportFailed)
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
                    val entries = parseAssociationCSV(csvString)
                    val imported =
                        withContext(Dispatchers.IO) {
                            NextWordService.batchImportAssociations(context, entries)
                        }
                    val skipped = entries.size - imported
                    resultMessage =
                        String.format(
                            languageManager.text(Tab3Texts.associationImportResult),
                            imported,
                            skipped,
                        )
                    showResultDialog = true
                    // Reload data
                    withContext(Dispatchers.IO) {
                        val assoc = NextWordService.allAssociations(context)
                        allData = assoc
                    }
                } catch (e: Exception) {
                    resultMessage = e.localizedMessage ?: languageManager.text(CommonTexts.importFailed)
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
                        text = languageManager.text(Tab3Texts.associationManagement),
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
                            label = languageManager.text(Tab3Texts.associationRecordingEnabled),
                            checked = prefs.associationRecordingEnabled,
                            infoText = languageManager.text(Tab3Texts.associationRecordingEnabledInfo),
                            onCheckedChange = { prefs.associationRecordingEnabled = it },
                        )
                    }
                }

                // Import/Export
                item {
                    Spacer(Modifier.height(16.dp))
                    Text(
                        text = languageManager.text(Tab3Texts.importExportTitle),
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(start = 16.dp, bottom = 8.dp),
                        style = MaterialTheme.typography.titleMedium,
                    )
                    SettingsCard {
                        Text(
                            text = languageManager.text(Tab3Texts.associationDescription),
                            color = MaterialTheme.colorScheme.onSurface,
                            style = MaterialTheme.typography.bodyLarge,
                            modifier = Modifier.padding(horizontal = 20.dp, vertical = 16.dp),
                        )
                        SettingsDivider()
                        ActionRow(
                            label = languageManager.text(Tab3Texts.associationExportCSV),
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
                            Row(
                                modifier =
                                    Modifier
                                        .fillMaxWidth()
                                        .heightIn(min = 48.dp)
                                        .padding(horizontal = 20.dp, vertical = 12.dp),
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.Center,
                            ) {
                                CircularProgressIndicator(modifier = Modifier.size(24.dp), strokeWidth = 2.dp)
                            }
                        } else {
                            ActionRow(
                                label = languageManager.text(Tab3Texts.associationImportCSV),
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
                            label = languageManager.text(Tab3Texts.clearAllAssociation),
                            onClick = { showClearDialog = true },
                            textColor = MaterialTheme.colorScheme.error,
                        )
                    }
                }

                // Privacy warning
                item {
                    Spacer(Modifier.height(16.dp))
                    SettingsCard {
                        Text(
                            text = languageManager.text(Tab3Texts.associationPrivacyWarning),
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
                            text = languageManager.text(Tab3Texts.associationManagement),
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            style = MaterialTheme.typography.titleMedium,
                        )
                        Spacer(Modifier.width(6.dp))
                        SettingInfoButton(description = languageManager.text(Tab3Texts.filterHint))
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
                                    text = languageManager.text(Tab3Texts.noData),
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                        }
                    }
                } else if (filterText.isNotEmpty() && filteredData.isEmpty()) {
                    item {
                        SettingsCard {
                            Text(
                                text = languageManager.text(Tab3Texts.noResults),
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
                                onClick = {
                                    scope.launch {
                                        withContext(Dispatchers.IO) {
                                            NextWordService.deleteAssociation(context, entry)
                                        }
                                        allData =
                                            allData.filter {
                                                !(
                                                    it.prevWord == entry.prevWord && it.prevTl == entry.prevTl &&
                                                        it.nextWord == entry.nextWord &&
                                                        it.nextTl == entry.nextTl
                                                )
                                            }
                                    }
                                },
                            ) {
                                Icon(
                                    imageVector = Icons.Default.Delete,
                                    contentDescription = languageManager.text(Tab3Texts.delete),
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
                        Text(languageManager.text(Tab3Texts.searchPlaceholder))
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
            title = languageManager.text(Tab3Texts.clearAllAssociation),
            message = languageManager.text(Tab3Texts.clearAssociationMessage),
            confirmLabel = languageManager.text(Tab3Texts.clear),
            dismissLabel = languageManager.text(CommonTexts.cancel),
            onConfirm = {
                showClearDialog = false
                scope.launch {
                    withContext(Dispatchers.IO) { NextWordService.clearAllAssociations(context) }
                    allData = emptyList()
                }
            },
            onDismiss = { showClearDialog = false },
        )
    }

    if (showResultDialog) {
        ResultDialog(
            message = resultMessage,
            confirmLabel = languageManager.text(Tab3Texts.ok),
            onDismiss = { showResultDialog = false },
        )
    }
}

private fun parseAssociationCSV(csv: String): List<NextWordService.AssociationEntry> {
    val entries = mutableListOf<NextWordService.AssociationEntry>()
    for (line in csv.split("\n")) {
        val trimmed = line.trim()
        if (trimmed.isEmpty()) continue
        val columns = CsvUtils.parseLine(trimmed)
        if (columns.size < 5) continue
        val prevWord = columns[0].trim()
        val prevTl = columns[1].trim()
        val nextWord = columns[2].trim()
        val nextTl = columns[3].trim()
        val count = columns[4].trim().toIntOrNull() ?: continue
        if (nextWord.isEmpty() || count <= 0) continue
        entries.add(NextWordService.AssociationEntry(prevWord, prevTl, nextWord, nextTl, count))
    }
    return entries
}
