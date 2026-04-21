package com.siansiansu.taigikeyboard.ime.text.composing

import com.siansiansu.taigikeyboard.ime.core.settings.ToneToggles
import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Pure-state tests for [ComposingState] — no [android.view.inputmethod.InputConnection],
 * no settings providers, no wrapper. Named per `composing-state-boundary.md`
 * §8 so `behavioral-invariants.md` §13 can reference them directly when
 * A9 wires `INVARIANT_*` labels.
 *
 * Mirrors iOS `ComposingStateTests.swift`. Where Android diverges from iOS
 * (live-typing preedit carries raw keystrokes vs iOS's tone-marked derived
 * form — see `composing-state-boundary.md` §11 Android addendum), the test
 * asserts Android's raw-text contract explicitly rather than mirroring iOS.
 */
class ComposingStateTest {
    private val tl = ToneConverterModels.InputMode.TL
    private val togglesOff = ToneToggles(isDoubleTapOOEnabled = false, isDoubleTapNNEnabled = false)

    // MARK: - Initial state

    @Test
    fun `initial state is idle with selectedIndex -1`() {
        val state = ComposingState()
        assertFalse(state.isComposing)
        assertEquals("", state.rawInput)
        assertEquals(-1, state.selectedCandidateIndex)
    }

    // MARK: - start / append effect ordering

    @Test
    fun `start emits UpdatePreedit(raw) then PerformAutocomplete`() {
        val (next, transition) =
            ComposingState().apply(ComposingState.Intent.Start("a"), tl, togglesOff)

        assertTrue(next.isComposing)
        assertEquals("a", next.rawInput)
        assertEquals(0, next.selectedCandidateIndex)
        assertEquals(0, transition.newSelectedIndex)
        assertEquals(
            listOf(
                ComposingTransition.Effect.UpdatePreedit("a"),
                ComposingTransition.Effect.PerformAutocomplete,
            ),
            transition.effects,
        )
    }

    @Test
    fun `append when idle behaves as start`() {
        val (_, transition) =
            ComposingState().apply(ComposingState.Intent.Append("a"), tl, togglesOff)
        assertEquals(
            listOf(
                ComposingTransition.Effect.UpdatePreedit("a"),
                ComposingTransition.Effect.PerformAutocomplete,
            ),
            transition.effects,
        )
        assertEquals(0, transition.newSelectedIndex)
    }

    @Test
    fun `append when composing appends to raw and resets selectedIndex`() {
        val (afterStart, _) =
            ComposingState().apply(ComposingState.Intent.Start("a"), tl, togglesOff)
        val (afterAppend, transition) = afterStart.apply(ComposingState.Intent.Append("b"), tl, togglesOff)

        assertEquals("ab", afterAppend.rawInput)
        assertEquals(0, transition.newSelectedIndex)
        assertEquals(
            listOf(
                ComposingTransition.Effect.UpdatePreedit("ab"),
                ComposingTransition.Effect.PerformAutocomplete,
            ),
            transition.effects,
        )
    }

    @Test
    fun `appendHyphen appends a literal hyphen`() {
        val (afterStart, _) =
            ComposingState().apply(ComposingState.Intent.Start("a"), tl, togglesOff)
        val (afterHyphen, transition) = afterStart.apply(ComposingState.Intent.AppendHyphen, tl, togglesOff)

        assertEquals("a-", afterHyphen.rawInput)
        assertEquals(
            listOf(
                ComposingTransition.Effect.UpdatePreedit("a-"),
                ComposingTransition.Effect.PerformAutocomplete,
            ),
            transition.effects,
        )
    }

    // MARK: - replaceLast preserves selectedCandidateIndex

    @Test
    fun `replaceLast preserves externally-set selectedCandidateIndex`() {
        val (afterStart, _) =
            ComposingState().apply(ComposingState.Intent.Start("ab"), tl, togglesOff)
        val withSelection = afterStart.withSelectedIndex(3)

        val (afterReplace, transition) =
            withSelection.apply(ComposingState.Intent.ReplaceLast("c"), tl, togglesOff)

        assertEquals("ac", afterReplace.rawInput)
        assertEquals(3, afterReplace.selectedCandidateIndex)
        assertEquals(3, transition.newSelectedIndex)
    }

