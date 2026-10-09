package com.siansiansu.taigikeyboard.ui.tabs.dictionary

import android.app.Application
import com.siansiansu.taigikeyboard.engine.proto.CustomDictionaryRefusal
import com.siansiansu.taigikeyboard.engine.proto.LearningRecord
import com.siansiansu.taigikeyboard.engine.proto.LearningRecordKind
import com.siansiansu.taigikeyboard.engine.proto.LearningRecordOrder
import com.siansiansu.taigikeyboard.engine.proto.LearningRecords
import com.siansiansu.taigikeyboard.ime.dictionary.StubUserDataClient
import com.siansiansu.taigikeyboard.ime.dictionary.UserDataException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import java.time.ZoneId
import java.time.ZoneOffset

/**
 * JVM tests for the Learning Records page's view model. The engine is faked
 * (no `.so` on the JVM); the fake pages like the engine does — the offset
 * pulled back to the last page that exists (`engine/userdata/src/paging.rs`).
 */
@OptIn(ExperimentalCoroutinesApi::class)
class LearningRecordsViewModelTest {
    private val dispatcher = StandardTestDispatcher()

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    private data class ListCall(
        val kind: LearningRecordKind,
        val order: LearningRecordOrder,
        val filter: String,
        val limit: Int,
        val offset: Int,
    )

    private class FakeLearningRecords(
        var rows: List<LearningRecord>,
    ) : StubUserDataClient() {
        val listCalls = mutableListOf<ListCall>()

        /** When set, every list answer waits in [held] until the test completes it. */
        var holdsAnswers = false
        val held = mutableListOf<CompletableDeferred<LearningRecords>>()
        var listFailure: Exception? = null

        val setCalls = mutableListOf<Pair<LearningRecord, Long>>()
        val deleteCalls = mutableListOf<LearningRecord>()
        val addCalls = mutableListOf<LearningRecord>()
        var isRowGone = false
        var writeFailure: Exception? = null

        override suspend fun listLearningRecords(
            kind: LearningRecordKind,
            order: LearningRecordOrder,
            filter: String,
            limit: Int,
            offset: Int,
        ): LearningRecords {
            listCalls += ListCall(kind, order, filter, limit, offset)
            listFailure?.let { throw it }
            if (holdsAnswers) return CompletableDeferred<LearningRecords>().also { held += it }.await()
            return page(kind, filter, limit, offset)
        }

        fun page(
            kind: LearningRecordKind,
            filter: String,
            limit: Int,
            offset: Int,
        ): LearningRecords {
            val ofKind = rows.filter { it.kind == kind }
            val matching = ofKind.filter { filter.isEmpty() || it.text.contains(filter) || it.tl.contains(filter) }
            val served = if (offset > 0 && offset >= matching.size) (maxOf(matching.size - 1, 0) / limit) * limit else offset
            return LearningRecords
                .newBuilder()
                .addAllRecords(matching.drop(served).take(limit))
                .setTotal(ofKind.size)
                .setMatchingTotal(matching.size)
                .setOffset(served)
                .build()
        }

        override suspend fun setLearningRecordCount(
            record: LearningRecord,
            count: Long,
        ): LearningRecord? {
            setCalls += record to count
            writeFailure?.let { throw it }
            return if (isRowGone) null else record.toBuilder().setCount(count).build()
        }

        override suspend fun deleteLearningRecord(record: LearningRecord): Boolean {
            deleteCalls += record
            writeFailure?.let { throw it }
            return !isRowGone
        }

        /** Like the engine: a learned phrase leaves the list once added, a frequency row stays; a refusal keeps it. */
        override suspend fun addLearningRecordToCustomDictionary(record: LearningRecord) {
            addCalls += record
            writeFailure?.let { throw it }
            if (record.kind == LearningRecordKind.LEARNING_RECORD_KIND_LEARNED_PHRASE) {
                rows = rows.filterNot { it.id == record.id }
            }
        }
    }

    private fun record(
        id: Long,
        text: String,
        kind: LearningRecordKind = LearningRecordKind.LEARNING_RECORD_KIND_FREQUENCY,
    ): LearningRecord =
        LearningRecord
            .newBuilder()
            .setKind(kind)
            .setId(id)
            .setText(text)
            .setTl("tl$id")
            .setCount(3)
            .build()

