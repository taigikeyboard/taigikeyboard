package com.siansiansu.taigikeyboard.ui.settings

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
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
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
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
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.dictionary.CustomDictionaryService
import com.siansiansu.taigikeyboard.localization.LanguageManager
import com.siansiansu.taigikeyboard.localization.Tab3Texts
import com.siansiansu.taigikeyboard.ui.components.ActionRow
import com.siansiansu.taigikeyboard.ui.components.SettingsCard
import com.siansiansu.taigikeyboard.ui.components.SettingsDivider
import kotlinx.coroutines.launch
import java.util.UUID

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun CustomDictionaryScreen(
    languageManager: LanguageManager,
    onNavigateBack: () -> Unit
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()

    var entries by remember { mutableStateOf<List<CustomDictionaryService.Entry>>(emptyList()) }
    var showEditDialog by remember { mutableStateOf(false) }
    var editingEntry by remember { mutableStateOf<CustomDictionaryService.Entry?>(null) }
    var showDeleteAllDialog by remember { mutableStateOf(false) }
    var showHelpDialog by remember { mutableStateOf(false) }
    var showResultDialog by remember { mutableStateOf(false) }
    var resultMessage by remember { mutableStateOf("") }

    fun reload() {
        scope.launch { entries = CustomDictionaryService.fetchAll() }
    }

    LaunchedEffect(Unit) {
        reload()
    }

    // File import launcher
    val importLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.OpenDocument()
    ) { uri: Uri? ->
        uri ?: return@rememberLauncherForActivityResult
        scope.launch {
            try {
                val result = CustomDictionaryService.importFromFile(context, uri)
                resultMessage = String.format(
                    languageManager.text(Tab3Texts.importResult),
                    result.imported, result.skipped
                )
                showResultDialog = true
                reload()
            } catch (e: Exception) {
                resultMessage = if (e.message?.contains("格式") == true) {
                    languageManager.text(Tab3Texts.invalidCSVFormat)
                } else {
                    e.localizedMessage ?: "Import failed"
                }
                showResultDialog = true
            }
        }
    }

    // File export launcher
    val exportLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.CreateDocument("text/csv")
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
                        fontWeight = FontWeight.Bold
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
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.surfaceContainer
                )
            )
        },
        containerColor = MaterialTheme.colorScheme.surfaceContainer
    ) { padding ->
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(horizontal = 20.dp),
        ) {
            // Import/Export section
            item {
                Spacer(Modifier.height(8.dp))

                // Section header with help icon
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    modifier = Modifier.padding(bottom = 8.dp)
                ) {
                    Text(
                        text = languageManager.text(Tab3Texts.importExportTitle),
                        fontSize = 16.sp,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                    Spacer(Modifier.width(4.dp))
                    IconButton(
                        onClick = { showHelpDialog = true },
                        modifier = Modifier.size(24.dp)
                    ) {
                        Icon(
                            painter = painterResource(R.drawable.ic_help),
                            contentDescription = null,
                            modifier = Modifier.size(18.dp),
                            tint = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                }

                SettingsCard {
                    ActionRow(
                        label = languageManager.text(Tab3Texts.importCSV),
                        onClick = { importLauncher.launch(arrayOf("text/*")) }
                    )
                    SettingsDivider()
                    ActionRow(
                        label = languageManager.text(Tab3Texts.exportCSV),
                        onClick = { exportLauncher.launch("custom_dictionary.csv") }
                    )
                }

                Spacer(Modifier.height(24.dp))
            }

            // Entry list
            if (entries.isEmpty()) {
                item {
                    SettingsCard {
                        Column(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(vertical = 32.dp),
                            horizontalAlignment = Alignment.CenterHorizontally
                        ) {
                            Text(
                                text = languageManager.text(Tab3Texts.customDictEmpty),
                                fontSize = 16.sp,
                                color = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                        }
                    }
                }
            } else {
                item {
                    Text(
                        text = "${entries.size} ${languageManager.text(Tab3Texts.entriesCount)}",
                        fontSize = 14.sp,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(bottom = 8.dp)
                    )
                }

                item {
                    SettingsCard {
                        entries.forEachIndexed { index, entry ->
                            if (index > 0) SettingsDivider()
                            EntryRow(
                                entry = entry,
                                onClick = {
                                    editingEntry = entry
                                    showEditDialog = true
                                },
                                onDelete = {
                                    scope.launch {
                                        CustomDictionaryService.delete(entry.id)
                                        reload()
                                    }
                                }
                            )
                        }
                    }
                }

                // Delete all
                item {
                    Spacer(Modifier.height(24.dp))
                    SettingsCard {
                        ActionRow(
                            label = languageManager.text(Tab3Texts.deleteAll),
                            onClick = { showDeleteAllDialog = true },
                            textColor = MaterialTheme.colorScheme.error
                        )
                    }
                }
            }

            item { Spacer(Modifier.height(40.dp)) }
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
            }
        )
    }

    // Delete all confirmation
    if (showDeleteAllDialog) {
        AlertDialog(
            onDismissRequest = { showDeleteAllDialog = false },
            title = { Text(languageManager.text(Tab3Texts.deleteAll)) },
            text = { Text(languageManager.text(Tab3Texts.deleteAllMessage)) },
            confirmButton = {
                TextButton(onClick = {
                    showDeleteAllDialog = false
                    scope.launch {
                        CustomDictionaryService.deleteAll()
                        reload()
                    }
                }) {
                    Text(languageManager.text(Tab3Texts.clear))
                }
            },
            dismissButton = {
                TextButton(onClick = { showDeleteAllDialog = false }) {
                    Text(languageManager.text(Tab3Texts.cancel))
                }
            }
        )
    }

    // Help dialog
    if (showHelpDialog) {
        AlertDialog(
            onDismissRequest = { showHelpDialog = false },
            title = { Text(languageManager.text(Tab3Texts.importExportHelpTitle)) },
            text = { Text(languageManager.text(Tab3Texts.importExportHelp)) },
            confirmButton = {
                TextButton(onClick = { showHelpDialog = false }) {
                    Text(languageManager.text(Tab3Texts.ok))
                }
            }
        )
    }

    // Result dialog
    if (showResultDialog) {
        AlertDialog(
            onDismissRequest = { showResultDialog = false },
            text = { Text(resultMessage) },
            confirmButton = {
                TextButton(onClick = { showResultDialog = false }) {
                    Text(languageManager.text(Tab3Texts.ok))
                }
            }
        )
    }
}