    @Test
    fun `append resets selectedIndex back to 0`() {
        val (afterStart, _) =
            ComposingState().apply(ComposingState.Intent.Start("a"), tl, togglesOff)
        val withSelection = afterStart.withSelectedIndex(3)

        val (afterAppend, transition) =
            withSelection.apply(ComposingState.Intent.Append("b"), tl, togglesOff)

        assertEquals(0, afterAppend.selectedCandidateIndex)
        assertEquals(0, transition.newSelectedIndex)
    }

    @Test
    fun `replaceLast when idle is noop`() {
        val (next, transition) =
            ComposingState().apply(ComposingState.Intent.ReplaceLast("x"), tl, togglesOff)

        assertFalse(next.isComposing)
        assertTrue(transition.effects.isEmpty())
    }

    // MARK: - deleteBackward effect ordering

    @Test
    fun `deleteBackward at raw length 1 emits clearPreedit then reset then deleteFromDoc`() {
        val (afterStart, _) =
            ComposingState().apply(ComposingState.Intent.Start("a"), tl, togglesOff)
        val (next, transition) = afterStart.apply(ComposingState.Intent.DeleteBackward, tl, togglesOff)

        assertFalse(next.isComposing)
        assertEquals(-1, next.selectedCandidateIndex)
        assertEquals(-1, transition.newSelectedIndex)
        assertEquals(
            listOf(
                ComposingTransition.Effect.ClearPreeditWithoutCommit,
                ComposingTransition.Effect.ResetAutocomplete,
                ComposingTransition.Effect.DeleteBackwardFromDocument,
            ),
            transition.effects,
        )
    }

    @Test
    fun `deleteBackward at raw length greater than 1 shortens and keeps composing`() {
        val (afterStart, _) =
            ComposingState().apply(ComposingState.Intent.Start("ab"), tl, togglesOff)
        val (next, transition) = afterStart.apply(ComposingState.Intent.DeleteBackward, tl, togglesOff)

        assertTrue(next.isComposing)
        assertEquals("a", next.rawInput)
        assertEquals(
            listOf(
                ComposingTransition.Effect.UpdatePreedit("a"),
                ComposingTransition.Effect.PerformAutocomplete,
            ),
            transition.effects,
        )
    }

    @Test
    fun `deleteBackward when idle is noop`() {
        val (next, transition) =
            ComposingState().apply(ComposingState.Intent.DeleteBackward, tl, togglesOff)
        assertFalse(next.isComposing)
        assertTrue(transition.effects.isEmpty())
    }

    // MARK: - Commit paths

    @Test
    fun `commitDerived emits CommitTextReplacingPreedit with derived form and resets contexts`() {
        val (afterStart, _) =
            ComposingState().apply(ComposingState.Intent.Start("a"), tl, togglesOff)
        val (next, transition) = afterStart.apply(ComposingState.Intent.CommitDerived, tl, togglesOff)

        assertFalse(next.isComposing)
        assertEquals(-1, transition.newSelectedIndex)
        assertEquals(3, transition.effects.size)
        val first = transition.effects[0]
        assertTrue(first is ComposingTransition.Effect.CommitTextReplacingPreedit)
        assertEquals(
            listOf(
                ComposingTransition.Effect.ResetAutocomplete,
                ComposingTransition.Effect.ResetAutocompleteContext,
            ),
            transition.effects.drop(1),
        )
    }

    @Test
    fun `commitRaw emits CommitTextReplacingPreedit with literal rawInput`() {
        val (afterStart, _) =
            ComposingState().apply(ComposingState.Intent.Start("gua2"), tl, togglesOff)
        val (next, transition) = afterStart.apply(ComposingState.Intent.CommitRaw, tl, togglesOff)

        assertFalse(next.isComposing)
        assertEquals(
            listOf(
                ComposingTransition.Effect.CommitTextReplacingPreedit("gua2"),
                ComposingTransition.Effect.ResetAutocomplete,
                ComposingTransition.Effect.ResetAutocompleteContext,
            ),
            transition.effects,
        )
    }

    @Test
    fun `selectSuggestion atomically commits suggestion and resets contexts`() {
        val (afterStart, _) =
            ComposingState().apply(ComposingState.Intent.Start("a"), tl, togglesOff)
        val (next, transition) =
            afterStart.apply(ComposingState.Intent.SelectSuggestion("chosen"), tl, togglesOff)

        assertFalse(next.isComposing)
        assertEquals(-1, transition.newSelectedIndex)
        assertEquals(
            listOf(
                ComposingTransition.Effect.CommitTextReplacingPreedit("chosen"),
                ComposingTransition.Effect.ResetAutocomplete,
                ComposingTransition.Effect.ResetAutocompleteContext,
            ),
            transition.effects,
        )
    }