    /** [count] frequency rows, ids 1..count, texts `w1`, `w2`, … */
    private fun frequencyRows(count: Int) = (1..count).map { record(it.toLong(), "w$it") }

    private fun viewModel(
        client: FakeLearningRecords,
        kind: LearningRecordKind = frequency,
    ) = LearningRecordsViewModel(Application(), client, kind)

    private val frequency = LearningRecordKind.LEARNING_RECORD_KIND_FREQUENCY
    private val phrase = LearningRecordKind.LEARNING_RECORD_KIND_LEARNED_PHRASE
    private val mostUsed = LearningRecordOrder.LEARNING_RECORD_ORDER_MOST_USED
    private val mostRecent = LearningRecordOrder.LEARNING_RECORD_ORDER_MOST_RECENT

    @Test
    fun `the page opens on word frequency, most used, one engine page, and appends the next`() =
        runTest(dispatcher) {
            val client = FakeLearningRecords(frequencyRows(150))
            val model = viewModel(client)
            advanceUntilIdle()

            assertEquals(listOf(ListCall(frequency, mostUsed, "", 100, 0)), client.listCalls)
            assertEquals(100, model.state.value.records.size)
            assertEquals(150, model.state.value.matchingTotal)
            assertTrue(model.state.value.canLoadMore)

            model.loadMore()
            advanceUntilIdle()

            assertEquals(ListCall(frequency, mostUsed, "", 100, 100), client.listCalls.last())
            assertEquals(
                (1L..150L).toList(),
                model.state.value.records
                    .map { it.id },
            )
            assertFalse(model.state.value.canLoadMore)

            model.loadMore()
            advanceUntilIdle()
            assertEquals("nothing more to ask for", 2, client.listCalls.size)
        }

    @Test
    fun `an order change drops the answer still in flight for the old order`() =
        runTest(dispatcher) {
            val client = FakeLearningRecords(frequencyRows(2))
            client.holdsAnswers = true
            val model = viewModel(client)
            advanceUntilIdle()

            model.selectOrder(mostRecent)
            advanceUntilIdle()
            assertEquals(mostRecent, client.listCalls.last().order)

            client.held[1].complete(
                client
                    .page(frequency, "", 100, 0)
                    .toBuilder()
                    .removeRecords(1)
                    .setMatchingTotal(1)
                    .build(),
            )
            client.held[0].complete(client.page(frequency, "", 100, 0))
            advanceUntilIdle()

            assertEquals(
                listOf(1L),
                model.state.value.records
                    .map { it.id },
            )
            assertEquals(mostRecent, model.state.value.order)
            assertFalse(model.state.value.isLoading)
        }

    @Test
    fun `a learned-phrases page lists only its own kind`() =
        runTest(dispatcher) {
            val client = FakeLearningRecords(frequencyRows(2) + record(9, "台灣", phrase))
            val model = viewModel(client, phrase)
            advanceUntilIdle()

            assertEquals(listOf(ListCall(phrase, mostUsed, "", 100, 0)), client.listCalls)
            assertEquals(
                listOf(9L),
                model.state.value.records
                    .map { it.id },
            )
        }

    @Test(expected = IllegalArgumentException::class)
    fun `an unrecognized kind is not a phone page`() {
        viewModel(FakeLearningRecords(emptyList()), LearningRecordKind.UNRECOGNIZED)
    }

    @Test
    fun `the filter goes to the engine trimmed once typing settles and lists from the first row`() =
        runTest(dispatcher) {
            val client = FakeLearningRecords(frequencyRows(150))
            val model = viewModel(client)
            advanceUntilIdle()
            model.loadMore()
            advanceUntilIdle()

            model.updateFilter("w")
            model.updateFilter(" w1 ")
            advanceTimeBy(LEARNING_RECORDS_FILTER_SETTLE_MILLIS - 1)
            assertEquals("not before the box settles", 2, client.listCalls.size)

            advanceUntilIdle()

            assertEquals(listOf(ListCall(frequency, mostUsed, "w1", 100, 0)), client.listCalls.drop(2))
            // trace: "w1" matches w1, w10..w19, w100..w150 = 1 + 10 + 51 = 62 rows.
            assertEquals(62, model.state.value.matchingTotal)
            assertEquals(62, model.state.value.records.size)
            assertEquals(150, model.state.value.total)
        }

