package com.siansiansu.taigikeyboard.settings

import android.content.Context
import android.content.Intent
import android.os.Bundle
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.lifecycleScope
import com.siansiansu.taigikeyboard.ime.dictionary.BackupService
import com.siansiansu.taigikeyboard.ime.dictionary.CustomDictionaryService
import com.siansiansu.taigikeyboard.localization.CommonTexts
import com.siansiansu.taigikeyboard.localization.Tab3Texts
import com.siansiansu.taigikeyboard.ui.tabs.tab3.DataManagementScreen
import com.siansiansu.taigikeyboard.ui.theme.TaigiKeyboardTheme
import com.siansiansu.taigikeyboard.util.setupEdgeToEdge
import kotlinx.coroutines.launch

class DataManagementActivity : ComponentActivity() {
    companion object {
        fun createIntent(context: Context): Intent = Intent(context, DataManagementActivity::class.java)
    }

    private var isProcessing by mutableStateOf(false)

    // Backup export launcher
    private val exportBackupLauncher =
        registerForActivityResult(
            ActivityResultContracts.CreateDocument("application/json"),
        ) { uri ->
            uri ?: return@registerForActivityResult
            isProcessing = true
            lifecycleScope.launch {
                try {
                    val json = BackupService.exportAll(this@DataManagementActivity)
                    contentResolver.openOutputStream(uri)?.use { it.write(json.toByteArray()) }
                    Toast.makeText(this@DataManagementActivity, Tab3Texts.exportBackupSuccess, Toast.LENGTH_SHORT).show()
                } catch (e: Exception) {
                    Toast.makeText(this@DataManagementActivity, CommonTexts.exportFailed, Toast.LENGTH_SHORT).show()
                } finally {
                    isProcessing = false
                }
            }
        }

    // Backup import launcher
    private val importBackupLauncher =
        registerForActivityResult(
            ActivityResultContracts.OpenDocument(),
        ) { uri ->
            uri ?: return@registerForActivityResult
            isProcessing = true
            lifecycleScope.launch {
                try {
                    val json = contentResolver.openInputStream(uri)?.bufferedReader()?.readText() ?: return@launch
                    val result = BackupService.importAll(this@DataManagementActivity, json)
                    Toast
                        .makeText(
                            this@DataManagementActivity,
                            String.format(Tab3Texts.importBackupResult, result.customDict, result.frequency, result.association),
                            Toast.LENGTH_LONG,
                        ).show()
                } catch (e: Exception) {
                    Toast.makeText(this@DataManagementActivity, CommonTexts.importFailed, Toast.LENGTH_SHORT).show()
                } finally {
                    isProcessing = false
                }
            }
        }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        CustomDictionaryService.init(this)

        setupEdgeToEdge()

        setContent {
            TaigiKeyboardTheme {
                DataManagementScreen(
                    isProcessing = isProcessing,
                    onNavigateBack = {
                        onBackPressedDispatcher.onBackPressed()
                    },
                    onExportBackup = {
                        val dateStr = java.text.SimpleDateFormat("yyyy-MM-dd", java.util.Locale.US).format(java.util.Date())
                        exportBackupLauncher.launch("備份復原_$dateStr.taigi")
                    },
                    onImportBackup = {
                        importBackupLauncher.launch(arrayOf("application/json", "*/*"))
                    },
                )
            }
        }
    }
}
