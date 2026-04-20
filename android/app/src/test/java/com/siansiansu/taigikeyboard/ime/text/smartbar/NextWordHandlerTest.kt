package com.siansiansu.taigikeyboard.ime.text.smartbar

import com.siansiansu.taigikeyboard.ime.core.logging.NullLoggerBackend
import com.siansiansu.taigikeyboard.ime.core.nextword.NextWordEngine
import com.siansiansu.taigikeyboard.ime.core.nextword.NextWordPredictor
import com.siansiansu.taigikeyboard.ime.core.nextword.RawNextWordPrediction
import com.siansiansu.taigikeyboard.ime.core.settings.EngineSettings
import com.siansiansu.taigikeyboard.ime.core.settings.EngineSettingsProvider
import com.siansiansu.taigikeyboard.ime.core.settings.ToneToggles
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import java.util.concurrent.atomic.AtomicInteger

/**
 * Platform-executor tests for [NextWordHandler] that require
 * `kotlinx-coroutines-test`. Labels match `nextword-engine-boundary.md`
 * §10 — A5-impl deferred both to A9 pending the dep + an interface seam.
 * A9 adds `NextWordPredictor` (seam) and the `kotlinx-coroutines-test`
 * dep, unblocking both labels.
 */
@OptIn(ExperimentalCoroutinesApi::class)
class NextWordHandlerTest {
    // `NextWordHandler.dispatchPredictionQuery` resolves the query result
    // with `withContext(Dispatchers.Main)`. Pure JVM tests do not have a
    // Main dispatcher wired up, so install the shared test scheduler as
    // the Main dispatcher for the test's lifetime.
    private val mainDispatcher = StandardTestDispatcher()

    @Before
    fun installMainDispatcher() {
        Dispatchers.setMain(mainDispatcher)
    }

    @After
    fun resetMainDispatcher() {
        Dispatchers.resetMain()
    }

    /**
     * When a prediction query races against a generation bump
     * (contextTimeoutFired / resetContext / backspace-to-empty), the
     * late-arriving result MUST NOT call `onUpdateCandidates`. Pin on
     * the state-generation drop path inside [NextWordHandler.handleQueryResult]
     * introduced by A5-impl.
     */
    @Test
    fun test_INVARIANT_nextword_late_prediction_is_discarded() =
        runTest(mainDispatcher) {
            val testScope = TestScope(mainDispatcher)
            val updateCount = AtomicInteger(0)
            val clearCount = AtomicInteger(0)
            val gate = CompletableDeferred<List<RawNextWordPrediction>>()
            val predictor = GatedPredictor(gate)

            val handler =
                NextWordHandler(
                    scope = testScope,
                    settingsProvider = FakeSettingsProvider(),
                    nextWord = predictor,
                    logger = NullLoggerBackend,
                    onUpdateCandidates = { updateCount.incrementAndGet() },
                    onClearCandidates = { clearCount.incrementAndGet() },
                )

            // Trigger a word selection → QueryPredictions (gen = N).
            handler.handleNextWordPrediction(
                displayText = "某",
                committedText = "某",
                roman = "bou2",
            )
            // Run the coroutine scope up to the `predictor.predict(...)` suspension
            // (predictor holds the result inside its gate).
            advanceUntilIdle()

            // Bump the generation counter — state.currentGeneration increments.
            handler.resetContext()

            // Unblock the predictor — result lands at the OLD generation.
            gate.complete(
                listOf(RawNextWordPrediction(hanzi = "某", tl = "bou2", score = 1.0)),
            )
            advanceUntilIdle()

            assertEquals(
                "late prediction at stale generation must NOT call onUpdateCandidates",
                0,
                updateCount.get(),
            )
        }