    @Test
    fun `an order change lists from the first row in the new order`() =
        runTest(dispatcher) {
            val client = FakeLearningRecords(frequencyRows(3))
            val model = viewModel(client)
            advanceUntilIdle()

            model.selectOrder(mostRecent)
            advanceUntilIdle()

            assertEquals(mostRecent, client.listCalls.last().order)
            assertEquals(0, client.listCalls.last().offset)
            assertEquals(3, model.state.value.records.size)
        }

    @Test
    fun `a count edit sends the listed row and reads every loaded row again one engine page at a time`() =
        runTest(dispatcher) {
            val client = FakeLearningRecords(frequencyRows(260))
            val model = viewModel(client)
            advanceUntilIdle()
            model.loadMore()
            advanceUntilIdle()
            model.loadMore()
            advanceUntilIdle()
            assertEquals(260, model.state.value.records.size)
            val listed = model.state.value.records[120]
            val before = client.listCalls.size

            model.setCount(listed, 40)
            advanceUntilIdle()

            assertEquals(listOf(listed to 40L), client.setCalls)
            // trace: 260 loaded → chunks (100, 0), (100, 100), (60, 200); never one 260-row request.
            assertEquals(
                listOf(
                    ListCall(frequency, mostUsed, "", 100, 0),
                    ListCall(frequency, mostUsed, "", 100, 100),
                    ListCall(frequency, mostUsed, "", 60, 200),
                ),
                client.listCalls.drop(before),
            )
            assertEquals(
                (1L..260L).toList(),
                model.state.value.records
                    .map { it.id },
            )
            assertNull(model.state.value.message)
        }

    @Test
    fun `a re-read after the matches shrank stops at the last matching row`() =
        runTest(dispatcher) {
            val client = FakeLearningRecords(frequencyRows(250))
            val model = viewModel(client)
            advanceUntilIdle()
            model.loadMore()
            advanceUntilIdle()
            model.loadMore()
            advanceUntilIdle()
            client.rows = frequencyRows(180)
            val before = client.listCalls.size

            model.delete(model.state.value.records[0])
            advanceUntilIdle()

            // trace: chunks (100, 0) → 100 rows; (100, 100) → rows 101..180, 180 matches → done.
            assertEquals(
                listOf(ListCall(frequency, mostUsed, "", 100, 0), ListCall(frequency, mostUsed, "", 100, 100)),
                client.listCalls.drop(before),
            )
            assertEquals(
                (1L..180L).toList(),
                model.state.value.records
                    .map { it.id },
            )
            assertFalse(model.state.value.canLoadMore)
        }

    @Test
    fun `a failed next page waits for a retry, which asks for the same page`() =
        runTest(dispatcher) {
            val client = FakeLearningRecords(frequencyRows(150))
            val model = viewModel(client)
            advanceUntilIdle()
            client.listFailure = UserDataException.EngineUnavailable("learningRecordsList")

            model.loadMore()
            advanceUntilIdle()
            assertTrue(model.state.value.hasReadFailed)
            assertEquals(100, model.state.value.records.size)

            model.loadMore()
            advanceUntilIdle()
            assertEquals("scrolling does not ask again after a failure", 2, client.listCalls.size)

            client.listFailure = null
            model.retry()
            advanceUntilIdle()

            assertEquals(ListCall(frequency, mostUsed, "", 100, 100), client.listCalls.last())
            assertEquals(150, model.state.value.records.size)
            assertFalse(model.state.value.hasReadFailed)
        }