    // MARK: - INVARIANT_composing_external_insert_commits_preedit_atomically

    @Test
    fun `commitPreeditThenInsertExternal when composing emits atomic combined commit`() {
        // Emoji / clipboard tap during active Taigi preedit: the derived
        // preedit must be committed together with the external text in one
        // `commitText` call (atomic from the editor's perspective) —
        // NOT via a bare `finishComposingText` + `commitText(emoji)` pair
        // which is the pre-A5 bypass flagged in
        // `composing-state-boundary.md` §11.6.
        val (afterStart, _) =
            ComposingState().apply(ComposingState.Intent.Start("a"), tl, togglesOff)
        val (next, transition) =
            afterStart.apply(
                ComposingState.Intent.CommitPreeditThenInsertExternal("😀"),
                tl,
                togglesOff,
            )

        assertFalse(next.isComposing)
        assertEquals(-1, next.selectedCandidateIndex)
        assertEquals(-1, transition.newSelectedIndex)
        val commit =
            transition.effects.filterIsInstance<ComposingTransition.Effect.CommitTextReplacingPreedit>().single()
        assertTrue(
            "commit text must end with emoji and include derived preedit",
            commit.text.endsWith("😀") && commit.text.length > "😀".length,
        )
        assertEquals(
            listOf(
                ComposingTransition.Effect.CommitTextReplacingPreedit(commit.text),
                ComposingTransition.Effect.ResetAutocomplete,
                ComposingTransition.Effect.ResetAutocompleteContext,
            ),
            transition.effects,
        )
        assertFalse(
            "must NOT emit a separate ClearPreeditWithoutCommit",
            transition.effects.any { it is ComposingTransition.Effect.ClearPreeditWithoutCommit },
        )
    }

    @Test
    fun `commitPreeditThenInsertExternal when idle emits plain insert only`() {
        val (next, transition) =
            ComposingState().apply(
                ComposingState.Intent.CommitPreeditThenInsertExternal("😀"),
                tl,
                togglesOff,
            )

        assertFalse(next.isComposing)
        assertEquals(
            listOf(ComposingTransition.Effect.CommitTextReplacingPreedit("😀")),
            transition.effects,
        )
        assertEquals(-1, transition.newSelectedIndex)
    }

    @Test
    fun `commitPreeditThenInsertExternal with empty text is noop and preserves preedit`() {
        // Guard against a stray empty-string delegate call discarding the
        // user's in-progress composition.
        val (afterStart, _) =
            ComposingState().apply(ComposingState.Intent.Start("abc"), tl, togglesOff)
        val (next, transition) =
            afterStart.apply(
                ComposingState.Intent.CommitPreeditThenInsertExternal(""),
                tl,
                togglesOff,
            )

        assertTrue(next.isComposing)
        assertEquals("abc", next.rawInput)
        assertTrue(transition.effects.isEmpty())
    }

    @Test
    fun `commitPreeditThenInsertExternal with non-zero selected index commits derived not candidate`() {
        // Pin the contract that emoji insertion does NOT implicitly choose
        // the currently-highlighted candidate. A user may have tapped into
        // candidate index 3; tapping an emoji still commits the derived
        // preedit + emoji atomically, and the candidate selection is
        // discarded as state transitions to idle.
        val (afterStart, _) =
            ComposingState().apply(ComposingState.Intent.Start("a"), tl, togglesOff)
        val composingWithIndex = afterStart.withSelectedIndex(3)

        val (next, transition) =
            composingWithIndex.apply(
                ComposingState.Intent.CommitPreeditThenInsertExternal("😀"),
                tl,
                togglesOff,
            )

        assertFalse(next.isComposing)
        assertEquals(-1, next.selectedCandidateIndex)
        assertEquals(-1, transition.newSelectedIndex)
        val commit =
            transition.effects.filterIsInstance<ComposingTransition.Effect.CommitTextReplacingPreedit>().single()
        assertTrue(
            "commit must combine derived preedit + emoji, not a candidate string",
            commit.text.endsWith("😀") && commit.text.length > "😀".length,
        )
    }

