// Pins host-callback reconciliation: a "no composing region" report is honoured only when the host
// really no longer holds the preedit the IME last wrote (USER report 2026-09-12, first key lost).

package com.siansiansu.taigikeyboard.ime.text.composing

import android.view.inputmethod.ExtractedText
import android.view.inputmethod.InputConnection
import com.siansiansu.taigikeyboard.engine.RustEngineBridge.ComposingTransition
import com.siansiansu.taigikeyboard.engine.RustEngineBridge.ComposingTransition.Effect
import java.lang.reflect.Proxy
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ComposingManagerHostReconciliationTest {
    /**
     * `InputConnection` stand-in over a document snapshot: `getExtractedText` returns [text] with a
     * cursor at [cursor] (`null` text = dead connection; [selectionStart] for a non-collapsed
     * selection; [startOffset] for a partial extraction). Records writes + composing regions, and
     * throws on a read when [readsForbidden] so an idle reconcile is proven not to round-trip.
     */
    private class FakeHost(
        private val text: String?,
        private val cursor: Int = text?.length ?: 0,
        private val selectionStart: Int = cursor,
        private val startOffset: Int = 0,
        private val readsForbidden: Boolean = false,
    ) {
        val writes = mutableListOf<String>()
        val regions = mutableListOf<Pair<Int, Int>>()

        val ic: InputConnection =
            Proxy.newProxyInstance(
                InputConnection::class.java.classLoader,
                arrayOf(InputConnection::class.java),
            ) { _, method, args ->
                when (method.name) {
                    "getExtractedText" -> {
                        check(!readsForbidden) { "host read while idle" }
                        text?.let {
                            ExtractedText().apply {
                                this.text = it
                                this.startOffset = this@FakeHost.startOffset
                                this.selectionStart = this@FakeHost.selectionStart
                                this.selectionEnd = cursor
                            }
                        }
                    }
                    "setComposingRegion" -> {
                        regions += (args!![0] as Int) to (args[1] as Int)
                        true
                    }
                    "setComposingText", "finishComposingText", "commitText", "deleteSurroundingText" -> {
                        writes += method.name
                        true
                    }
                    else -> null
                }
            } as InputConnection
    }

    private fun preedit(display: String) =
        ComposingTransition(
            rawInput = display,
            displayText = display,
            effects = listOf(Effect.UpdatePreedit(display)),
            selectedCandidateIndex = -1,
            isComposing = true,
        )

    /** Puts a fresh manager into composing state with [display] written to a matching host. */
    private fun composing(display: String, delegate: ComposingDelegate = DefaultComposingDelegate) =
        composingManagerForTest(delegate).also { it.applyTransition(preedit(display), FakeHost(display).ic) }

    @Test
    fun reconcile_whenIdle_keepsWithoutReadingHost() {
        val manager = composingManagerForTest()
        manager.reconcileWithHost(FakeHost("x", readsForbidden = true).ic)
        assertFalse(manager.isComposing())
    }

    @Test
    fun reconcile_duringSelfCommit_skipsWithoutReadingHost() {
        val manager = composing("a")
        manager.selfCommitInProgress = true
        manager.reconcileWithHost(FakeHost("z", readsForbidden = true).ic)
        assertTrue(manager.isComposing())
    }

    @Test
    fun reconcile_hostStillHoldsPreedit_keepsStateAndGeneration_andReassertsSpan() {
        // trace: doc "abcd", first key wrote "tai" → "abcdtai|"; a stale focus-move report (-1/-1,
        // its own selEnd=4) arrives; the snapshot (ordered after the write) shows "tai" before
        // cursor 7 → keep, no generation bump, span re-asserted at [4,7) — never [1,4) from the
        // report's stale coordinate.
        val manager = composing("tai")
        val tokenBefore = manager.stateToken()
        val host = FakeHost("abcdtai")

        manager.reconcileWithHost(host.ic)

        assertTrue(manager.isComposing())
        assertEquals("tai", manager.getRawInput())
        assertEquals(tokenBefore, manager.stateToken())
        assertEquals(listOf(4 to 7), host.regions)
        assertTrue(host.writes.isEmpty())
    }

    @Test
    fun reconcile_partialExtraction_usesStartOffset() {
        // trace: host extracted only "xtai" starting at document offset 100, cursor at local 4.
        val manager = composing("tai")
        val host = FakeHost("xtai", startOffset = 100)
        manager.reconcileWithHost(host.ic)
        assertEquals(listOf(101 to 104), host.regions)
    }

    @Test
    fun reconcile_unicodePreedit_keeps() {
        // trace: "o͘" = 'o' + U+0358 (two UTF-16 units); region length follows the display text.
        val manager = composing("o͘")
        val host = FakeHost("o͘")
        manager.reconcileWithHost(host.ic)
        assertTrue(manager.isComposing())
        assertEquals(listOf(0 to 2), host.regions)
    }

    @Test
    fun reconcile_hostLostPreedit_clearsOnceWithoutWritingHost() {
        // trace: real tap-away — cursor moved, the text before it is no longer the preedit.
        val manager = composing("a")
        val generationBefore = manager.stateToken().generation
        val host = FakeHost("az", cursor = 2)

        manager.reconcileWithHost(host.ic)

        assertFalse(manager.isComposing())
        assertNull(manager.getRawInput())
        assertEquals(generationBefore + 1, manager.stateToken().generation)
        assertTrue("INVARIANT_composing_external_region_clear_discards_state: no IC write", host.writes.isEmpty())
        assertTrue(host.regions.isEmpty())
        // A second report for the same clear finds an idle manager → no read, no further bump.
        manager.reconcileWithHost(FakeHost("az", readsForbidden = true).ic)
        assertEquals(generationBefore + 1, manager.stateToken().generation)
    }

    @Test
    fun reconcile_cursorMovedInsidePreedit_clears() {
        // trace: "tai" written, user tapped between "t" and "a" → text before cursor is "t".
        val manager = composing("tai")
        manager.reconcileWithHost(FakeHost("tai", cursor = 1).ic)
        assertFalse(manager.isComposing())
    }

    @Test
    fun reconcile_nonCollapsedSelection_clears() {
        val manager = composing("tai")
        val host = FakeHost("tai", cursor = 3, selectionStart = 0)
        manager.reconcileWithHost(host.ic)
        assertFalse(manager.isComposing())
        assertTrue(host.regions.isEmpty())
    }

    @Test
    fun reconcile_nullRead_clears() {
        val manager = composing("a")
        manager.reconcileWithHost(FakeHost(null).ic)
        assertFalse(manager.isComposing())
    }

    @Test
    fun reconcile_nullConnection_clears() {
        val manager = composing("a")
        manager.reconcileWithHost(null)
        assertFalse(manager.isComposing())
    }

    @Test
    fun onHostSelectionUpdate_onlyMinusOneMinusOneReconciles() {
        val manager = composing("a")
        // Acknowledged composing range → no read at all.
        manager.onHostSelectionUpdate(candidatesStart = 0, candidatesEnd = 1, ic = FakeHost("z", readsForbidden = true).ic)
        assertTrue(manager.isComposing())
        manager.onHostSelectionUpdate(candidatesStart = -1, candidatesEnd = -1, ic = FakeHost("z").ic)
        assertFalse(manager.isComposing())
    }

    @Test
    fun reconcile_commitAThenStartB_lateCallbackForA_keepsB() {
        // trace: commit "A" → start "b" → A's late -1/-1 report. isComposing is true again
        // for B, so the snapshot must be compared against B's preedit.
        val manager = composing("a")
        manager.applyTransition(
            ComposingTransition(
                rawInput = "",
                displayText = "",
                effects = listOf(Effect.CommitTextReplacingPreedit("A")),
                selectedCandidateIndex = -1,
                isComposing = false,
            ),
            FakeHost("a").ic,
        )
        assertFalse(manager.isComposing())
        manager.applyTransition(preedit("b"), FakeHost("Ab").ic)

        manager.reconcileWithHost(FakeHost("Ab").ic)

        assertTrue(manager.isComposing())
        assertEquals("b", manager.getRawInput())
    }

    @Test
    fun reconcile_reentrantFromInsideWrite_seesNewPreedit() {
        // trace: a host that fires onUpdateSelection synchronously from inside setComposingText;
        // the mirror is assigned before the effects run, so the re-entrant reconcile keeps.
        var reentrantKept: Boolean? = null
        lateinit var manager: ComposingManager
        val delegate =
            ComposingDelegate { effect, ic ->
                if (effect is Effect.UpdatePreedit) {
                    manager.reconcileWithHost(FakeHost(effect.display).ic)
                    reentrantKept = manager.isComposing()
                }
                DefaultComposingDelegate.execute(effect, ic)
            }
        manager = composingManagerForTest(delegate)

        manager.applyTransition(preedit("ta"), FakeHost("ta").ic)

        assertEquals(true, reentrantKept)
        assertTrue(manager.isComposing())
    }

    @Test
    fun bumpGeneration_zeroesMirror_soReconcileCannotKeepDroppedComposition() {
        // trace: input-mode switch with no InputConnection bumps the generation (engine will
        // drop its state) — the mirror must not survive to be "kept" by a later same-text reconcile.
        val manager = composing("a")
        manager.bumpGeneration()
        assertFalse(manager.isComposing())
        manager.reconcileWithHost(FakeHost("a", readsForbidden = true).ic)
        assertFalse(manager.isComposing())
    }
}
