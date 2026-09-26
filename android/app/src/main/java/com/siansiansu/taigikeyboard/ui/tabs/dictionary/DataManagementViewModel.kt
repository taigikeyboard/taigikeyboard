package com.siansiansu.taigikeyboard.ui.tabs.dictionary

import android.app.Application
import android.net.Uri
import androidx.lifecycle.AndroidViewModel
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.ime.core.CompositionRoot
import com.siansiansu.taigikeyboard.ime.dictionary.BackupImportResult
import com.siansiansu.taigikeyboard.ime.dictionary.UserDataClient
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.withContext

// ViewModel for DataManagementScreen — backup export/import orchestration. The
// `.taigi` file is the engine's codec (roadmap P8b); this side picks the file.
class DataManagementViewModel(
    application: Application,
) : AndroidViewModel(application) {
    private val userData: UserDataClient = CompositionRoot.shared(application).userData

    private val _isProcessing = MutableStateFlow(false)
    val isProcessing: StateFlow<Boolean> = _isProcessing.asStateFlow()

    // Runs [write] against the freshly exported backup with `isProcessing` held true
    // across the generate + write phases. `write` is supplied by the Activity so
    // `contentResolver` streams stay at Activity scope (matches pre-impl D2).
    suspend fun exportBackup(write: suspend (ByteArray) -> Unit) {
        _isProcessing.value = true
        try {
            val backup = userData.exportBackup(BuildConfig.VERSION_NAME)
            write(backup)
        } finally {
            _isProcessing.value = false
        }
    }

    // Returns null when the input stream cannot be opened, matching the silent
    // early-return semantics of the pre-A3 Activity-side import handler. A file
    // the engine cannot read as a backup throws.
    suspend fun importBackup(uri: Uri): BackupImportResult? {
        _isProcessing.value = true
        try {
            val backup =
                withContext(Dispatchers.IO) {
                    getApplication<Application>().contentResolver.openInputStream(uri)?.use { it.readBytes() }
                } ?: return null
            return userData.importBackup(backup)
        } finally {
            _isProcessing.value = false
        }
    }
}