    // MARK: - Reset semantics

    @Test
    fun `reset when composing emits ClearPreeditWithoutCommit and ResetAutocomplete`() {
        val (afterStart, _) =
            ComposingState().apply(ComposingState.Intent.Start("a"), tl, togglesOff)
        val (next, transition) = afterStart.apply(ComposingState.Intent.Reset, tl, togglesOff)

        assertFalse(next.isComposing)
        assertEquals(-1, next.selectedCandidateIndex)
        assertEquals(-1, transition.newSelectedIndex)
        assertEquals(
            listOf(
                ComposingTransition.Effect.ClearPreeditWithoutCommit,
                ComposingTransition.Effect.ResetAutocomplete,
            ),
            transition.effects,
        )
        assertFalse(
            "reset must NOT commit the preedit",
            transition.effects.any { it is ComposingTransition.Effect.CommitTextReplacingPreedit },
        )
    }

    // Phase 0 §13 — label matches `docs/architecture/behavioral-invariants.md`.
    @Test
    fun test_INVARIANT_composing_idle_to_idle_is_noop() {
        val (next, transition) =
            ComposingState().apply(ComposingState.Intent.Reset, tl, togglesOff)
        assertFalse(next.isComposing)
        assertTrue(transition.effects.isEmpty())
    }

    // MARK: - Idle invariant: selectedCandidateIndex == -1 on every idle-producing path

    @Test
    fun `every idle-producing path sets selectedCandidateIndex to -1`() {
        val startTL = ComposingState().apply(ComposingState.Intent.Start("a"), tl, togglesOff).first
        val startTLWithIndex = startTL.withSelectedIndex(5)

        val paths =
            listOf(
                ComposingState.Intent.DeleteBackward,
                ComposingState.Intent.CommitDerived,
                ComposingState.Intent.CommitRaw,
                ComposingState.Intent.SelectSuggestion("x"),
                ComposingState.Intent.CommitPreeditThenInsertExternal("😀"),
                ComposingState.Intent.Reset,
            )

        for (intent in paths) {
            val (next, transition) = startTLWithIndex.apply(intent, tl, togglesOff)
            assertFalse("intent $intent should leave state idle", next.isComposing)
            assertEquals("intent $intent should zero selectedCandidateIndex", -1, next.selectedCandidateIndex)
            assertEquals(-1, transition.newSelectedIndex)
        }
    }

    // MARK: - Android-specific: UpdatePreedit text is rawInput (§11 addendum)

    @Test
    fun `live-typing UpdatePreedit carries raw keystrokes not derived form`() {
        // Android divergence from iOS: for Start / Append / ReplaceLast /
        // non-empty DeleteBackward, the UpdatePreedit effect carries the raw
        // buffer AND `transition.derivedDisplay` is intentionally empty.
        // Android skips the synchronous ToneConverter call on the main-thread
        // dispatch path; the async loop in
        // `CandidateUpdateCoordinator.scheduleDisplayDerivation` runs it on
        // `Dispatchers.Default` and calls `ComposingManager.applyDerivedDisplay`
        // to replace the preedit. See `composing-state-boundary.md` §11.10.
        val (_, transition) =
            ComposingState().apply(ComposingState.Intent.Start("gua2"), tl, togglesOff)

        val update =
            transition.effects.filterIsInstance<ComposingTransition.Effect.UpdatePreedit>().single()
        assertEquals("gua2", update.text)
        assertEquals(
            "live-typing path must NOT synchronously derive (main-thread perf)",
            "",
            transition.derivedDisplay,
        )
    }

    @Test
    fun `CommitDerived still computes derivedDisplay synchronously (off hot path)`() {
        // Per-commit derivation is acceptable — it fires once on Enter/Space,
        // not per keystroke. CommitDerived needs the derived string inside
        // the CommitTextReplacingPreedit effect so the platform binding can
        // issue `ic.commitText(derived, 1)` atomically.
        val (afterStart, _) =
            ComposingState().apply(ComposingState.Intent.Start("a"), tl, togglesOff)
        val (_, transition) = afterStart.apply(ComposingState.Intent.CommitDerived, tl, togglesOff)

        val commit =
            transition.effects.filterIsInstance<ComposingTransition.Effect.CommitTextReplacingPreedit>().single()
        assertEquals("a", commit.text)
    }
}
