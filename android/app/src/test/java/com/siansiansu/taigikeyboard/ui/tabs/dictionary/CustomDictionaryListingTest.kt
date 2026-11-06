package com.siansiansu.taigikeyboard.ui.tabs.dictionary

import com.siansiansu.taigikeyboard.ime.dictionary.CustomDictionaryPage
import com.siansiansu.taigikeyboard.ime.dictionary.CustomDictionaryWord
import com.siansiansu.taigikeyboard.ime.dictionary.StubUserDataClient
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * JVM tests for the custom dictionary list. The engine is faked (no `.so` on
 * the JVM); the fake pages like the engine does — the offset pulled back to
 * the last page that exists (`engine/userdata/src/paging.rs`).
 */
@OptIn(ExperimentalCoroutinesApi::class)
class CustomDictionaryListingTest {
    private data class ListCall(
        val filter: String,
        val limit: Int,
        val offset: Int,
    )

    private class FakeDictionary(
        var rows: List<CustomDictionaryWord>,
    ) : StubUserDataClient() {
        val calls = mutableListOf<ListCall>()
        var failure: Exception? = null

        /** When set, every answer waits in [held] until the test completes it. */
        var holdsAnswers = false
        val held = mutableListOf<CompletableDeferred<CustomDictionaryPage>>()

        override suspend fun listCustomEntries(
            filter: String,
            limit: Int,
            offset: Int,
        ): CustomDictionaryPage {
            calls += ListCall(filter, limit, offset)
            failure?.let { throw it }
            if (holdsAnswers) return CompletableDeferred<CustomDictionaryPage>().also { held += it }.await()
            return page(filter, limit, offset)
        }

        fun page(
            filter: String,
            limit: Int,
            offset: Int,
        ): CustomDictionaryPage {
            val matching = rows.filter { filter.isBlank() || it.roman.contains(filter.trim()) }
            val served = minOf(offset, maxOf(matching.size - 1, 0) / limit * limit)
            return CustomDictionaryPage(matching.drop(served).take(limit), rows.size, matching.size, served)
        }
    }

    private fun words(count: Int) = (0 until count).map { CustomDictionaryWord(id = "$it", roman = "word$it", hanji = "字") }

    private fun TestScope.listing(fake: FakeDictionary) = CustomDictionaryListing(this, fake) {}

    @Test
    fun `lists one page, then every row a page at a time`() =
        runTest {
            val fake = FakeDictionary(words(250))
            val list = listing(fake)

            list.refresh()
            advanceUntilIdle()
            assertEquals(100, list.state.value.entries.size)
            assertFalse(list.state.value.isLoading)
            assertTrue(list.state.value.canLoadMore)

            list.loadMore()
            advanceUntilIdle()
            list.loadMore()
            advanceUntilIdle()

            // trace: 250 rows, pages at 0 / 100 / 200 — row 101 is reachable with no filter.
            assertEquals(listOf(0, 100, 200), fake.calls.map { it.offset })
            assertEquals(250, list.state.value.entries.size)
            assertEquals(
                "100",
                list.state.value.entries[100]
                    .id,
            )
            assertFalse(list.state.value.canLoadMore)

            list.loadMore()
            advanceUntilIdle()
            assertEquals(3, fake.calls.size)
        }

    @Test
    fun `a filter goes to the engine once typing settles`() =
        runTest {
            val fake = FakeDictionary(words(250))
            val list = listing(fake)

            list.updateFilter("w")
            advanceTimeBy(LEARNING_RECORDS_FILTER_SETTLE_MILLIS - 1)
            list.updateFilter("word249")
            advanceUntilIdle()

            assertEquals(listOf(ListCall("word249", 100, 0)), fake.calls)
            assertEquals(
                listOf("249"),
                list.state.value.entries
                    .map { it.id },
            )
            assertEquals(250, list.state.value.total)
            assertEquals(1, list.state.value.matchingTotal)
        }

