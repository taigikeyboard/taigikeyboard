package com.siansiansu.taigikeyboard.settings

import android.content.Context
import android.content.Intent
import android.os.Bundle
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.result.contract.ActivityResultContracts
import androidx.activity.viewModels
import androidx.compose.runtime.getValue
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.lifecycleScope
import com.siansiansu.taigikeyboard.i18n.currentStringResolver
import com.siansiansu.taigikeyboard.i18n.generated.StringKey
import com.siansiansu.taigikeyboard.i18n.generated.dictionaryImportBackupResult
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ui.setTaigiContent
import com.siansiansu.taigikeyboard.ui.tabs.dictionary.DataManagementScreen
import com.siansiansu.taigikeyboard.ui.tabs.dictionary.DataManagementViewModel
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

// Backup export and import for user dictionary data
class DataManagementActivity : ComponentActivity() {
    companion object {
        private const val BACKUP_DATE_FORMAT = "yyyy-MM-dd"

        fun createIntent(context: Context): Intent = Intent(context, DataManagementActivity::class.java)
    }

    private val viewModel: DataManagementViewModel by viewModels()

    // Backup export launcher
    private val exportBackupLauncher =
        registerForActivityResult(
            ActivityResultContracts.CreateDocument("application/json"),
        ) { uri ->
            uri ?: return@registerForActivityResult
            lifecycleScope.launch {
                try {
                    viewModel.exportBackup { json ->
                        withContext(Dispatchers.IO) {
                            contentResolver.openOutputStream(uri)?.use { it.write(json.toByteArray(Charsets.UTF_8)) }
                        }
                    }
                    Toast
                        .makeText(
                            this@DataManagementActivity,
                            commonString(StringKey.DICTIONARY_EXPORT_BACKUP_SUCCESS),
                            Toast.LENGTH_SHORT,
                        ).show()
                } catch (_: Exception) {
                    Toast.makeText(this@DataManagementActivity, commonString(StringKey.COMMON_EXPORT_FAILED), Toast.LENGTH_SHORT).show()
                }
            }
        }

    // Backup import launcher
    private val importBackupLauncher =
        registerForActivityResult(
            ActivityResultContracts.OpenDocument(),
        ) { uri ->
            uri ?: return@registerForActivityResult
            lifecycleScope.launch {
                try {
                    val result = viewModel.importBackup(uri) ?: return@launch
                    Toast
                        .makeText(
                            this@DataManagementActivity,
                            currentStringResolver().dictionaryImportBackupResult(
                                customDict = result.customDict,
                                frequency = result.frequency,
                                association = result.association,
                            ),
                            Toast.LENGTH_LONG,
                        ).show()
                } catch (_: Exception) {
                    Toast.makeText(this@DataManagementActivity, commonString(StringKey.COMMON_IMPORT_FAILED), Toast.LENGTH_SHORT).show()
                }
            }
        }

    // The export/import failure Toasts fire from launcher callbacks, outside any Composition,
    // so they resolve via the non-Compose entry point instead of a Compose-local resolver.
    private fun commonString(key: StringKey): String = currentStringResolver().resolve(key)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val prefs = PrefHelper(this)
        setTaigiContent(prefs) {
            val isProcessing by viewModel.isProcessing.collectAsStateWithLifecycle()

            DataManagementScreen(
                isProcessing = isProcessing,
                onNavigateBack = {
                    onBackPressedDispatcher.onBackPressed()
                },
                onExportBackup = {
                    val dateStr = SimpleDateFormat(BACKUP_DATE_FORMAT, Locale.US).format(Date())
                    exportBackupLauncher.launch("taigi_backup_$dateStr.taigi")
                },
                onImportBackup = {
                    importBackupLauncher.launch(arrayOf("application/json", "*/*"))
                },
            )
        }
    }
}
