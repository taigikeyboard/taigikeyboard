package com.siansiansu.taigikeyboard.ui.tabs.dictionary

import android.app.Application
import android.util.Log
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.engine.LexiconBridge
import com.siansiansu.taigikeyboard.ime.core.CompositionRoot
import com.siansiansu.taigikeyboard.ime.core.Outcome
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.settings.InputMode
import com.siansiansu.taigikeyboard.ime.dictionary.CustomDictionaryDerivation
import com.siansiansu.taigikeyboard.ime.dictionary.DictionarySearchResult
import com.siansiansu.taigikeyboard.ime.dictionary.DictionarySource
import kotlinx.coroutines.FlowPreview
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.debounce
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.launch

// ViewModel for dictionary search in Tab 3 (詞庫)
class DictionarySearchViewModel(
    application: Application,
) : AndroidViewModel(application) {
    companion object {
        private const val TAG = "DictionarySearchVM"
        private const val SEARCH_DEBOUNCE_MILLIS = 300L
        private const val SEARCH_RESULT_LIMIT = 20
    }

    private val _searchText = MutableStateFlow("")
    val searchText: StateFlow<String> = _searchText.asStateFlow()

    private val _results = MutableStateFlow<List<DictionarySearchResult>>(emptyList())
    val results: StateFlow<List<DictionarySearchResult>> = _results.asStateFlow()

    private val _isSearching = MutableStateFlow(false)
    val isSearching: StateFlow<Boolean> = _isSearching.asStateFlow()

    private val prefs = PrefHelper(application)
    private val root = CompositionRoot.shared(application)

    init {
        observeSearchText()
    }

    fun updateSearchText(text: String) {
        _searchText.value = text
    }

    @OptIn(FlowPreview::class)
    private fun observeSearchText() {
        viewModelScope.launch {
            _searchText
                .debounce(SEARCH_DEBOUNCE_MILLIS)
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
            val inputMode =
                when (prefs.inputMode) {
                    "poj" -> InputMode.POJ
                    else -> InputMode.TL
                }

            // Kotlin `Char.code` is 16-bit (UTF-16 code unit), so any inline
            // CJK range check fails to match supplementary-plane codepoints.
            // Route through Rust for the canonical 6-range coverage. See
            // INVARIANT_LEX_INPUT_CLASSIFICATION_HANZI_RANGE.
            val isCJK = LexiconBridge.isHanzi(query)

            if (BuildConfig.DEBUG) {
                Log.d(TAG, "[SEARCH] query='$query' isCJK=$isCJK inputMode=$inputMode")
            }

            // Resolve filter bitmask + enabled-source set ONCE per query and
            // hand both down the pipeline (mask into LexiconService, codes
            // into retag). Splitting the snapshot would let toggle changes
            // mid-search produce a mask/badge mismatch (Codex pre-impl
            // BLOCK 6).
            val toggles = LexiconBridge.DictionaryToggles.from(prefs)
            val filters = LexiconBridge.dictionaryFilters(toggles)

            val outcome =
                if (isCJK) {
                    root.lexicon.searchByHanzi(
                        input = query,
                        inputMode = inputMode,
                        filterBitmask = filters.dictionaryFilterBitmask,
                        limit = SEARCH_RESULT_LIMIT,
                    )
                } else {
                    root.lexicon.searchWithSources(
                        input = query,
                        inputMode = inputMode,
                        filterBitmask = filters.dictionaryFilterBitmask,
                        limit = SEARCH_RESULT_LIMIT,
                    )
                }
            val searchResults =
                when (outcome) {
                    is Outcome.Success -> {
                        outcome.value
                    }

                    is Outcome.Failure -> {
                        if (BuildConfig.DEBUG) {
                            Log.w(TAG, "[SEARCH] ${if (isCJK) "hanzi" else "roman"} path failed: ${outcome.error}")
                        }
                        _results.value = emptyList()
                        _isSearching.value = false
                        return
                    }
                }

            if (BuildConfig.DEBUG) {
                Log.d(TAG, "[SEARCH] ${if (isCJK) "hanzi" else "roman"} path returned ${searchResults.size} results")
            }

            val customResults = searchCustomDictionary(query, isCJK)

            if (BuildConfig.DEBUG) {
                Log.d(TAG, "[SEARCH] custom dictionary returned ${customResults.size} results")
            }

            // Sort: KAUTIAN (教育部) first, then by frequency
            val sorted =
                searchResults.sortedWith(
                    compareByDescending<DictionarySearchResult> { DictionarySource.KAUTIAN in it.sources }
                        .thenByDescending { it.frequency },
                )
            val filtered =
                sorted.map { result ->
                    result.copy(sources = result.sources.filter { it in filters.enabledSources })
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

    // Non-CJK custom dictionary search (matching iOS behavior)
    private suspend fun searchCustomDictionary(
        query: String,
        isCJK: Boolean,
    ): List<DictionarySearchResult> {
        if (isCJK) return emptyList()
        return try {
            val isToneAware = query.any { it.isDigit() }
            val searchPrefix =
                if (isToneAware) {
                    query.lowercase().replace("-", "").replace(" ", "")
                } else {
                    CustomDictionaryDerivation.generateNotone(query)
                }
            root.customDict
                .search(
                    prefix = searchPrefix,
                    isToneAware = isToneAware,
                    limit = SEARCH_RESULT_LIMIT,
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
}
