package com.siansiansu.taigikeyboard.ui.settings

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
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Clear
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Search
import com.siansiansu.taigikeyboard.ui.components.FileDownload
import com.siansiansu.taigikeyboard.ui.components.FileUpload
import com.siansiansu.taigikeyboard.ui.components.MenuBook
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.dictionary.CustomDictionaryService
import com.siansiansu.taigikeyboard.localization.LanguageManager
import com.siansiansu.taigikeyboard.localization.Tab3Texts
import com.siansiansu.taigikeyboard.ui.components.ActionRow
import com.siansiansu.taigikeyboard.ui.components.ConfirmationDialog
import com.siansiansu.taigikeyboard.ui.components.ResultDialog
import com.siansiansu.taigikeyboard.ui.components.SettingInfoButton
import com.siansiansu.taigikeyboard.ui.components.SettingsCard
import com.siansiansu.taigikeyboard.ui.components.SettingsDivider
import com.siansiansu.taigikeyboard.ui.components.SwitchRow
import com.siansiansu.taigikeyboard.ui.theme.AppStyle
import com.siansiansu.taigikeyboard.ui.theme.SectionHeader
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import java.util.UUID

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun CustomDictionaryScreen(
    languageManager: LanguageManager,
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
            entries.take(100)
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
                            languageManager.text(Tab3Texts.importResult),
                            result.imported,
                            result.skipped,
                        )
                    showResultDialog = true
                    reload()
                } catch (e: Exception) {
                    resultMessage =
                        when {
                            e.message == "fileTooLarge" -> languageManager.text(Tab3Texts.fileTooLarge)
                            e.message == "tooManyEntries" -> languageManager.text(Tab3Texts.tooManyEntries)
                            e.message?.contains("格式") == true -> languageManager.text(Tab3Texts.invalidCSVFormat)
                            else -> e.localizedMessage ?: "Import failed"
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
                    resultMessage = languageManager.text(Tab3Texts.exportSuccess)
                    showResultDialog = true
                } catch (e: Exception) {
                    resultMessage = e.localizedMessage ?: "Export failed"
                    showResultDialog = true
                }
            }
        }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Text(
                        text = languageManager.text(Tab3Texts.customDictionary),
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
                            label = languageManager.text(Tab3Texts.customDictEnabled),
                            checked = prefs.customDictEnabled,
                            infoText = languageManager.text(Tab3Texts.customDictEnabledInfo),
                            onCheckedChange = { prefs.customDictEnabled = it },
                        )
                    }
                }

                // Import/Export + Delete
                item {
                    Spacer(Modifier.height(16.dp))
                    Text(
                        text = languageManager.text(Tab3Texts.importExportTitle),
                        fontSize = AppStyle.sectionHeaderFontSize,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(start = 16.dp, bottom = 8.dp),
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
                            text = languageManager.text(Tab3Texts.customDictDescription),
                            fontSize = AppStyle.bodyFontSize,
                            color = MaterialTheme.colorScheme.onSurface,
                            modifier = Modifier.padding(horizontal = 20.dp, vertical = 16.dp),
                        )
                        SettingsDivider()
                        ActionRow(
                            label = languageManager.text(Tab3Texts.exportCSV),
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
                            Row(
                                modifier =
                                    Modifier
                                        .fillMaxWidth()
                                        .heightIn(min = 48.dp)
                                        .padding(horizontal = 20.dp, vertical = 12.dp),
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.Center,
                            ) {
                                CircularProgressIndicator(
                                    modifier = Modifier.size(24.dp),
                                    strokeWidth = 2.dp,
                                )
                            }
                        } else {
                            ActionRow(
                                label = languageManager.text(Tab3Texts.importCSV),
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
                            label = languageManager.text(Tab3Texts.deleteAll),
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
                            text = languageManager.text(Tab3Texts.customDictPrivacyWarning),
                            fontSize = AppStyle.bodyFontSize,
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
                            text = languageManager.text(Tab3Texts.customDictionary),
                            fontSize = AppStyle.sectionHeaderFontSize,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                        Spacer(Modifier.width(6.dp))
                        SettingInfoButton(description = languageManager.text(Tab3Texts.filterHint))
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
                                    text = languageManager.text(Tab3Texts.customDictEmpty),
                                    fontSize = AppStyle.bodyFontSize,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                        }
                    }
                } else if (filterText.isNotEmpty() && filteredEntries.isEmpty()) {
                    item {
                        SettingsCard {
                            Text(
                                text = languageManager.text(Tab3Texts.noResults),
                                fontSize = AppStyle.bodyFontSize,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                modifier = Modifier.padding(horizontal = 20.dp, vertical = 16.dp),
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
                                fontSize = AppStyle.bodyFontSize,
                                color = MaterialTheme.colorScheme.onSurface,
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
                                    contentDescription = languageManager.text(Tab3Texts.delete),
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

    // Edit/Add dialog
    if (showEditDialog) {
        EditEntryDialog(
            languageManager = languageManager,
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
            title = languageManager.text(Tab3Texts.deleteAll),
            message = languageManager.text(Tab3Texts.deleteAllMessage),
            confirmLabel = languageManager.text(Tab3Texts.clear),
            dismissLabel = languageManager.text(Tab3Texts.cancel),
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
            confirmLabel = languageManager.text(Tab3Texts.ok),
            onDismiss = { showResultDialog = false },
        )
    }
}

@Composable
private fun EditEntryDialog(
    languageManager: LanguageManager,
    entry: CustomDictionaryService.Entry?,
    onDismiss: () -> Unit,
    onSave: (CustomDictionaryService.Entry) -> Unit,
) {
    var roman by remember(entry) { mutableStateOf(entry?.roman ?: "") }
    var hanzi by remember(entry) { mutableStateOf(entry?.hanzi ?: "") }
    val isEditing = entry != null
    val canSave = roman.isNotBlank() && hanzi.isNotBlank()

    AlertDialog(
        onDismissRequest = onDismiss,
        title = {
            Text(
                languageManager.text(
                    if (isEditing) Tab3Texts.editEntry else Tab3Texts.addEntry,
                ),
            )
        },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                OutlinedTextField(
                    value = roman,
                    onValueChange = { roman = it },
                    label = { Text(languageManager.text(Tab3Texts.romanLabel)) },
                    placeholder = { Text(languageManager.text(Tab3Texts.romanPlaceholder)) },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                )
                OutlinedTextField(
                    value = hanzi,
                    onValueChange = { hanzi = it },
                    label = { Text(languageManager.text(Tab3Texts.hanziLabel)) },
                    placeholder = { Text(languageManager.text(Tab3Texts.hanziPlaceholder)) },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                )
            }
        },
        confirmButton = {
            TextButton(
                onClick = {
                    val saved =
                        CustomDictionaryService.Entry(
                            id = entry?.id ?: UUID.randomUUID().toString(),
                            roman = roman.trim(),
                            hanzi = hanzi.trim(),
                        )
                    onSave(saved)
                },
                enabled = canSave,
            ) {
                Text(languageManager.text(Tab3Texts.save))
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) {
                Text(languageManager.text(Tab3Texts.cancel))
            }
        },
    )
}
