// Pins the publish guard for worker-side candidate fetches: a result is dropped when the
// manager instance or its state token (raw buffer + input-context generation) moved underneath it.

package com.siansiansu.taigikeyboard.ime.text

import com.siansiansu.taigikeyboard.ime.text.composing.composingManagerForTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class FetchContextTest {
    private fun manager() = composingManagerForTest()

    @Test
    fun stateToken_isStableWhileNothingChanges() {
        val manager = manager()
        // trace: not composing → rawInput null; generation is the process-wide counter.
        assertEquals(manager.stateToken(), manager.stateToken())
    }

    @Test
    fun stateToken_changesAfterGenerationBump() {
        val manager = manager()
        val before = manager.stateToken()
        manager.bumpGeneration()
        assertNotEquals(before, manager.stateToken())
    }

    @Test
    fun isCurrent_sameManagerSameToken_isTrue() {
        val manager = manager()
        assertTrue(FetchContext(manager, manager.stateToken()).isCurrent(manager))
    }

    @Test
    fun isCurrent_afterGenerationBump_isFalse() {
        val manager = manager()
        val context = FetchContext(manager, manager.stateToken())
        // Field switch while the fetch was in flight: same buffer text, new input context.
        manager.bumpGeneration()
        assertFalse(context.isCurrent(manager))
    }

    @Test
    fun isCurrent_differentManagerInstance_isFalse() {
        val manager = manager()
        val context = FetchContext(manager, manager.stateToken())
        // Keyboard-mode swap replaced the manager between capture and publish.
        assertFalse(context.isCurrent(manager()))
    }

    @Test
    fun isCurrent_managerGone_isFalse() {
        val manager = manager()
        assertFalse(FetchContext(manager, manager.stateToken()).isCurrent(null))
    }
}
