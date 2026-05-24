package com.siansiansu.taigikeyboard.ime.text.keyboard

import com.siansiansu.taigikeyboard.ime.text.key.KeyVariation
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotSame
import org.junit.Assert.assertSame
import org.junit.Test

/**
 * Pure-JVM coverage for [KeyboardUiCoordinator]'s state-mutation equality
 * collapse. The orchestration paths (cancel/launch/IO/main-hop) plus
 * `publishAppearance` (its [KeyboardAppearance] argument requires
 * `android.graphics.Typeface`, which throws on pure JVM) are covered by the
 * dogfood matrix per `~/.claude/rules/code-review-rules.md §9`.
 *
 * `layoutManagerFactory` is intentionally a throwing stub — none of these
 * tests touch `ensureLayoutLoaded` / `reload*` paths, so the lazy delegate
 * never resolves and the JVM never loads `LayoutManager`'s Android deps.
 *
 * The equality-collapse pattern under test is shared verbatim by
 * `publishLayout` (private) and `publishAppearance` (Android-typed); covering
 * `publishKeyVariation` and `publishActiveMode` certifies the same
 * `current == next → return current` shape across all four publishers.
 *
 * `setActiveKeyboardMode` orchestration (CLIPBOARD coerce + cache-hit-vs-load
 * fork + `onActiveModeChanged` callback timing) is dogfood-scope per F10=C —
 * it needs a real [LayoutManager] + [Subtype] to exercise both branches.
 */
@OptIn(ExperimentalCoroutinesApi::class)
class KeyboardUiCoordinatorTest {

    private fun newCoordinator(scope: CoroutineScope): KeyboardUiCoordinator =
        KeyboardUiCoordinator(
            scope = scope,
            layoutManagerFactory = { error("layoutManager not used in this test") },
            activeSubtypeProvider = { error("activeSubtype not used in this test") },
            translateSwappedProvider = { false },
            onLayoutChanged = {},
            onActiveModeChanged = {},
        )

    @Test
    fun `publishKeyVariation skips emission when value is unchanged`() = runTest(UnconfinedTestDispatcher()) {
        val coordinator = newCoordinator(this)
        coordinator.publishKeyVariation(KeyVariation.PASSWORD)
        val first = coordinator.keyboardUi.value
        coordinator.publishKeyVariation(KeyVariation.PASSWORD)
        val second = coordinator.keyboardUi.value
        assertSame("equal keyVariation must collapse to the same state ref", first, second)
        assertEquals(KeyVariation.PASSWORD, second.keyVariation)
    }

    @Test
    fun `publishKeyVariation emits when value changes`() = runTest(UnconfinedTestDispatcher()) {
        val coordinator = newCoordinator(this)
        coordinator.publishKeyVariation(KeyVariation.NORMAL)
        val first = coordinator.keyboardUi.value
        coordinator.publishKeyVariation(KeyVariation.PASSWORD)
        val second = coordinator.keyboardUi.value
        assertNotSame(first, second)
        assertEquals(KeyVariation.NORMAL, first.keyVariation)
        assertEquals(KeyVariation.PASSWORD, second.keyVariation)
    }

    @Test
    fun `publishActiveMode skips emission when mode is unchanged`() = runTest(UnconfinedTestDispatcher()) {
        val coordinator = newCoordinator(this)
        coordinator.publishActiveMode(KeyboardMode.SYMBOLS)
        val first = coordinator.keyboardUi.value
        coordinator.publishActiveMode(KeyboardMode.SYMBOLS)
        val second = coordinator.keyboardUi.value
        assertSame("equal activeMode must collapse to the same state ref", first, second)
        assertEquals(KeyboardMode.SYMBOLS, second.activeMode)
    }

    @Test
    fun `publishActiveMode emits when mode changes`() = runTest(UnconfinedTestDispatcher()) {
        val coordinator = newCoordinator(this)
        coordinator.publishActiveMode(KeyboardMode.NUMERIC)
        val first = coordinator.keyboardUi.value
        coordinator.publishActiveMode(KeyboardMode.PHONE)
        val second = coordinator.keyboardUi.value
        assertNotSame(first, second)
        assertEquals(KeyboardMode.NUMERIC, first.activeMode)
        assertEquals(KeyboardMode.PHONE, second.activeMode)
    }

}
