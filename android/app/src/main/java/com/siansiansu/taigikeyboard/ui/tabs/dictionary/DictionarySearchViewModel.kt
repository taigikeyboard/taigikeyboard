package com.siansiansu.taigikeyboard.ui.tabs.dictionary

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.siansiansu.taigikeyboard.ime.core.CompositionRoot
import com.siansiansu.taigikeyboard.ime.dictionary.DictionarySearchResult
import com.siansiansu.taigikeyboard.ime.dictionary.DictionarySearchService
import com.siansiansu.taigikeyboard.ime.settings.PrefHelper
import kotlinx.coroutines.FlowPreview
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.debounce
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.launch

// ViewModel for dictionary search on the Manage Dictionaries page: debounces the query and
// publishes the results; the search itself is `DictionarySearchService`.
class DictionarySearchViewModel(
    application: Application,
) : AndroidViewModel(application) {
    companion object {
        private const val SEARCH_DEBOUNCE_MILLIS = 300L
    }

    private val _searchText = MutableStateFlow("")
    val searchText: StateFlow<String> = _searchText.asStateFlow()

    private val _results = MutableStateFlow<List<DictionarySearchResult>>(emptyList())
    val results: StateFlow<List<DictionarySearchResult>> = _results.asStateFlow()

    private val _isSearching = MutableStateFlow(false)
    val isSearching: StateFlow<Boolean> = _isSearching.asStateFlow()

    private val root = CompositionRoot.shared(application)
    private val service =
        DictionarySearchService(
            lexicon = root.lexicon,
            settingsProvider = PrefHelper(application),
        )

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
        _results.value = service.search(query)
        _isSearching.value = false
    }
}
