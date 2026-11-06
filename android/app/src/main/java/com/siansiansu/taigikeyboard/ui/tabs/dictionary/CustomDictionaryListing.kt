package com.siansiansu.taigikeyboard.ui.tabs.dictionary

import com.siansiansu.taigikeyboard.ime.dictionary.CustomDictionaryPage
import com.siansiansu.taigikeyboard.ime.dictionary.CustomDictionaryWord
import com.siansiansu.taigikeyboard.ime.dictionary.UserDataClient
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

data class CustomDictionaryListState(
    /** The search box as typed; the engine trims it. */
    val filter: String = "",
    /** The rows loaded so far, from the first. */
    val entries: List<CustomDictionaryWord> = emptyList(),
    /** Every stored word: 0 is the empty dictionary, not an empty search. */
    val total: Int = 0,
    /** The rows [filter] matches — how far loading more can go. */
    val matchingTotal: Int = 0,
    /** Only until the first answer: later loads leave the rows on screen. */
    val isLoading: Boolean = true,
    /** The last read failed: the rows on screen stay, and nothing is asked again until a retry. */
    val hasReadFailed: Boolean = false,
    /** Bumped by every load that lands, so the screen asks for the next page again when the size did not change. */
    val landedLoads: Int = 0,
) {
    val canLoadMore: Boolean get() = entries.size < matchingTotal
}

/**
 * The custom dictionary list, filtered and paged by the engine as on the
 * desktop. A newer request cancels the one in flight, a filter's settle delay
 * included, and a cancelled request never lands, so the newest filter wins.
 */
class CustomDictionaryListing(
    private val scope: CoroutineScope,
    private val userData: UserDataClient,
    private val onReadFailed: (Exception) -> Unit,
) {
    private val _state = MutableStateFlow(CustomDictionaryListState())
    val state: StateFlow<CustomDictionaryListState> = _state.asStateFlow()

    private var loadJob: Job? = null

    fun updateFilter(filter: String) {
        _state.update { it.copy(filter = filter) }
        reload(settleMillis = LEARNING_RECORDS_FILTER_SETTLE_MILLIS, rows = LEARNING_RECORDS_PAGE_SIZE)
    }

    /** The next page, when the list has more and no request is in flight. */
    fun loadMore() {
        val shown = _state.value
        if (loadJob?.isActive == true || shown.hasReadFailed || !shown.canLoadMore) return
        loadJob = scope.launch { appendPage(offset = shown.entries.size) }
    }

    /** Reads the listed rows again, so the list keeps its place; a failed read's retry too. */
    fun refresh() = reload(settleMillis = 0, rows = maxOf(LEARNING_RECORDS_PAGE_SIZE, _state.value.entries.size))

    private fun reload(
        settleMillis: Long,
        rows: Int,
    ) {
        loadJob?.cancel()
        loadJob =
            scope.launch {
                delay(settleMillis)
                readFromStart(rows)
            }
    }

    /** Lists the first [rows] rows in one request; a failed read keeps the rows on screen. */
    private suspend fun readFromStart(rows: Int) {
        val page = read(_state.value.filter, rows, offset = 0) ?: return
        land(page.words, page)
    }

    /** Appends the page at [offset]; it overrides the rows from where it really starts ([CustomDictionaryPage.offset]). */
    private suspend fun appendPage(offset: Int) {
        val shown = _state.value
        val page = read(shown.filter, LEARNING_RECORDS_PAGE_SIZE, offset) ?: return
        land(shown.entries.take(page.offset) + page.words, page)
    }

    /** One engine page, or `null` after the failure is reported. */
    private suspend fun read(
        filter: String,
        limit: Int,
        offset: Int,
    ): CustomDictionaryPage? =
        try {
            userData.listCustomEntries(filter, limit, offset)
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            onReadFailed(e)
            _state.update { it.copy(isLoading = false, hasReadFailed = true) }
            null
        }

    private fun land(
        words: List<CustomDictionaryWord>,
        page: CustomDictionaryPage,
    ) {
        _state.update {
            it.copy(
                // A row edited while a page was read may come twice; it is listed once.
                entries = words.distinctBy { word -> word.id },
                total = page.total,
                matchingTotal = page.matchingTotal,
                isLoading = false,
                hasReadFailed = false,
                landedLoads = it.landedLoads + 1,
            )
        }
    }
}
