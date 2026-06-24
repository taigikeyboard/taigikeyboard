package com.siansiansu.taigikeyboard.ui.tabs.dictionary

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
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.i18n.LocalStringResolver
import com.siansiansu.taigikeyboard.i18n.generated.L10n
import com.siansiansu.taigikeyboard.i18n.generated.StringKey
import com.siansiansu.taigikeyboard.i18n.generated.dictionaryImportResult
import com.siansiansu.taigikeyboard.ime.dictionary.CustomDictionaryService
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
    viewModel: CustomDictionaryViewModel,
    onNavigateBack: () -> Unit,
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    // Captured for the non-Composable export/import callbacks; tracks the live display language.
    val stringResolver by rememberUpdatedState(LocalStringResolver.current)

    val entries by viewModel.entries.collectAsStateWithLifecycle()
    val isImporting by viewModel.isImporting.collectAsStateWithLifecycle()
    val isCustomDictEnabled by viewModel.isCustomDictEnabled.collectAsStateWithLifecycle()

    var showEditDialog by remember { mutableStateOf(false) }
    var editingEntry by remember { mutableStateOf<CustomDictionaryService.Entry?>(null) }
    var showDeleteAllDialog by remember { mutableStateOf(false) }
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

    LaunchedEffect(Unit) { viewModel.load() }

    // File import launcher
    val importLauncher =
        rememberLauncherForActivityResult(
            contract = ActivityResultContracts.OpenDocument(),
        ) { uri: Uri? ->
            uri ?: return@rememberLauncherForActivityResult
            scope.launch {
                try {
                    val result = viewModel.importFile(uri)
                    resultMessage =
                        stringResolver.dictionaryImportResult(
                            imported = result.imported,
                            skipped = result.skipped,
                        )
                    showResultDialog = true
                } catch (e: Exception) {
                    resultMessage =
                        when {
                            e.message == "fileTooLarge" -> stringResolver.resolve(StringKey.DICTIONARY_FILE_TOO_LARGE)
                            e.message == "tooManyEntries" -> stringResolver.resolve(StringKey.DICTIONARY_TOO_MANY_ENTRIES)
                            e.message?.contains("格式") == true -> stringResolver.resolve(StringKey.DICTIONARY_INVALID_C_S_V_FORMAT)
                            else -> e.localizedMessage ?: stringResolver.resolve(StringKey.COMMON_IMPORT_FAILED)
                        }
                    showResultDialog = true
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
                    val csv = viewModel.exportCSV()
                    withContext(Dispatchers.IO) {
                        context.contentResolver.openOutputStream(uri)?.use { outputStream ->
                            outputStream.write(csv.toByteArray(Charsets.UTF_8))
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

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Text(
                        text = L10n.dictionaryCustomDictionary,
                        fontWeight = FontWeight.Bold,
                    )
                },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = L10n.commonBack)
                    }
                },
                actions = {
                    IconButton(onClick = {
                        editingEntry = null
                        showEditDialog = true
                    }) {
                        Icon(Icons.Default.Add, contentDescription = L10n.dictionaryAddEntry)
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
                            label = L10n.dictionaryCustomDictEnabled,
                            checked = isCustomDictEnabled,
                            infoText = L10n.dictionaryCustomDictEnabledInfo,
                            onCheckedChange = { viewModel.setCustomDictEnabled(it) },
                        )
                    }
                }

                // Import/Export + Delete
                item {
                    Spacer(Modifier.height(16.dp))
                    Text(
                        text = L10n.dictionaryImportExportTitle,
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
                            text = L10n.dictionaryCustomDictDescription,
                            color = MaterialTheme.colorScheme.onSurface,
                            modifier = Modifier.padding(horizontal = 20.dp, vertical = 16.dp),
                            style = MaterialTheme.typography.bodyLarge,
                        )
                        SettingsDivider()
                        ActionRow(
                            label = L10n.dictionaryExportCSV,
                            onClick = {
                                if (!isImporting) {
                                    val dateStr = SimpleDateFormat("yyyy-MM-dd", Locale.US).format(Date())
                                    exportLauncher.launch("taigi_custom_dictionary_$dateStr.csv")
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
                                label = L10n.dictionaryImportCSV,
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
                            label = L10n.dictionaryDeleteAll,
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
                            text = L10n.dictionaryCustomDictPrivacyWarning,
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
                            text = L10n.dictionaryCustomDictionary,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            style = MaterialTheme.typography.titleMedium,
                        )
                        Spacer(Modifier.width(6.dp))
                        SettingInfoButton(description = L10n.dictionaryFilterHint)
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
                                    imageVector = Icons.AutoMirrored.Outlined.MenuBook,
                                    contentDescription = null,
                                    modifier = Modifier.size(48.dp),
                                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                                Text(
                                    text = L10n.dictionaryCustomDictEmpty,
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
                                text = L10n.dictionaryNoResults,
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
                                onClick = { viewModel.delete(entry.id) },
                            ) {
                                Icon(
                                    imageVector = Icons.Default.Delete,
                                    contentDescription = L10n.commonDelete,
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
                placeholder = L10n.dictionarySearchPlaceholder,
            )
        }
    }

    // Edit/Add dialog
    if (showEditDialog) {
        EditEntryDialog(
            entry = editingEntry,
            onDismiss = { showEditDialog = false },
            onSave = { entry ->
                viewModel.save(entry)
                showEditDialog = false
            },
        )
    }

    // Delete all confirmation
    if (showDeleteAllDialog) {
        ConfirmationDialog(
            title = L10n.dictionaryDeleteAll,
            message = L10n.dictionaryDeleteAllMessage,
            confirmLabel = L10n.dictionaryClear,
            dismissLabel = L10n.commonCancel,
            onConfirm = {
                showDeleteAllDialog = false
                viewModel.deleteAll()
            },
            onDismiss = { showDeleteAllDialog = false },
        )
    }

    // Result dialog
    if (showResultDialog) {
        ResultDialog(
            message = resultMessage,
            confirmLabel = L10n.commonOk,
            onDismiss = { showResultDialog = false },
        )
    }
}