@Composable
private fun EntryRow(
    entry: CustomDictionaryService.Entry,
    onClick: () -> Unit,
    onDelete: () -> Unit
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .heightIn(min = 48.dp)
            .clickable(onClick = onClick)
            .padding(horizontal = 20.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        Row(
            modifier = Modifier.weight(1f),
            horizontalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            Text(
                text = entry.roman,
                fontSize = 16.sp,
                color = MaterialTheme.colorScheme.onSurface
            )
            Text(
                text = entry.hanzi,
                fontSize = 16.sp,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
        IconButton(onClick = onDelete, modifier = Modifier.size(24.dp)) {
            Icon(
                imageVector = Icons.Default.Delete,
                contentDescription = null,
                modifier = Modifier.size(18.dp),
                tint = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
    }
}

@Composable
private fun EditEntryDialog(
    languageManager: LanguageManager,
    entry: CustomDictionaryService.Entry?,
    onDismiss: () -> Unit,
    onSave: (CustomDictionaryService.Entry) -> Unit
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
                    if (isEditing) Tab3Texts.editEntry else Tab3Texts.addEntry
                )
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
                    modifier = Modifier.fillMaxWidth()
                )
                OutlinedTextField(
                    value = hanzi,
                    onValueChange = { hanzi = it },
                    label = { Text(languageManager.text(Tab3Texts.hanziLabel)) },
                    placeholder = { Text(languageManager.text(Tab3Texts.hanziPlaceholder)) },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth()
                )
            }
        },
        confirmButton = {
            TextButton(
                onClick = {
                    val saved = CustomDictionaryService.Entry(
                        id = entry?.id ?: UUID.randomUUID().toString(),
                        roman = roman.trim(),
                        hanzi = hanzi.trim()
                    )
                    onSave(saved)
                },
                enabled = canSave
            ) {
                Text(languageManager.text(Tab3Texts.save))
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) {
                Text(languageManager.text(Tab3Texts.cancel))
            }
        }
    )
}

