package com.siansiansu.taigikeyboard.ui.tabs.dictionary

import android.app.Application
import android.net.Uri
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.siansiansu.taigikeyboard.ime.core.CompositionRoot
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.dictionary.DictionaryCsvCodec
import com.siansiansu.taigikeyboard.ime.dictionary.NextWordService
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

// ViewModel for AssociationDataScreen — bigram list, CSV import/export, recording toggle.
class AssociationDataViewModel(
    application: Application,
) : AndroidViewModel(application) {
    data class ImportOutcome(
        val imported: Int,
        val skipped: Int,
    )

    private val prefs = PrefHelper(application)
    private val nextWord: NextWordService = CompositionRoot.shared(application).nextWord

    private val _allData = MutableStateFlow<List<NextWordService.AssociationEntry>>(emptyList())
    val allData: StateFlow<List<NextWordService.AssociationEntry>> = _allData.asStateFlow()

    private val _isLoading = MutableStateFlow(true)
    val isLoading: StateFlow<Boolean> = _isLoading.asStateFlow()

    private val _isImporting = MutableStateFlow(false)
    val isImporting: StateFlow<Boolean> = _isImporting.asStateFlow()

    private val _isAssociationRecordingEnabled = MutableStateFlow(prefs.associationRecordingEnabled)
    val isAssociationRecordingEnabled: StateFlow<Boolean> = _isAssociationRecordingEnabled.asStateFlow()

    fun setRecordingEnabled(enabled: Boolean) {
        _isAssociationRecordingEnabled.value = enabled
        prefs.associationRecordingEnabled = enabled
    }

    fun load() {
        viewModelScope.launch {
            val assoc = withContext(Dispatchers.IO) { nextWord.allAssociations() }
            _allData.value = assoc
            _isLoading.value = false
        }
    }

    fun delete(entry: NextWordService.AssociationEntry) {
        viewModelScope.launch {
            withContext(Dispatchers.IO) { nextWord.deleteAssociation(entry) }
            _allData.value =
                _allData.value.filter {
                    !(
                        it.prevWord == entry.prevWord &&
                            it.prevTl == entry.prevTl &&
                            it.nextWord == entry.nextWord &&
                            it.nextTl == entry.nextTl
                    )
                }
        }
    }

    fun clearAll() {
        viewModelScope.launch {
            withContext(Dispatchers.IO) { nextWord.clearAllAssociations() }
            _allData.value = emptyList()
        }
    }

    suspend fun exportCSV(): String =
        withContext(Dispatchers.IO) {
            DictionaryCsvCodec.encodeAssociationCSV(nextWord.allAssociations())
        }

    suspend fun importCSV(uri: Uri): ImportOutcome {
        _isImporting.value = true
        try {
            val csvString =
                withContext(Dispatchers.IO) {
                    getApplication<Application>().contentResolver.openInputStream(uri)?.use {
                        it.bufferedReader(Charsets.UTF_8).readText()
                    } ?: throw Exception("Cannot read file")
                }
            val entries = DictionaryCsvCodec.decodeAssociationCSV(csvString)
            val imported = withContext(Dispatchers.IO) { nextWord.batchImportAssociations(entries) }
            val refreshed = withContext(Dispatchers.IO) { nextWord.allAssociations() }
            _allData.value = refreshed
            return ImportOutcome(imported = imported, skipped = entries.size - imported)
        } finally {
            _isImporting.value = false
        }
    }
}