    @Test
    fun `a retry after a failed re-read lists the loaded rows again from the first, not the next page`() =
        runTest(dispatcher) {
            val client = FakeLearningRecords(frequencyRows(250))
            val model = viewModel(client)
            advanceUntilIdle()
            model.loadMore()
            advanceUntilIdle()
            client.listFailure = UserDataException.EngineUnavailable("learningRecordsList")

            model.delete(model.state.value.records[0])
            advanceUntilIdle()
            assertTrue(model.state.value.hasReadFailed)
            assertEquals("the stale rows stay shown", 200, model.state.value.records.size)

            client.listFailure = null
            val before = client.listCalls.size
            model.retry()
            advanceUntilIdle()

            // trace: 200 loaded → chunks (100, 0), (100, 100); a next-page retry would ask (100, 200).
            assertEquals(
                listOf(ListCall(frequency, mostUsed, "", 100, 0), ListCall(frequency, mostUsed, "", 100, 100)),
                client.listCalls.drop(before),
            )
            assertEquals(
                (1L..200L).toList(),
                model.state.value.records
                    .map { it.id },
            )
            assertFalse(model.state.value.hasReadFailed)
        }

    @Test
    fun `a refresh lists what the keyboard learned meanwhile, the loaded range from the first row`() =
        runTest(dispatcher) {
            val client = FakeLearningRecords(frequencyRows(150))
            val model = viewModel(client, phrase)
            advanceUntilIdle()
            model.selectOrder(mostRecent)
            advanceUntilIdle()
            model.updateFilter(" w ")
            advanceUntilIdle()
            client.rows = frequencyRows(150) + (1L..120L).map { record(1000 + it, "w台$it", phrase) }
            val before = client.listCalls.size

            model.refresh()
            advanceUntilIdle()

            // trace: 0 rows listed → one page (100, 0), kind / order / filter kept; 120 matches → load more stays possible.
            assertEquals(listOf(ListCall(phrase, mostRecent, "w", 100, 0)), client.listCalls.drop(before))
            assertEquals(100, model.state.value.records.size)
            assertEquals(120, model.state.value.matchingTotal)
        }

    @Test
    fun `a refresh re-reads every loaded row one engine page at a time`() =
        runTest(dispatcher) {
            val client = FakeLearningRecords(frequencyRows(150))
            val model = viewModel(client)
            advanceUntilIdle()
            model.loadMore()
            advanceUntilIdle()
            client.rows = frequencyRows(151)
            val before = client.listCalls.size

            model.refresh()
            advanceUntilIdle()

            // trace: 150 loaded → chunks (100, 0), (50, 100); the 151st row waits for load more.
            assertEquals(
                listOf(ListCall(frequency, mostUsed, "", 100, 0), ListCall(frequency, mostUsed, "", 50, 100)),
                client.listCalls.drop(before),
            )
            assertEquals(150, model.state.value.records.size)
            assertEquals(151, model.state.value.matchingTotal)
        }

    @Test
    fun `a retry after a failed first read lists from the first row`() =
        runTest(dispatcher) {
            val client = FakeLearningRecords(frequencyRows(3))
            client.listFailure = UserDataException.EngineUnavailable("learningRecordsList")
            val model = viewModel(client)
            advanceUntilIdle()

            client.listFailure = null
            model.retry()
            advanceUntilIdle()

            assertEquals(ListCall(frequency, mostUsed, "", 100, 0), client.listCalls.last())
            assertEquals(3, model.state.value.records.size)
            assertFalse(model.state.value.hasReadFailed)
        }

    @Test
    fun `a row already gone is said, not reported as a failure, and the list reloads`() =
        runTest(dispatcher) {
            val client = FakeLearningRecords(frequencyRows(2))
            val model = viewModel(client)
            advanceUntilIdle()
            client.isRowGone = true

            model.delete(model.state.value.records[0])
            advanceUntilIdle()

            assertEquals(LearningRecordsMessage.Gone, model.state.value.message)
            assertEquals(2, client.listCalls.size)

            model.dismissMessage()
            assertNull(model.state.value.message)
        }

    @Test
    fun `a failed write carries the engine's words and still reloads`() =
        runTest(dispatcher) {
            val client = FakeLearningRecords(frequencyRows(2))
            val model = viewModel(client)
            advanceUntilIdle()
            client.writeFailure = UserDataException.EngineUnavailable("learningRecordDelete")

            model.delete(model.state.value.records[0])
            advanceUntilIdle()

            assertEquals(
                LearningRecordsMessage.WriteFailed("the engine did not answer learningRecordDelete"),
                model.state.value.message,
            )
            assertEquals(2, client.listCalls.size)
        }

