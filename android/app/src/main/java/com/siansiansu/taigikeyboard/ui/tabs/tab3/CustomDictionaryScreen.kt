package com.siansiansu.taigikeyboard.ui.tabs.tab3

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
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
import androidx.compose.material.icons.filled.Add
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
import androidx.compose.ui.draw.clip
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.dictionary.CustomDictionaryService
import com.siansiansu.taigikeyboard.localization.CommonTexts
import com.siansiansu.taigikeyboard.localization.Tab3Texts
import com.siansiansu.taigikeyboard.ui.components.ActionRow
import com.siansiansu.taigikeyboard.ui.components.ConfirmationDialog
import com.siansiansu.taigikeyboard.ui.components.FileDownload
import com.siansiansu.taigikeyboard.ui.components.FileUpload
import com.siansiansu.taigikeyboard.ui.components.FilterSearchBar
import com.siansiansu.taigikeyboard.ui.components.LoadingRow
import com.siansiansu.taigikeyboard.ui.components.MenuBook
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

// Custom dictionary management — add, edit, import/export, and delete entries
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun CustomDictionaryScreen(
    prefs: PrefHelper,
    onNavigateBack: () -> Unit,
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()

    var entries by remember { mutableStateOf<List<CustomDictionaryService.Entry>>(emptyList()) }
    var showEditDialog by remember { mutableStateOf(false) }
    var editingEntry by remember { mutableStateOf<CustomDictionaryService.Entry?>(null) }
    var showDeleteAllDialog by remember { mutableStateOf(false) }
    var isImporting by remember { mutableStateOf(false) }
    var showResultDialog by remember { mutableStateOf(false) }
    var resultMessage by remember { mutableStateOf("") }
    var filterText by remember { mutableStateOf("") }

    val filteredEntries =
        if (filterText.isEmpty()) {
            entries.take(DISPLAY_LIMIT)
        } else {
            val query = filterText.lowercase()
            entries.filter {
                it.roman.lowercase().contains(query) ||
                    it.hanzi.lowercase().contains(query)
            }
        }

    fun reload() {
        scope.launch { entries = CustomDictionaryService.fetchAll() }
    }

    LaunchedEffect(Unit) {
        reload()
    }

    // File import launcher
    val importLauncher =
        rememberLauncherForActivityResult(
            contract = ActivityResultContracts.OpenDocument(),
        ) { uri: Uri? ->
            uri ?: return@rememberLauncherForActivityResult
            isImporting = true
            scope.launch {
                try {
                    val result = CustomDictionaryService.importFromFile(context, uri)
                    resultMessage =
                        String.format(
                            Tab3Texts.importResult,
                            result.imported,
                            result.skipped,
                        )
                    showResultDialog = true
                    reload()
                } catch (e: Exception) {
                    resultMessage =
                        when {
                            e.message == "fileTooLarge" -> Tab3Texts.fileTooLarge
                            e.message == "tooManyEntries" -> Tab3Texts.tooManyEntries
                            e.message?.contains("格式") == true -> Tab3Texts.invalidCSVFormat
                            else -> e.localizedMessage ?: CommonTexts.importFailed
                        }
                    showResultDialog = true
                } finally {
                    isImporting = false
                }
            }
        }

    // File export launcher
    val exportLauncher =
        rememberLauncherForActivityResult(
            contract = ActivityResultContracts.CreateDocument("text/csv"),
        ) { uri: Uri? ->
            uri ?: return@rememberLauncherForActivityResult
            scope.launch {
                try {
                    val csv = CustomDictionaryService.exportCSV()
                    context.contentResolver.openOutputStream(uri)?.use { outputStream ->
                        outputStream.write(csv.toByteArray(Charsets.UTF_8))
                    }
                    resultMessage = Tab3Texts.exportSuccess
                    showResultDialog = true
                } catch (e: Exception) {
                    resultMessage = e.localizedMessage ?: CommonTexts.exportFailed
                    showResultDialog = true
                }
            }
        }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Text(
                        text = Tab3Texts.customDictionary,
                        fontWeight = FontWeight.Bold,
                    )
                },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = null)
                    }
                },
                actions = {
                    IconButton(onClick = {
                        editingEntry = null
                        showEditDialog = true
                    }) {
                        Icon(Icons.Default.Add, contentDescription = null)
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
                // Enable/Disable toggle
                item {
                    Spacer(Modifier.height(8.dp))
                    SettingsCard {
                        SwitchRow(
                            label = Tab3Texts.customDictEnabled,
                            checked = prefs.customDictEnabled,
                            infoText = Tab3Texts.customDictEnabledInfo,
                            onCheckedChange = { prefs.customDictEnabled = it },
                        )
                    }
                }

                // Import/Export + Delete
                item {
                    Spacer(Modifier.height(16.dp))
                    Text(
                        text = Tab3Texts.importExportTitle,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(start = 16.dp, bottom = 8.dp),
                        style = MaterialTheme.typography.titleMedium,
                    )

                    SettingsCard {
                        Image(
                            painter = painterResource(R.drawable.csv_example),
                            contentDescription = null,
                            modifier =
                                Modifier
                                    .fillMaxWidth()
                                    .padding(horizontal = 20.dp)
                                    .padding(top = 16.dp)
                                    .clip(RoundedCornerShape(8.dp)),
                            contentScale = ContentScale.FillWidth,
                        )
                        Text(
                            text = Tab3Texts.customDictDescription,
                            color = MaterialTheme.colorScheme.onSurface,
                            modifier = Modifier.padding(horizontal = 20.dp, vertical = 16.dp),
                            style = MaterialTheme.typography.bodyLarge,
                        )
                        SettingsDivider()
                        ActionRow(
                            label = Tab3Texts.exportCSV,
                            onClick = {
                                if (!isImporting) {
                                    val dateStr = SimpleDateFormat("yyyy-MM-dd", Locale.US).format(Date())
                                    exportLauncher.launch("自訂詞庫_$dateStr.csv")
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
                                label = Tab3Texts.importCSV,
                                onClick = { importLauncher.launch(arrayOf("text/*")) },
                                icon = Icons.Outlined.FileDownload,
                                textColor = MaterialTheme.colorScheme.primary,
                            )
                        }
                    }
                }

                // Delete all
                item {
                    Spacer(Modifier.height(16.dp))
                    SettingsCard {
                        ActionRow(
                            label = Tab3Texts.deleteAll,
                            onClick = { if (!isImporting) showDeleteAllDialog = true },
                            textColor = MaterialTheme.colorScheme.error,
                        )
                    }
                }

                // Privacy warning
                item {
                    Spacer(Modifier.height(16.dp))
                    SettingsCard {
                        Text(
                            text = Tab3Texts.customDictPrivacyWarning,
                            style = MaterialTheme.typography.bodyLarge,
                            color = MaterialTheme.colorScheme.onSurface,
                            modifier = Modifier.padding(horizontal = 20.dp, vertical = 16.dp),
                        )
                    }
                }

                // Entry list header
                item {
                    Spacer(Modifier.height(24.dp))
                    Row(
                        modifier = Modifier.padding(start = 16.dp, bottom = 8.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(
                            text = Tab3Texts.customDictionary,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            style = MaterialTheme.typography.titleMedium,
                        )
                        Spacer(Modifier.width(6.dp))
                        SettingInfoButton(description = Tab3Texts.filterHint)
                    }
                }

                if (entries.isEmpty()) {
                    item {
                        SettingsCard {
                            Column(
                                modifier =
                                    Modifier
                                        .fillMaxWidth()
                                        .padding(vertical = 32.dp),
                                horizontalAlignment = Alignment.CenterHorizontally,
                                verticalArrangement = Arrangement.spacedBy(16.dp),
                            ) {
                                Icon(
                                    imageVector = Icons.Outlined.MenuBook,
                                    contentDescription = null,
                                    modifier = Modifier.size(48.dp),
                                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                                Text(
                                    text = Tab3Texts.customDictEmpty,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    style = MaterialTheme.typography.bodyLarge,
                                )
                            }
                        }
                    }
                } else if (filterText.isNotEmpty() && filteredEntries.isEmpty()) {
                    item {
                        SettingsCard {
                            Text(
                                text = Tab3Texts.noResults,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                modifier = Modifier.padding(horizontal = 20.dp, vertical = 16.dp),
                                style = MaterialTheme.typography.bodyLarge,
                            )
                        }
                    }
                } else {
                    itemsIndexed(
                        items = filteredEntries,
                        key = { _, entry -> entry.id },
                    ) { index, entry ->
                        Row(
                            modifier =
                                Modifier
                                    .fillMaxWidth()
                                    .background(MaterialTheme.colorScheme.surface)
                                    .heightIn(min = 48.dp)
                                    .padding(start = 20.dp, end = 4.dp),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Text(
                                text = "${entry.roman} → ${entry.hanzi}",
                                color = MaterialTheme.colorScheme.onSurface,
                                style = MaterialTheme.typography.bodyLarge,
                                modifier =
                                    Modifier
                                        .weight(1f)
                                        .clickable {
                                            editingEntry = entry
                                            showEditDialog = true
                                        }.padding(vertical = 12.dp),
                            )
                            IconButton(
                                onClick = {
                                    scope.launch {
                                        CustomDictionaryService.delete(entry.id)
                                        reload()
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
                        if (index < filteredEntries.lastIndex) {
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
                placeholder = Tab3Texts.searchPlaceholder,
            )
        }
    }

    // Edit/Add dialog
    if (showEditDialog) {
        EditEntryDialog(
            entry = editingEntry,
            onDismiss = { showEditDialog = false },
            onSave = { entry ->
                scope.launch {
                    CustomDictionaryService.save(entry)
                    reload()
                }
                showEditDialog = false
            },
        )
    }

    // Delete all confirmation
    if (showDeleteAllDialog) {
        ConfirmationDialog(
            title = Tab3Texts.deleteAll,
            message = Tab3Texts.deleteAllMessage,
            confirmLabel = Tab3Texts.clear,
            dismissLabel = CommonTexts.cancel,
            onConfirm = {
                showDeleteAllDialog = false
                scope.launch {
                    CustomDictionaryService.deleteAll()
                    reload()
                }
            },
            onDismiss = { showDeleteAllDialog = false },
        )
    }

    // Result dialog
    if (showResultDialog) {
        ResultDialog(
            message = resultMessage,
            confirmLabel = CommonTexts.ok,
            onDismiss = { showResultDialog = false },
        )
    }
}
