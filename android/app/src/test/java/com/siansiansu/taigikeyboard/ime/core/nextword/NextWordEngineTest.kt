package com.siansiansu.taigikeyboard.ime.core.nextword

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Pure-engine tests for A5-impl. Mirrors iOS
 * `NextWordEngineTests.swift` boundary hooks in `nextword-engine-boundary.md`
 * §10. Platform-harness tests (late-prediction-discarded + timer-leak)
 * deferred to A9 per §13.11 of the same doc.
 *
 * `INVARIANT_*` grep-visible labels follow in A9 per
 * `android-g9-coverage-matrix.md` §7–§8. These tests already pin the
 * behaviors the labels will wrap.
 */
class NextWordEngineTest {
    private val defaultSettings =
        NextWordEngineSettings(
            inputMode = "tl",
            isTranslateSwapped = false,
            isAssociationRecordingEnabled = true,
        )

    private fun input(
        nowMs: Long,
        settings: NextWordEngineSettings = defaultSettings,
    ) = NextWordDecisionInput(nowMs = nowMs, settings = settings)

    // region shouldRecordAssociation — association window

    @Test
    fun `shouldRecordAssociation returns true at 9_999ms delta`() {
        val state =
            NextWordPersistedState(
                lastSelectedWord = "a",
                lastSelectionTimeMs = 0,
            )
        assertTrue(NextWordEngine.shouldRecordAssociation(state, nowMs = 9_999L))
    }

    @Test
    fun `shouldRecordAssociation returns false at exactly 10_000ms delta`() {
        val state =
            NextWordPersistedState(
                lastSelectedWord = "a",
                lastSelectionTimeMs = 0,
            )
        assertFalse(NextWordEngine.shouldRecordAssociation(state, nowMs = 10_000L))
    }

    @Test
    fun `shouldRecordAssociation returns false on negative delta`() {
        val state =
            NextWordPersistedState(
                lastSelectedWord = "a",
                lastSelectionTimeMs = 100,
            )
        assertFalse(NextWordEngine.shouldRecordAssociation(state, nowMs = 50L))
    }

    @Test
    fun `shouldRecordAssociation returns false without lastSelectedWord`() {
        val state = NextWordPersistedState.initial
        assertFalse(NextWordEngine.shouldRecordAssociation(state, nowMs = 5_000L))
    }

    // endregion

    // region splitCompound + compoundAssociationPairs

    @Test
    fun `splitCompound splits on hyphen`() {
        assertEquals(listOf("tshit", "niu"), NextWordEngine.splitCompound("tshit-niu"))
    }

    @Test
    fun `splitCompound splits on whitespace (Android divergence from iOS)`() {
        assertEquals(listOf("tshit", "niu"), NextWordEngine.splitCompound("tshit niu"))
    }

    @Test
    fun `splitCompound drops empty parts from leading or trailing hyphen`() {
        assertEquals(listOf("a"), NextWordEngine.splitCompound("-a-"))
    }

    @Test
    fun `splitCompound returns empty list for empty input`() {
        assertTrue(NextWordEngine.splitCompound("").isEmpty())
    }

    @Test
    fun `compoundAssociationPairs emits sequential bigrams in order`() {
        val pairs = NextWordEngine.compoundAssociationPairs(displayText = "a-b-c", roman = "a-b-c")
        assertEquals(2, pairs.size)
        assertEquals("a", pairs[0].prev)
        assertEquals("b", pairs[0].next)
        assertEquals("b", pairs[1].prev)
        assertEquals("c", pairs[1].next)
    }

    @Test
    fun `compoundAssociationPairs returns empty when only one part`() {
        assertTrue(NextWordEngine.compoundAssociationPairs("a", "a").isEmpty())
    }

    // endregion

    // region isNoiseText

    @Test
    fun `isNoiseText flags pure digits`() {
        assertTrue(NextWordEngine.isNoiseText("123"))
    }

    @Test
    fun `isNoiseText flags punctuation chars`() {
        assertTrue(NextWordEngine.isNoiseText("。"))
        assertTrue(NextWordEngine.isNoiseText("!"))
        assertTrue(NextWordEngine.isNoiseText(","))
    }

