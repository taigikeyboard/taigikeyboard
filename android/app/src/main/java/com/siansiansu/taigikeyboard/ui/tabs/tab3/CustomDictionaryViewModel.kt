package com.siansiansu.taigikeyboard.ui.tabs.tab3

import android.app.Application
import android.net.Uri
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.siansiansu.taigikeyboard.ime.core.CompositionRoot
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.dictionary.CustomDictionaryService
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

// ViewModel for CustomDictionaryScreen — entries, CRUD, CSV import/export, enable toggle.
class CustomDictionaryViewModel(
    application: Application,
) : AndroidViewModel(application) {
    private val prefs = PrefHelper(application)
    private val customDict: CustomDictionaryService = CompositionRoot.shared(application).customDict

    private val _entries = MutableStateFlow<List<CustomDictionaryService.Entry>>(emptyList())
    val entries: StateFlow<List<CustomDictionaryService.Entry>> = _entries.asStateFlow()

    private val _isLoading = MutableStateFlow(true)
    val isLoading: StateFlow<Boolean> = _isLoading.asStateFlow()

    private val _isImporting = MutableStateFlow(false)
    val isImporting: StateFlow<Boolean> = _isImporting.asStateFlow()

    private val _isCustomDictEnabled = MutableStateFlow(prefs.customDictEnabled)
    val isCustomDictEnabled: StateFlow<Boolean> = _isCustomDictEnabled.asStateFlow()

    fun setCustomDictEnabled(enabled: Boolean) {
        _isCustomDictEnabled.value = enabled
        prefs.customDictEnabled = enabled
    }

    fun load() {
        viewModelScope.launch {
            val all = withContext(Dispatchers.IO) { customDict.fetchAll() }
            _entries.value = all
            _isLoading.value = false
        }
    }

    fun save(entry: CustomDictionaryService.Entry) {
        viewModelScope.launch {
            withContext(Dispatchers.IO) { customDict.save(entry) }
            load()
        }
    }

    fun delete(id: String) {
        viewModelScope.launch {
            withContext(Dispatchers.IO) { customDict.delete(id) }
            load()
        }
    }

    fun deleteAll() {
        viewModelScope.launch {
            withContext(Dispatchers.IO) { customDict.deleteAll() }
            load()
        }
    }

    suspend fun exportCSV(): String = withContext(Dispatchers.IO) { customDict.exportCSV() }

    suspend fun importFile(uri: Uri): CustomDictionaryService.ImportResult {
        _isImporting.value = true
        try {
            return withContext(Dispatchers.IO) {
                customDict.importFromFile(getApplication(), uri)
            }
        } finally {
            _isImporting.value = false
            load()
        }
    }
}
