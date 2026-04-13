package com.siansiansu.taigikeyboard.ui.tabs.tab3

import android.app.Application
import android.util.Log
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.dictionary.CustomDictionaryService
import com.siansiansu.taigikeyboard.ime.dictionary.DictionarySearchResult
import com.siansiansu.taigikeyboard.ime.dictionary.DictionarySource
import com.siansiansu.taigikeyboard.ime.dictionary.LexiconService
import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels
import kotlinx.coroutines.FlowPreview
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.debounce
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.launch

/**
 * ViewModel for dictionary search in Tab 3 (詞庫)
 */
class DictionarySearchViewModel(
    application: Application,
) : AndroidViewModel(application) {
    companion object {
        private const val TAG = "DictionarySearchVM"
    }

    private val _searchText = MutableStateFlow("")
    val searchText: StateFlow<String> = _searchText.asStateFlow()

    private val _results = MutableStateFlow<List<DictionarySearchResult>>(emptyList())
    val results: StateFlow<List<DictionarySearchResult>> = _results.asStateFlow()

    private val _isSearching = MutableStateFlow(false)
    val isSearching: StateFlow<Boolean> = _isSearching.asStateFlow()

    private val prefs = PrefHelper(application)

    init {
        observeSearchText()
    }

    // Build set of enabled dictionary sources from current preferences
    private fun buildEnabledSources(): Set<DictionarySource> {
        val sources = mutableSetOf(DictionarySource.DEV, DictionarySource.CUSTOM)
        if (prefs.moeDictEnabled) sources.add(DictionarySource.KAUTIAN)
        if (prefs.newwordDictEnabled) sources.add(DictionarySource.TAIGITV)
        if (prefs.kunggeDictEnabled) sources.add(DictionarySource.KUNGGE)
        if (prefs.itaigiDictEnabled) sources.add(DictionarySource.ITAIGI)
        if (prefs.taiwanJapanDictEnabled) sources.add(DictionarySource.TAIJIT)
        if (prefs.taiHuaDictEnabled) sources.add(DictionarySource.TAIHOA)
        if (prefs.taiwanPlantDictEnabled) sources.add(DictionarySource.SITBUT)
        if (prefs.sttiDictEnabled) sources.add(DictionarySource.STTI)
        if (prefs.khpooDictEnabled) sources.add(DictionarySource.KHPOO)
        if (prefs.khiin) sources.add(DictionarySource.KHIIN)
        if (prefs.lkkDictEnabled) sources.add(DictionarySource.LKK)
        return sources
    }

    fun updateSearchText(text: String) {
        _searchText.value = text
    }

    @OptIn(FlowPreview::class)
    private fun observeSearchText() {
        viewModelScope.launch {
            _searchText
                .debounce(300L)
                .distinctUntilChanged()
                .collect { query ->
                    val trimmed = query.trim()
                    if (trimmed.isEmpty()) {
                        _results.value = emptyList()
                        _isSearching.value = false
                    } else {
                        performSearch(trimmed)
                    }
                }
        }
    }

    private suspend fun performSearch(query: String) {
        _isSearching.value = true
        try {
            val context = getApplication<Application>()
            val inputMode =
                when (prefs.inputMode) {
                    "poj" -> ToneConverterModels.InputMode.POJ
                    else -> ToneConverterModels.InputMode.TL
                }

            // Detect CJK input and use hanzi search path
            val isCJK = query.any { it.code in 0x4E00..0x9FFF || it.code in 0x3400..0x4DBF || it.code in 0x20000..0x2A6DF }

            if (BuildConfig.DEBUG) {
                Log.d(TAG, "[SEARCH] query='$query' isCJK=$isCJK inputMode=$inputMode")
            }

            val searchResults =
                if (isCJK) {
                    LexiconService.searchByHanzi(
                        input = query,
                        inputMode = inputMode,
                        limit = 20,
                        context = context,
                    )
                } else {
                    LexiconService.searchWithSources(
                        input = query,
                        inputMode = inputMode,
                        limit = 20,
                        context = context,
                    )
                }

            if (BuildConfig.DEBUG) {
                Log.d(TAG, "[SEARCH] ${if (isCJK) "hanzi" else "roman"} path returned ${searchResults.size} results")
            }

            // Search custom dictionary for non-CJK input (matching iOS behavior)
            val customResults =
                if (isCJK) {
                    emptyList()
                } else {
                    try {
                        CustomDictionaryService.init(context)
                        val isToneAware = query.any { it.isDigit() }
                        val searchPrefix =
                            if (isToneAware) {
                                query.lowercase().replace("-", "").replace(" ", "")
                            } else {
                                CustomDictionaryService.generateNotone(query)
                            }
                        CustomDictionaryService
                            .search(
                                prefix = searchPrefix,
                                isToneAware = isToneAware,
                                limit = 20,
                            ).map { entry ->
                                DictionarySearchResult(
                                    id = -2,
                                    roman = entry.roman,
                                    tl = entry.roman,
                                    hanzi = entry.hanzi,
                                    frequency = Int.MAX_VALUE,
                                    sources = listOf(DictionarySource.CUSTOM),
                                )
                            }
                    } catch (e: Exception) {
                        if (BuildConfig.DEBUG) {
                            Log.w(TAG, "[SEARCH] Custom dictionary query failed: ${e.message}", e)
                        }
                        emptyList()
                    }
                }

            if (BuildConfig.DEBUG) {
                Log.d(TAG, "[SEARCH] custom dictionary returned ${customResults.size} results")
            }

            // Sort: KAUTIAN (教育部) first, then by frequency
            val sorted =
                searchResults.sortedWith(
                    compareByDescending<DictionarySearchResult> { DictionarySource.KAUTIAN in it.sources }
                        .thenByDescending { it.frequency },
                )
            // Filter source tags to only show enabled dictionaries
            val enabledSources = buildEnabledSources()
            val filtered =
                sorted.map { result ->
                    result.copy(sources = result.sources.filter { it in enabledSources })
                }
            _results.value = customResults + filtered
            _isSearching.value = false
        } catch (e: Exception) {
            if (BuildConfig.DEBUG) {
                Log.e(TAG, "[SEARCH] Failed: ${e.message}", e)
            }
            _results.value = emptyList()
            _isSearching.value = false
        }
    }
}
