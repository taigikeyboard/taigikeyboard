package com.siansiansu.taigikeyboard.ui.tabs.dictionary

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.SavedStateHandle
import androidx.lifecycle.viewModelScope
import com.siansiansu.taigikeyboard.engine.proto.LearningRecord
import com.siansiansu.taigikeyboard.engine.proto.LearningRecordKind
import com.siansiansu.taigikeyboard.engine.proto.LearningRecordOrder
import com.siansiansu.taigikeyboard.engine.proto.LearningRecords
import com.siansiansu.taigikeyboard.ime.core.CompositionRoot
import com.siansiansu.taigikeyboard.ime.dictionary.UserDataClient
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import java.time.Instant
import java.time.ZoneId

/** The intent extra (a [LearningRecordKind] name) naming the one kind the page lists. */
const val LEARNING_RECORDS_KIND_ARG = "learningRecordsKind"

/** Rows per engine page. */
internal const val LEARNING_RECORDS_PAGE_SIZE = 100

/** How long the filter must stay unchanged before the engine is asked — the desktop's `FILTER_SETTLE`. */
internal const val LEARNING_RECORDS_FILTER_SETTLE_MILLIS = 200L

/**
 * The largest count the edit dialog accepts. CROSS-PLATFORM INVARIANT — the
 * engine clamps to the same (`engine/userdata/src/learning_records.rs`
 * `MAX_COUNT`).
 */
internal const val MAX_LEARNING_RECORD_COUNT = 1_000_000L

/** A read [LearningRecordsViewModel.retry] can repeat: the page after the loaded rows, or the list from the first row. */
private enum class FailedRead { NEXT_PAGE, LIST }

/** Why the screen shows a dialog. */
sealed interface LearningRecordsMessage {
    /** The list could not be read; [detail] is why, in the engine's words. */
    data class ReadFailed(
        val detail: String,
    ) : LearningRecordsMessage

    /** A count edit, a delete or an add failed — an add the engine refused included. */
    data class WriteFailed(
        val detail: String,
    ) : LearningRecordsMessage

    /** The row was no longer stored (deleted, evicted, its id reused) — said, not a failure. */
    data object Gone : LearningRecordsMessage

    /** The row's word is in the custom dictionary now; a learned phrase is gone from this list, a frequency row stays. */
    data object AddedToCustomDictionary : LearningRecordsMessage
}

data class LearningRecordsState(
    val order: LearningRecordOrder = LearningRecordOrder.LEARNING_RECORD_ORDER_MOST_USED,
    /** The search box as typed; the engine gets it trimmed. */
    val filter: String = "",
    /** The rows loaded so far, from the first. */
    val records: List<LearningRecord> = emptyList(),
    /** Every row of the page's kind. */
    val total: Int = 0,
    /** The rows [filter] matches — how far loading more can go. */
    val matchingTotal: Int = 0,
    val isLoading: Boolean = true,
    /** The last read failed, so neither "nothing learned yet" nor "no results" is known. */
    val hasReadFailed: Boolean = false,
    val message: LearningRecordsMessage? = null,
) {
    val canLoadMore: Boolean get() = records.size < matchingTotal
}

