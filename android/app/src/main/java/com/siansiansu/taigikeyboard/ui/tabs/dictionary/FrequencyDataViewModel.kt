package com.siansiansu.taigikeyboard.ui.tabs.dictionary

import android.app.Application
import android.net.Uri
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.siansiansu.taigikeyboard.ime.core.CompositionRoot
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.dictionary.DictionaryCsvCodec
import com.siansiansu.taigikeyboard.ime.text.composing.UserFrequencyService
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

    // One displayed frequency row: a (word, tl) reading + its count. R5 (#7):
    // identity is the pair, so 一字多音 (重/tāng vs 重/tîng) are distinct rows.
    data class FrequencyListItem(
        val word: String,
        val tl: String,
        val count: Int,
    )

    private val prefs = PrefHelper(application)
    private val userFreq: UserFrequencyService = CompositionRoot.shared(application).userFreq

    private val _allData = MutableStateFlow<List<FrequencyListItem>>(emptyList())
    val allData: StateFlow<List<FrequencyListItem>> = _allData.asStateFlow()

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
            val rows = withContext(Dispatchers.IO) { userFreq.getAllFrequencyRows() }
            _allData.value = rows.toListItems()
            _isLoading.value = false
        }
    }

    private fun List<Triple<String, String, Int>>.toListItems(): List<FrequencyListItem> =
        map { (word, tl, count) -> FrequencyListItem(word, tl, count) }

    fun deleteWord(
        word: String,
        tl: String,
    ) {
        viewModelScope.launch {
            withContext(Dispatchers.IO) { userFreq.deleteWord(word, tl) }
            _allData.value = _allData.value.filter { !(it.word == word && it.tl == tl) }
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
            DictionaryCsvCodec.encodeFrequencyCSV(userFreq.getAllFrequencyRows())
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
            // 3-column rows carry the reading; legacy 2-column rows decode to
            // tl="" (the tolerant fallback bucket, #7). Upsert is
            // ON CONFLICT(word, tl), so each (漢字, 羅馬字) reading merges
            // into its own bucket.
            val entries = DictionaryCsvCodec.decodeFrequencyCSV(csvString)
            val imported = withContext(Dispatchers.IO) { userFreq.batchImportMerge(entries) }
            val refreshed = withContext(Dispatchers.IO) { userFreq.getAllFrequencyRows() }
            _allData.value = refreshed.toListItems()
            return ImportOutcome(imported = imported, skipped = entries.size - imported)
        } finally {
            _isImporting.value = false
        }
    }
}