    /**
     * Rapid reschedule intents must not leak timer jobs. N sequential
     * reschedules leave exactly ONE active timeout — each new schedule
     * cancels the previous one before launching its replacement.
     * Observed via: after advancing time past the timeout once, the
     * timeout fires at most once (no duplicate state transitions).
     */
    @Test
    fun test_INVARIANT_nextword_rescheduling_leaks_no_timer() =
        runTest(mainDispatcher) {
            val testScope = TestScope(mainDispatcher)
            val clearCount = AtomicInteger(0)
            val handler =
                NextWordHandler(
                    scope = testScope,
                    settingsProvider = FakeSettingsProvider(),
                    nextWord = NoopPredictor(),
                    logger = NullLoggerBackend,
                    onUpdateCandidates = { /* no-op */ },
                    onClearCandidates = { clearCount.incrementAndGet() },
                )

            // Prime `isShowing` so the eventual context-timeout reset
            // actually calls `onClearCandidates` (engine gates the UI
            // clear on isShowing being true — see `NextWordEngine.decide`
            // for the ContextTimeoutFired case).
            handler.setShowingNextWord(true)

            // Issue 100 rapid reschedules at t=0.
            repeat(100) {
                handler.handleNextWordPrediction(
                    displayText = "某",
                    committedText = "a", // non-sentence-end to avoid ResetFull path
                    roman = "bou2",
                )
            }
            advanceUntilIdle()

            // Advance past the timeout window exactly once.
            advanceTimeBy(NextWordEngine.CONTEXT_TIMEOUT_MS + 10)
            advanceUntilIdle()

            assertTrue(
                "rescheduling must collapse to one live timer; fired count = ${clearCount.get()}",
                clearCount.get() <= 1,
            )
            assertFalse("state must have dropped isShowing", handler.isShowingNextWordCandidates())
        }

    // --- Fakes ---

    /**
     * Fake [NextWordPredictor] whose `predict` suspends on a caller-owned
     * `CompletableDeferred` — lets the test drive the race between
     * `dispatchPredictionQuery` and `resetContext`.
     */
    private class GatedPredictor(
        private val gate: CompletableDeferred<List<RawNextWordPrediction>>,
    ) : NextWordPredictor {
        override suspend fun predict(
            word: String,
            roman: String,
            limit: Int,
            settings: EngineSettings,
            nowMs: Long,
        ): List<RawNextWordPrediction> = gate.await()

        override suspend fun recordAssociation(
            prev: String,
            prevTl: String,
            nextHanzi: String,
            nextTl: String,
        ) = Unit
    }

    /** Predictor that never blocks and returns empty on every call. */
    private class NoopPredictor : NextWordPredictor {
        override suspend fun predict(
            word: String,
            roman: String,
            limit: Int,
            settings: EngineSettings,
            nowMs: Long,
        ): List<RawNextWordPrediction> = emptyList()

        override suspend fun recordAssociation(
            prev: String,
            prevTl: String,
            nextHanzi: String,
            nextTl: String,
        ) = Unit
    }

    private class FakeSettingsProvider : EngineSettingsProvider {
        override val current: EngineSettings = FakeEngineSettings()
    }

    private class FakeEngineSettings : EngineSettings {
        override val inputMode: String = "tl"
        override val isAutoCap: Boolean = false
        override val isTranslateSwapped: Boolean = false
        override val isAssociationRecordingEnabled: Boolean = true
        override val toneToggles: ToneToggles =
            ToneToggles(isDoubleTapOOEnabled = false, isDoubleTapNNEnabled = false)
        override val isCustomDictEnabled: Boolean = false
        override val isTpsOrMappedToER: Boolean = false
        override val isMoeDictEnabled: Boolean = true
        override val isNewwordDictEnabled: Boolean = false
        override val isKunggeDictEnabled: Boolean = false
        override val isITaigiDictEnabled: Boolean = false
        override val isTaiwanJapanDictEnabled: Boolean = false
        override val isTaiHuaDictEnabled: Boolean = false
        override val isTaiwanPlantDictEnabled: Boolean = false
        override val isSttiDictEnabled: Boolean = false
        override val isKhpooDictEnabled: Boolean = false
        override val isVariantEnabled: Boolean = false
        override val isKhiinEnabled: Boolean = false
        override val isLkkDictEnabled: Boolean = false
    }
}

// Silence unused-symbol warnings on `TaigiWord` — it's imported for future
// assertion growth (e.g. pinning the exact candidate list emitted).
@Suppress("unused")
private typealias _UnusedTaigiWord = TaigiWord