// ViewModel for LearningRecordsScreen — one kind of learned row in one order, filtered and paged
// by the engine (docs/architecture/learning-records-page-roadmap.md); the client runs every
// request on Dispatchers.IO.
class LearningRecordsViewModel internal constructor(
    application: Application,
    private val userData: UserDataClient,
    val kind: LearningRecordKind,
) : AndroidViewModel(application) {
    /** Built by the activity's default factory; the intent extras are [savedState]'s defaults. */
    constructor(application: Application, savedState: SavedStateHandle) : this(
        application,
        CompositionRoot.shared(application).userData,
        LearningRecordKind.valueOf(requireNotNull(savedState.get<String>(LEARNING_RECORDS_KIND_ARG)) { "no $LEARNING_RECORDS_KIND_ARG extra" }),
    )

    private val _state = MutableStateFlow(LearningRecordsState())
    val state: StateFlow<LearningRecordsState> = _state.asStateFlow()

    /**
     * The one list request in flight, a filter's settle delay included. A newer
     * request cancels it, and a cancelled request never lands (the client's
     * `withContext` resumes a cancelled caller with `CancellationException`),
     * so the newest order / filter always wins.
     */
    private var loadJob: Job? = null

    /** Which read [retry] repeats; set by the read that failed. */
    private var failedRead = FailedRead.LIST

    init {
        require(kind != LearningRecordKind.UNRECOGNIZED) { "Learning Records cannot list $kind" }
        reload()
    }

    fun selectOrder(order: LearningRecordOrder) {
        if (order == _state.value.order) return
        // The rows shown are in the old order, so the list starts empty.
        _state.update { it.copy(order = order, records = emptyList(), total = 0, matchingTotal = 0) }
        reload()
    }

    fun updateFilter(filter: String) {
        _state.update { it.copy(filter = filter) }
        reload(settleMillis = LEARNING_RECORDS_FILTER_SETTLE_MILLIS)
    }

    /**
     * The next page, when the list has more and no request is in flight. A
     * failed page waits for [retry]: scrolling never asks again on its own.
     */
    fun loadMore() {
        val shown = _state.value
        if (loadJob?.isActive == true || shown.hasReadFailed || !shown.canLoadMore) return
        _state.update { it.copy(isLoading = true) }
        loadJob = viewModelScope.launch { appendPage(offset = shown.records.size) }
    }

    /** Repeats the read that failed: the next page, or the listed rows from the first. */
    fun retry() {
        if (!_state.value.hasReadFailed) return
        _state.update { it.copy(hasReadFailed = false) }
        when (failedRead) {
            FailedRead.NEXT_PAGE -> loadMore()
            FailedRead.LIST -> refresh()
        }
    }

    fun setCount(
        record: LearningRecord,
        count: Long,
    ) = write { goneUnless(userData.setLearningRecordCount(record, count) != null) }

    /** No confirmation, like deleting one custom word: the keyboard learns the row again on the next pick. */
    fun delete(record: LearningRecord) = write { goneUnless(userData.deleteLearningRecord(record)) }

    /** Offered where `canAddToCustomDictionary` — the engine forgets a learned phrase, keeps a frequency row; a refusal keeps the row and is said. */
    fun addToCustomDictionary(record: LearningRecord) =
        write {
            userData.addLearningRecordToCustomDictionary(record)
            LearningRecordsMessage.AddedToCustomDictionary
        }

    /**
     * Reads the rows listed so far again from the first, one engine page at a
     * time, keeping order and filter: the keyboard may have learned,
     * moved or evicted rows since they were read.
     */
    fun refresh() {
        reload(rows = maxOf(LEARNING_RECORDS_PAGE_SIZE, _state.value.records.size))
    }

    fun dismissMessage() {
        _state.update { it.copy(message = null) }
    }

    /**
     * Runs [request] — answering the message to show, if any — then reads
     * every row loaded so far again: the row moved, went, or was never there.
     */
    private fun write(request: suspend () -> LearningRecordsMessage?) {
        viewModelScope.launch {
            val message =
                try {
                    request()
                } catch (e: CancellationException) {
                    throw e
                } catch (e: Exception) {
                    LearningRecordsMessage.WriteFailed(e.message.orEmpty())
                }
            if (message != null) _state.update { it.copy(message = message) }
            refresh()
        }
    }

    /** Lists the first [rows] rows again, after [settleMillis]. */
    private fun reload(
        settleMillis: Long = 0,
        rows: Int = LEARNING_RECORDS_PAGE_SIZE,
    ) {
        loadJob?.cancel()
        _state.update { it.copy(isLoading = true) }
        loadJob =
            viewModelScope.launch {
                delay(settleMillis)
                readFromStart(rows)
            }
    }

    /**
     * Reads the first [rows] rows, one engine page (at most
     * [LEARNING_RECORDS_PAGE_SIZE]) at a time, and replaces the list with them
     * in one go. A chunk the engine served from an earlier offset (the matches
     * shrank under it) is the last page that exists: it overrides the rows
     * read from that offset on.
     */
    private suspend fun readFromStart(rows: Int) {
        val asked = _state.value
        val records = mutableListOf<LearningRecord>()
        while (true) {
            val offset = records.size
            val page = read(asked, offset, minOf(LEARNING_RECORDS_PAGE_SIZE, rows - offset), FailedRead.LIST) ?: return
            if (page.offset < offset) records.subList(page.offset, offset).clear()
            records += page.recordsList
            val isDone = page.offset != offset || page.recordsCount == 0 || records.size >= rows || records.size >= page.matchingTotal
            if (isDone) {
                land(records = records, page = page)
                return
            }
        }
    }

    /**
     * Appends the page at [offset]. A page the engine served from an earlier
     * offset would overlap the list, so the loaded rows are read again instead.
     */
    private suspend fun appendPage(offset: Int) {
        val asked = _state.value
        val page = read(asked, offset, LEARNING_RECORDS_PAGE_SIZE, FailedRead.NEXT_PAGE) ?: return
        if (page.offset != offset) {
            readFromStart(rows = offset)
            return
        }
        land(records = asked.records + page.recordsList, page = page)
    }

    /** One engine page, or `null` after the failure is shown (it waits for [retry]). */
    private suspend fun read(
        asked: LearningRecordsState,
        offset: Int,
        limit: Int,
        whenFailed: FailedRead,
    ): LearningRecords? =
        try {
            userData.listLearningRecords(kind, asked.order, asked.filter.trim(), limit, offset)
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            failedRead = whenFailed
            _state.update {
                it.copy(isLoading = false, hasReadFailed = true, message = LearningRecordsMessage.ReadFailed(e.message.orEmpty()))
            }
            null
        }

    private fun land(
        records: List<LearningRecord>,
        page: LearningRecords,
    ) {
        _state.update {
            it.copy(
                // A row the keyboard moved between two pages may come twice; it is listed once.
                records = records.distinctBy { record -> record.id },
                total = page.total,
                matchingTotal = page.matchingTotal,
                isLoading = false,
                hasReadFailed = false,
            )
        }
    }
}

/** [LearningRecordsMessage.Gone] when the write found no stored row. */
private fun goneUnless(isStored: Boolean): LearningRecordsMessage? = if (isStored) null else LearningRecordsMessage.Gone

/**
 * The day a row was last used, `yyyy-MM-dd` in [zone] — the desktop's
 * `last_used_label`. Empty when the store held no readable time.
 */
internal fun learningRecordLastUsedLabel(
    lastUsedMs: Long,
    zone: ZoneId = ZoneId.systemDefault(),
): String =
    if (lastUsedMs <= 0) {
        ""
    } else {
        Instant
            .ofEpochMilli(lastUsedMs)
            .atZone(zone)
            .toLocalDate()
            .toString()
    }