    @Test
    fun `refresh rereads the listed rows in one request`() =
        runTest {
            val fake = FakeDictionary(words(250))
            val list = listing(fake)
            list.refresh()
            advanceUntilIdle()
            list.loadMore()
            advanceUntilIdle()
            list.loadMore()
            advanceUntilIdle()
            fake.calls.clear()

            list.refresh()
            advanceUntilIdle()

            assertEquals(listOf(ListCall("", 250, 0)), fake.calls)
            assertEquals(250, list.state.value.entries.size)
        }

    @Test
    fun `an empty search keeps the stored total, an empty dictionary has none`() =
        runTest {
            val list = listing(FakeDictionary(words(3)))
            list.updateFilter("nothing")
            advanceUntilIdle()
            assertTrue(
                list.state.value.entries
                    .isEmpty(),
            )
            assertEquals(3, list.state.value.total)

            val empty = listing(FakeDictionary(emptyList()))
            empty.refresh()
            advanceUntilIdle()
            assertEquals(0, empty.state.value.total)
        }

    @Test
    fun `a page served from an earlier offset rereads the list`() =
        runTest {
            val fake = FakeDictionary(words(150))
            val list = listing(fake)
            list.refresh()
            advanceUntilIdle()

            // trace: 60 rows left, offset 100 asked → the fake serves (59 / 100) * 100 = 0.
            fake.rows = fake.rows.take(60)
            list.loadMore()
            advanceUntilIdle()

            assertEquals(60, list.state.value.entries.size)
            assertEquals(60, list.state.value.matchingTotal)
            assertEquals(
                60,
                list.state.value.entries
                    .distinctBy { it.id }
                    .size,
            )
        }

    @Test
    fun `a word added to a fully listed list is reached through the pulled back page`() =
        runTest {
            val fake = FakeDictionary(words(120))
            val list = listing(fake)
            list.refresh()
            advanceUntilIdle()
            list.loadMore()
            advanceUntilIdle()
            assertFalse(list.state.value.canLoadMore)

            fake.rows = listOf(CustomDictionaryWord(id = "new", roman = "new", hanji = "字")) + fake.rows
            list.refresh()
            advanceUntilIdle()
            assertEquals(120, list.state.value.entries.size)
            assertTrue(list.state.value.canLoadMore)

            // trace: 121 matches, limit 100 → last page 120 / 100 * 100 = 100, so offset 120 serves 100.
            list.loadMore()
            advanceUntilIdle()
            assertEquals(121, list.state.value.entries.size)
            assertEquals(
                121,
                list.state.value.entries
                    .distinctBy { it.id }
                    .size,
            )
            assertFalse(list.state.value.canLoadMore)
        }

    @Test
    fun `a failed read keeps the rows, and a retry resumes paging`() =
        runTest {
            val fake = FakeDictionary(words(150))
            val failures = mutableListOf<Exception>()
            val list = CustomDictionaryListing(this, fake) { failures += it }
            list.refresh()
            advanceUntilIdle()

            fake.failure = IllegalStateException("disk I/O error")
            list.loadMore()
            advanceUntilIdle()
            assertEquals(100, list.state.value.entries.size)
            assertTrue(list.state.value.hasReadFailed)

            list.refresh()
            advanceUntilIdle()
            assertEquals(100, list.state.value.entries.size)
            assertEquals(150, list.state.value.total)
            assertEquals(2, failures.size)

            fake.calls.clear()
            list.loadMore()
            advanceUntilIdle()
            assertTrue("nothing is asked until the retry", fake.calls.isEmpty())

            fake.failure = null
            list.refresh()
            advanceUntilIdle()
            assertFalse(list.state.value.hasReadFailed)
            list.loadMore()
            advanceUntilIdle()
            assertEquals(150, list.state.value.entries.size)
        }

    @Test
    fun `a newer filter drops the answer in flight`() =
        runTest {
            val fake = FakeDictionary(words(3))
            val list = listing(fake)
            fake.holdsAnswers = true
            list.refresh()
            advanceUntilIdle()
            val stale = fake.held.single()

            fake.holdsAnswers = false
            list.updateFilter("word1")
            advanceUntilIdle()
            stale.complete(fake.page("", 100, 0))
            advanceUntilIdle()

            assertEquals(
                listOf("1"),
                list.state.value.entries
                    .map { it.id },
            )
        }
}
