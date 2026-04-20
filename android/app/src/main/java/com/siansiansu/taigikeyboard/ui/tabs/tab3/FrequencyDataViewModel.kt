package com.siansiansu.taigikeyboard.ui.tabs.tab3

import android.app.Application
import android.net.Uri
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.siansiansu.taigikeyboard.ime.core.CompositionRoot
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.text.composing.UserFrequencyService
import com.siansiansu.taigikeyboard.util.CsvUtils
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

// ViewModel for FrequencyDataScreen — frequency list, CSV import/export, recording toggle.
class FrequencyDataViewModel(
    application: Application,
) : AndroidViewModel(application) {
    data class ImportOutcome(
        val imported: Int,
        val skipped: Int,
    )

    private val prefs = PrefHelper(application)
    private val userFreq: UserFrequencyService = CompositionRoot.shared(application).userFreq

    private val _allData = MutableStateFlow<List<Pair<String, Int>>>(emptyList())
    val allData: StateFlow<List<Pair<String, Int>>> = _allData.asStateFlow()

    private val _isLoading = MutableStateFlow(true)
    val isLoading: StateFlow<Boolean> = _isLoading.asStateFlow()

    private val _isImporting = MutableStateFlow(false)
    val isImporting: StateFlow<Boolean> = _isImporting.asStateFlow()

    private val _isFrequencyRecordingEnabled = MutableStateFlow(prefs.frequencyRecordingEnabled)
    val isFrequencyRecordingEnabled: StateFlow<Boolean> = _isFrequencyRecordingEnabled.asStateFlow()

    fun setRecordingEnabled(enabled: Boolean) {
        _isFrequencyRecordingEnabled.value = enabled
        prefs.frequencyRecordingEnabled = enabled
    }

    fun load() {
        viewModelScope.launch {
            val freq = withContext(Dispatchers.IO) { userFreq.getAllFrequencies() }
            _allData.value = freq
            _isLoading.value = false
        }
    }

    fun deleteWord(word: String) {
        viewModelScope.launch {
            withContext(Dispatchers.IO) { userFreq.deleteWord(word) }
            _allData.value = _allData.value.filter { it.first != word }
        }
    }

    fun clearAll() {
        viewModelScope.launch {
            withContext(Dispatchers.IO) { userFreq.deleteDatabase() }
            _allData.value = emptyList()
        }
    }

    suspend fun exportCSV(): String =
        withContext(Dispatchers.IO) {
            val data = userFreq.getAllFrequencies()
            buildString {
                for ((word, count) in data) {
                    append("${CsvUtils.escape(word)},$count\n")
                }
            }
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
            val entries = parseCSV(csvString)
            val imported = withContext(Dispatchers.IO) { userFreq.batchImportMerge(entries) }
            val refreshed = withContext(Dispatchers.IO) { userFreq.getAllFrequencies() }
            _allData.value = refreshed
            return ImportOutcome(imported = imported, skipped = entries.size - imported)
        } finally {
            _isImporting.value = false
        }
    }

    private fun parseCSV(csv: String): List<Pair<String, Int>> {
        val entries = mutableListOf<Pair<String, Int>>()
        for (line in csv.split("\n")) {
            val trimmed = line.trim()
            if (trimmed.isEmpty()) continue
            val columns = CsvUtils.parseLine(trimmed)
            if (columns.size < 2) continue
            val word = columns[0].trim()
            val count = columns[1].trim().toIntOrNull() ?: continue
            if (word.isEmpty() || count <= 0) continue
            entries.add(word to count)
        }
        return entries
    }
}
