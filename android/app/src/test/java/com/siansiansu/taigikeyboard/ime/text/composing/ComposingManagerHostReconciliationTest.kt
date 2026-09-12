// Pins host-callback reconciliation: a "no composing region" report is honoured only when the host
// really no longer holds the preedit the IME last wrote (USER report 2026-09-12, first key lost).

package com.siansiansu.taigikeyboard.ime.text.composing

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
     * `InputConnection` stand-in: answers `getTextBeforeCursor` with [textBeforeCursor] (`null` for
     * a dead connection), records every write by name, and throws on a read when [readsForbidden]
     * so an idle reconcile is proven not to round-trip.
     */
    private class FakeHost(
        private val textBeforeCursor: CharSequence?,
        private val readsForbidden: Boolean = false,
    ) {
        val writes = mutableListOf<String>()

        val ic: InputConnection =
            Proxy.newProxyInstance(
                InputConnection::class.java.classLoader,
                arrayOf(InputConnection::class.java),
            ) { _, method, _ ->
                when (method.name) {
                    "getTextBeforeCursor" -> {
                        check(!readsForbidden) { "host read while idle" }
                        textBeforeCursor
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
    fun reconcile_hostStillHoldsPreedit_keepsStateAndGeneration() {
        // trace: first key wrote "a"; a stale focus-move report (-1/-1) arrives; the host
        // re-read (ordered after the write) returns "a" → keep, no generation bump.
        val manager = composing("a")
        val tokenBefore = manager.stateToken()

        manager.reconcileWithHost(FakeHost("a").ic)

        assertTrue(manager.isComposing())
        assertEquals("a", manager.getRawInput())
        assertEquals(tokenBefore, manager.stateToken())
    }

    @Test
    fun reconcile_unicodePreedit_keeps() {
        // trace: "o͘" = 'o' + U+0358 (two UTF-16 units); read length follows the display text.
        val manager = composing("o͘")
        manager.reconcileWithHost(FakeHost("o͘").ic)
        assertTrue(manager.isComposing())
    }

    @Test
    fun reconcile_hostLostPreedit_clearsOnceWithoutWritingHost() {
        // trace: real tap-away — cursor moved, the text before it is no longer the preedit.
        val manager = composing("a")
        val generationBefore = manager.stateToken().generation
        val host = FakeHost("z")

        manager.reconcileWithHost(host.ic)

        assertFalse(manager.isComposing())
        assertNull(manager.getRawInput())
        assertEquals(generationBefore + 1, manager.stateToken().generation)
        assertTrue("INVARIANT_composing_external_region_clear_discards_state: no IC write", host.writes.isEmpty())
        // A second report for the same clear finds an idle manager → no read, no further bump.
        manager.reconcileWithHost(FakeHost("z", readsForbidden = true).ic)
        assertEquals(generationBefore + 1, manager.stateToken().generation)
    }

    @Test
    fun reconcile_shortRead_clears() {
        val manager = composing("tai")
        manager.reconcileWithHost(FakeHost("ai").ic)
        assertFalse(manager.isComposing())
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
        // for B, so the read must compare against B's preedit.
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
        manager.applyTransition(preedit("b"), FakeHost("b").ic)

        manager.reconcileWithHost(FakeHost("b").ic)

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