    @Test
    fun `isNoiseText flags whitespace`() {
        assertTrue(NextWordEngine.isNoiseText(" "))
        assertTrue(NextWordEngine.isNoiseText("　"))
    }

    @Test
    fun `isNoiseText does not flag regular Han characters`() {
        assertFalse(NextWordEngine.isNoiseText("你好"))
    }

    @Test
    fun `isNoiseText does not flag romanization`() {
        assertFalse(NextWordEngine.isNoiseText("tsit"))
    }

    @Test
    fun `isNoiseText returns true on empty string`() {
        assertTrue(NextWordEngine.isNoiseText(""))
    }

    // endregion

    // region isSentenceEndPunctuation

    @Test
    fun `isSentenceEndPunctuation flags full stop variants`() {
        assertTrue(NextWordEngine.isSentenceEndPunctuation("。"))
        assertTrue(NextWordEngine.isSentenceEndPunctuation("."))
        assertTrue(NextWordEngine.isSentenceEndPunctuation("!"))
        assertTrue(NextWordEngine.isSentenceEndPunctuation("！"))
        assertTrue(NextWordEngine.isSentenceEndPunctuation("?"))
        assertTrue(NextWordEngine.isSentenceEndPunctuation("？"))
    }

    @Test
    fun `isSentenceEndPunctuation does not flag comma or semicolon`() {
        assertFalse(NextWordEngine.isSentenceEndPunctuation(","))
        assertFalse(NextWordEngine.isSentenceEndPunctuation(";"))
    }

    // endregion

    // region decide — backspace never records

    @Test
    fun `backspace does not emit recordAssociation effects`() {
        val state =
            NextWordPersistedState(
                lastSelectedWord = "a",
                lastSelectionTimeMs = 0,
            )
        val outcome =
            NextWordEngine.decide(
                intent = NextWordIntent.Backspace(lastChar = "b"),
                state = state,
                input = input(nowMs = 1_000L),
            )
        assertTrue(
            "backspace must not emit RecordAssociation",
            outcome.effects.none { it is NextWordOutcome.Effect.RecordAssociation },
        )
        assertTrue(
            "backspace must not emit RecordCompoundAssociations",
            outcome.effects.none { it is NextWordOutcome.Effect.RecordCompoundAssociations },
        )
    }

    @Test
    fun `backspace emits a single QueryPredictions effect with bumped generation`() {
        val state =
            NextWordPersistedState(
                lastSelectedWord = "a",
                currentGeneration = 5,
            )
        val outcome =
            NextWordEngine.decide(
                intent = NextWordIntent.Backspace(lastChar = "b"),
                state = state,
                input = input(nowMs = 100L),
            )
        val query =
            outcome.effects.filterIsInstance<NextWordOutcome.Effect.QueryPredictions>().single()
        assertEquals("b", query.word)
        assertEquals("", query.roman)
        assertEquals(6L, query.generation)
        assertEquals("QueryPredictions must carry the decision-time nowMs", 100L, query.nowMs)
        assertEquals(6L, outcome.newState.currentGeneration)
        assertEquals("b", outcome.newState.lastSelectedWord)
        assertNull(outcome.newState.lastSelectedRoman)
    }

    // endregion

    // region decide — sentence-end resets + bumps generation

    @Test
    fun `wordSelected with sentence-end punctuation resets state and bumps generation`() {
        val state =
            NextWordPersistedState(
                lastSelectedWord = "你好",
                lastSelectedRoman = "li-ho",
                lastSelectionTimeMs = 100,
                isShowing = true,
                currentGeneration = 7,
            )
        val outcome =
            NextWordEngine.decide(
                intent =
                    NextWordIntent.WordSelected(
                        text = "。",
                        roman = "",
                        requireRomanMode = false,
                        triggerPrediction = false,
                    ),
                state = state,
                input = input(nowMs = 1_000L),
            )
        assertNull(outcome.newState.lastSelectedWord)
        assertNull(outcome.newState.lastSelectedRoman)
        assertEquals(0L, outcome.newState.lastSelectionTimeMs)
        assertFalse(outcome.newState.isShowing)
        assertEquals(8L, outcome.newState.currentGeneration)
        assertTrue(outcome.effects.any { it is NextWordOutcome.Effect.CancelContextTimeout })
        assertTrue(
            "sentence-end with isShowing=true must emit ClearPredictionsUI",
            outcome.effects.any { it is NextWordOutcome.Effect.ClearPredictionsUI },
        )
    }

