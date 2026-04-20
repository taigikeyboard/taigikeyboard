package com.siansiansu.taigikeyboard.ui.tabs.tab3

import android.app.Application
import android.net.Uri
import androidx.lifecycle.AndroidViewModel
import com.siansiansu.taigikeyboard.ime.core.CompositionRoot
import com.siansiansu.taigikeyboard.ime.dictionary.BackupService
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.withContext

// ViewModel for DataManagementScreen — backup export/import orchestration.
class DataManagementViewModel(
    application: Application,
) : AndroidViewModel(application) {
    private val backup: BackupService = CompositionRoot.shared(application).backup

    private val _isProcessing = MutableStateFlow(false)
    val isProcessing: StateFlow<Boolean> = _isProcessing.asStateFlow()

    // Runs [write] against the freshly exported JSON with `isProcessing` held true
    // across the generate + write phases. `write` is supplied by the Activity so
    // `contentResolver` streams stay at Activity scope (matches pre-impl D2).
    suspend fun exportBackup(write: suspend (String) -> Unit) {
        _isProcessing.value = true
        try {
            val json = backup.exportAll(getApplication())
            write(json)
        } finally {
            _isProcessing.value = false
        }
    }

    // Returns null when the input stream cannot be opened, matching the silent
    // early-return semantics of the pre-A3 Activity-side import handler.
    suspend fun importBackup(uri: Uri): BackupService.ImportResult? {
        _isProcessing.value = true
        try {
            val json =
                withContext(Dispatchers.IO) {
                    getApplication<Application>().contentResolver.openInputStream(uri)?.use {
                        it.bufferedReader(Charsets.UTF_8).readText()
                    }
                } ?: return null
            return backup.importAll(json)
        } finally {
            _isProcessing.value = false
        }
    }
}