    @Test
    fun `an added learned phrase is said and leaves the reloaded list`() =
        runTest(dispatcher) {
            val client = FakeLearningRecords(listOf(record(7, "台灣", phrase), record(8, "食飯", phrase)))
            val model = viewModel(client, phrase)
            advanceUntilIdle()
            val listed = model.state.value.records[0]

            model.addToCustomDictionary(listed)
            advanceUntilIdle()

            assertEquals(listOf(listed), client.addCalls)
            assertEquals(LearningRecordsMessage.AddedToCustomDictionary, model.state.value.message)
            assertEquals(2, client.listCalls.size)
            assertEquals(
                listOf(8L),
                model.state.value.records
                    .map { it.id },
            )
        }

    @Test
    fun `an added frequency row is said and stays in the reloaded list`() =
        runTest(dispatcher) {
            val client = FakeLearningRecords(listOf(record(7, "台灣"), record(8, "食飯")))
            val model = viewModel(client)
            advanceUntilIdle()
            val listed = model.state.value.records[0]

            model.addToCustomDictionary(listed)
            advanceUntilIdle()

            assertEquals(listOf(listed), client.addCalls)
            assertEquals(LearningRecordsMessage.AddedToCustomDictionary, model.state.value.message)
            assertEquals(2, client.listCalls.size)
            assertEquals(
                listOf(7L, 8L),
                model.state.value.records
                    .map { it.id },
            )
        }

    @Test
    fun `a refused add carries the engine's words and keeps the row`() =
        runTest(dispatcher) {
            val client = FakeLearningRecords(listOf(record(7, "台灣", phrase)))
            val model = viewModel(client, phrase)
            advanceUntilIdle()
            client.writeFailure = UserDataException.Refused(CustomDictionaryRefusal.CUSTOM_DICTIONARY_REFUSAL_FULL, "the custom dictionary is full")

            model.addToCustomDictionary(model.state.value.records[0])
            advanceUntilIdle()

            assertEquals(LearningRecordsMessage.WriteFailed("the custom dictionary is full"), model.state.value.message)
            assertEquals(2, client.listCalls.size)
            assertEquals(
                listOf(7L),
                model.state.value.records
                    .map { it.id },
            )
        }

    @Test
    fun `a failed read is its own state, neither empty nor no results`() =
        runTest(dispatcher) {
            val client = FakeLearningRecords(emptyList())
            client.listFailure = UserDataException.EngineUnavailable("learningRecordsList")
            val model = viewModel(client)
            advanceUntilIdle()

            val state = model.state.value
            assertEquals(LearningRecordsMessage.ReadFailed("the engine did not answer learningRecordsList"), state.message)
            assertTrue(state.hasReadFailed)
            assertFalse(state.isLoading)
        }

    @Test
    fun `a later page served from an earlier offset reads the loaded rows again`() =
        runTest(dispatcher) {
            val client = FakeLearningRecords(frequencyRows(150))
            val model = viewModel(client)
            advanceUntilIdle()
            client.rows = frequencyRows(90)

            model.loadMore()
            advanceUntilIdle()

            // trace: offset 100 >= 90 matches → served (89 / 100) * 100 = 0 ≠ 100 → reload 100 rows from 0.
            assertEquals(ListCall(frequency, mostUsed, "", 100, 0), client.listCalls.last())
            assertEquals(
                (1L..90L).toList(),
                model.state.value.records
                    .map { it.id },
            )
            assertFalse(model.state.value.canLoadMore)
        }

    @Test
    fun `the last-used day is the viewer's calendar day`() {
        // trace: 2026-01-01T00:00:00Z = 1 767 225 600 s.
        val newYear = 1_767_225_600_000L
        assertEquals("2026-01-01", learningRecordLastUsedLabel(newYear, ZoneOffset.UTC))
        assertEquals("2026-01-01", learningRecordLastUsedLabel(newYear, ZoneId.of("Asia/Taipei")))
        assertEquals("2025-12-31", learningRecordLastUsedLabel(newYear, ZoneOffset.ofHours(-1)))
        assertEquals("no readable time", "", learningRecordLastUsedLabel(0, ZoneOffset.UTC))
    }
}