    @Test
    fun `wordSelected with noise but not sentence-end returns no effects`() {
        val state = NextWordPersistedState(lastSelectedWord = "a", currentGeneration = 3)
        val outcome =
            NextWordEngine.decide(
                intent =
                    NextWordIntent.WordSelected(
                        text = ",",
                        roman = "",
                        requireRomanMode = false,
                        triggerPrediction = true,
                    ),
                state = state,
                input = input(nowMs = 1_000L),
            )
        assertTrue(outcome.effects.isEmpty())
        assertEquals(
            "noise that is not sentence-end must leave generation unchanged",
            3L,
            outcome.newState.currentGeneration,
        )
    }

    // endregion

    // region decide — generation bumps on invalidating intents

    @Test
    fun `wordSelected bumps generation`() {
        val state = NextWordPersistedState(currentGeneration = 10)
        val outcome =
            NextWordEngine.decide(
                intent =
                    NextWordIntent.WordSelected(
                        text = "我",
                        roman = "gua",
                        requireRomanMode = false,
                        triggerPrediction = false,
                    ),
                state = state,
                input = input(nowMs = 500L),
            )
        assertEquals(11L, outcome.newState.currentGeneration)
    }

    @Test
    fun `contextTimeoutFired bumps generation and cancels timeout`() {
        val state = NextWordPersistedState(isShowing = true, currentGeneration = 2)
        val outcome =
            NextWordEngine.decide(
                intent = NextWordIntent.ContextTimeoutFired,
                state = state,
                input = input(nowMs = 1_000L),
            )
        assertEquals(3L, outcome.newState.currentGeneration)
        assertTrue(outcome.effects.any { it is NextWordOutcome.Effect.CancelContextTimeout })
        assertTrue(outcome.effects.any { it is NextWordOutcome.Effect.ClearPredictionsUI })
    }

    @Test
    fun `resetFull bumps generation`() {
        val state = NextWordPersistedState(currentGeneration = 42)
        val outcome =
            NextWordEngine.decide(
                intent = NextWordIntent.ResetFull,
                state = state,
                input = input(nowMs = 0L),
            )
        assertEquals(43L, outcome.newState.currentGeneration)
    }

    // endregion

    // region decide — clearForNewComposing behavior

    @Test
    fun `clearForNewComposing emits ClearPredictionsUI only when isShowing`() {
        val showing = NextWordPersistedState(isShowing = true, currentGeneration = 1)
        val notShowing = NextWordPersistedState(isShowing = false, currentGeneration = 1)

        val showingOutcome =
            NextWordEngine.decide(
                intent = NextWordIntent.ClearForNewComposing,
                state = showing,
                input = input(nowMs = 0L),
            )
        val notShowingOutcome =
            NextWordEngine.decide(
                intent = NextWordIntent.ClearForNewComposing,
                state = notShowing,
                input = input(nowMs = 0L),
            )

        assertTrue(showingOutcome.effects.any { it is NextWordOutcome.Effect.ClearPredictionsUI })
        assertTrue(notShowingOutcome.effects.isEmpty())

        assertEquals(2L, showingOutcome.newState.currentGeneration)
        assertEquals(2L, notShowingOutcome.newState.currentGeneration)
        assertFalse(showingOutcome.newState.isShowing)
    }

    // endregion

    // region decide — wordSelected records when inside window + updates state

    @Test
    fun `wordSelected inside association window records and reschedules timeout`() {
        val state =
            NextWordPersistedState(
                lastSelectedWord = "我",
                lastSelectedRoman = "gua",
                lastSelectionTimeMs = 500,
                currentGeneration = 1,
            )
        val outcome =
            NextWordEngine.decide(
                intent =
                    NextWordIntent.WordSelected(
                        text = "好",
                        roman = "ho",
                        requireRomanMode = false,
                        triggerPrediction = true,
                    ),
                state = state,
                // 5s after last selection — inside 10s window
                input = input(nowMs = 5_500L),
            )

        val assoc = outcome.effects.filterIsInstance<NextWordOutcome.Effect.RecordAssociation>().single()
        assertEquals("我", assoc.pair.prev)
        assertEquals("好", assoc.pair.next)

        assertTrue(
            "valid wordSelected must reschedule context timeout",
            outcome.effects.any { it is NextWordOutcome.Effect.RescheduleContextTimeout },
        )

        val query = outcome.effects.filterIsInstance<NextWordOutcome.Effect.QueryPredictions>().single()
        assertEquals("好", query.word)
        assertEquals(2L, query.generation)
        assertEquals("QueryPredictions must carry the decision-time nowMs", 5_500L, query.nowMs)
        assertEquals(5_500L, outcome.newState.lastSelectionTimeMs)
    }

    @Test
    fun `wordSelected outside association window does not record but still reschedules`() {
        val state =
            NextWordPersistedState(
                lastSelectedWord = "我",
                lastSelectionTimeMs = 0,
            )
        val outcome =
            NextWordEngine.decide(
                intent =
                    NextWordIntent.WordSelected(
                        text = "好",
                        roman = "ho",
                        requireRomanMode = false,
                        triggerPrediction = false,
                    ),
                state = state,
                // 11s later — outside 10s window
                input = input(nowMs = 11_000L),
            )
        assertTrue(
            "outside window must not emit RecordAssociation",
            outcome.effects.none { it is NextWordOutcome.Effect.RecordAssociation },
        )
        assertTrue(outcome.effects.any { it is NextWordOutcome.Effect.RescheduleContextTimeout })
    }

    @Test
    fun `wordSelected requireRomanMode skipped when isTranslateSwapped`() {
        val settings = defaultSettings.copy(isTranslateSwapped = true)
        val outcome =
            NextWordEngine.decide(
                intent =
                    NextWordIntent.WordSelected(
                        text = "我",
                        roman = "gua",
                        requireRomanMode = true,
                        triggerPrediction = true,
                    ),
                state = NextWordPersistedState.initial,
                input = input(nowMs = 1_000L, settings = settings),
            )
        assertTrue(outcome.effects.isEmpty())
        assertEquals(0L, outcome.newState.currentGeneration)
    }

    // endregion

    // region filterPredictions

    @Test
    fun `filterPredictions drops empty-tl rows in roman mode`() {
        val raw =
            listOf(
                RawNextWordPrediction(hanzi = "你", tl = "", score = 1.0),
                RawNextWordPrediction(hanzi = "我", tl = "gua", score = 2.0),
            )
        val result = NextWordEngine.filterPredictions(raw, defaultSettings)
        assertEquals(1, result.size)
        assertEquals("我", result[0].hanzi)
    }

    @Test
    fun `filterPredictions keeps empty-tl rows when isTranslateSwapped`() {
        val raw =
            listOf(
                RawNextWordPrediction(hanzi = "你", tl = "", score = 1.0),
                RawNextWordPrediction(hanzi = "我", tl = "gua", score = 2.0),
            )
        val result =
            NextWordEngine.filterPredictions(
                raw,
                defaultSettings.copy(isTranslateSwapped = true),
            )
        assertEquals(2, result.size)
    }

    @Test
    fun `filterPredictions sets subtitle only when roman non-empty`() {
        val raw =
            listOf(
                RawNextWordPrediction(hanzi = "你", tl = "", score = 1.0),
                RawNextWordPrediction(hanzi = "我", tl = "gua", score = 2.0),
            )
        val result =
            NextWordEngine.filterPredictions(
                raw,
                defaultSettings.copy(isTranslateSwapped = true),
            )
        assertNull("empty roman → null subtitle", result[0].subtitle)
        assertEquals("我", result[1].subtitle)
    }

    // endregion
}
