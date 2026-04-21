package com.siansiansu.taigikeyboard.ime.text.composing

import android.os.Bundle
import android.text.TextUtils
import android.view.KeyEvent
import android.view.inputmethod.CompletionInfo
import android.view.inputmethod.CorrectionInfo
import android.view.inputmethod.ExtractedText
import android.view.inputmethod.ExtractedTextRequest
import android.view.inputmethod.InputConnection
import android.view.inputmethod.InputContentInfo
import com.siansiansu.taigikeyboard.ime.core.settings.EngineSettings
import com.siansiansu.taigikeyboard.ime.core.settings.EngineSettingsProvider
import com.siansiansu.taigikeyboard.ime.core.settings.ToneToggles
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Pins binding-side parity with iOS for `ComposingManager.reset(ic)`.
 *
 * iOS counterpart: `ComposingState.apply(.reset)` emits
 * `[.clearPreeditWithoutCommit, .resetAutocomplete]`; the iOS binding
 * (`KeyboardViewController+TextInput.swift`) calls `clearMarkedText()`
 * which never inserts text. Android `InputConnection.finishComposingText()`
 * silently commits the active composing region — so the binding must
 * pre-zero with `setComposingText("", 1)` first.
 *
 * See `docs/architecture/composing-state-boundary.md` §11.6 and
 * `docs/architecture/behavioral-invariants.md` §13.
 */
class ComposingManagerTest {
    private fun newManager(): ComposingManager = ComposingManager(settingsProvider = FakeSettingsProvider())

    // MARK: - Phase 0 §13 — composing-buffer reset invariants.
    // Labels match `docs/architecture/behavioral-invariants.md` §13.

    @Test
    fun test_INVARIANT_composing_clear_preedit_does_not_commit() {
        val manager = newManager()
        val ic = RecordingInputConnection()
        manager.appendCharacter("a", ic)
        ic.calls.clear()

        manager.reset(ic)

        // Order matters: zero-then-finish prevents finishComposingText
        // from silently committing the preedit.
        assertEquals(
            listOf(
                IcCall.SetComposingText("", 1),
                IcCall.FinishComposingText,
            ),
            ic.calls,
        )
        // No commitText anywhere — that would defeat the invariant.
        assertTrue(ic.calls.none { it is IcCall.CommitText })
    }

    @Test
    fun `deleteBackward to empty also pre-zeros via reset`() {
        // After moving the pre-zero into reset(), the deleteBackward
        // empty-raw path must still satisfy zero-then-finish via reset.
        val manager = newManager()
        val ic = RecordingInputConnection()
        manager.appendCharacter("a", ic)
        ic.calls.clear()

        manager.deleteBackward(ic)

        assertEquals(
            listOf(
                IcCall.SetComposingText("", 1),
                IcCall.FinishComposingText,
            ),
            ic.calls,
        )
        assertTrue(ic.calls.none { it is IcCall.CommitText })
    }

    @Test
    fun test_INVARIANT_composing_reset_when_idle_is_noop() {
        val manager = newManager()
        val ic = RecordingInputConnection()

        manager.reset(ic)

        assertTrue("Idle reset must not touch InputConnection", ic.calls.isEmpty())
    }

    @Test
    fun `startComposing over an active preedit pre-zeros before finish`() {
        // Mid-composition restart must not silently commit the previous
        // preedit. Same invariant as reset(ic) — different entry point.
        val manager = newManager()
        val ic = RecordingInputConnection()
        manager.appendCharacter("a", ic)
        ic.calls.clear()

        manager.startComposing("b", ic)

        // First two calls must be the pre-zero pair; the third is the
        // fresh-composition update.
        assertEquals(IcCall.SetComposingText("", 1), ic.calls[0])
        assertEquals(IcCall.FinishComposingText, ic.calls[1])
        assertEquals(IcCall.SetComposingText("b", 1), ic.calls[2])
        assertTrue(ic.calls.none { it is IcCall.CommitText })
    }

    // MARK: - CommitTextReplacingPreedit binding (atomic commitText, no pre-finish)

    @Test
    fun `commitComposition slow path uses atomic commitText when async has not caught up`() {
        // Slow path: `cachedDerivedDisplay` is empty (unit test harness
        // never runs CandidateUpdateCoordinator's async derivation). Wrapper
        // routes through CommitDerived → CommitTextReplacingPreedit →
        // ic.commitText. §11.2 rule 2: no pre-finish.
        val manager = newManager()
        val ic = RecordingInputConnection()
        manager.appendCharacter("a", ic)
        ic.calls.clear()

        manager.commitComposition(ic)

        assertEquals(1, ic.calls.size)
        assertTrue(ic.calls[0] is IcCall.CommitText)
        val commit = ic.calls[0] as IcCall.CommitText
        assertEquals(1, commit.newCursorPosition)
        assertTrue(ic.calls.none { it is IcCall.FinishComposingText })
    }

    @Test
    fun `commitComposition fast path uses finishComposingText after async derivation`() {
        // Fast path: after async derivation populates `cachedDerivedDisplay`
        // (simulated here via `applyDerivedDisplay`), commit issues
        // `finishComposingText()` only — no commitText. This preserves the
        // pre-A4 displayDirty=false behavior where an externally-cleared
        // composing region (cursor move) results in a no-op finalize
        // instead of a duplicate text insertion at the new cursor.
        val manager = newManager()
        val ic = RecordingInputConnection()
        manager.appendCharacter("a", ic)
        manager.applyDerivedDisplay("a", ic) // simulate async path completing
        ic.calls.clear()

        manager.commitComposition(ic)

        assertEquals(listOf(IcCall.FinishComposingText), ic.calls)
        assertTrue(ic.calls.none { it is IcCall.CommitText })
        assertFalse(manager.isComposing())
    }

    @Test
    fun `selectSuggestion commits suggestion atomically without pre-finish`() {
        val manager = newManager()
        val ic = RecordingInputConnection()
        manager.appendCharacter("a", ic)
        ic.calls.clear()

        manager.selectSuggestion("chosen", ic)

        assertEquals(
            listOf(IcCall.CommitText("chosen", 1)),
            ic.calls,
        )
        // selectSuggestion also clears composing state — sanity check.
        assertFalse(manager.isComposing())
    }

    @Test
    fun `commitRawInput commits the raw keystrokes`() {
        val manager = newManager()
        val ic = RecordingInputConnection()
        manager.appendCharacter("a", ic)
        manager.appendCharacter("b", ic)
        ic.calls.clear()

        manager.commitRawInput(ic)

        assertEquals(
            listOf(IcCall.CommitText("ab", 1)),
            ic.calls,
        )
        assertFalse(manager.isComposing())
    }

    // MARK: - Live-typing preedit text (Android raw vs iOS derived — §11 addendum)

    // MARK: - INVARIANT_composing_external_insert_commits_preedit_atomically

    @Test
    fun `commitPreeditThenInsertExternal when composing emits single atomic commitText`() {
        // Pre-A5 `sendEmojiKeyPress` used `finishComposingText` + `commitText` —
        // a silent double-commit flagged in
        // `composing-state-boundary.md` §11.6 deferred parity follow-up.
        // Post-A5 the binding must issue ONE `commitText` containing the
        // derived preedit + external text, with zero `finishComposingText`
        // calls.
        val manager = newManager()
        val ic = RecordingInputConnection()
        manager.appendCharacter("a", ic)
        ic.calls.clear()

        manager.commitPreeditThenInsertExternal("😀", ic)

        assertEquals(1, ic.calls.size)
        val commit = ic.calls.single() as IcCall.CommitText
        assertTrue(
            "commit text must carry preedit + emoji in one write",
            commit.text.endsWith("😀") && commit.text.length > "😀".length,
        )
        assertEquals(1, commit.newCursorPosition)
        assertTrue(
            "must NOT pre-finish the composing region (double-commit risk)",
            ic.calls.none { it is IcCall.FinishComposingText },
        )
        assertFalse(manager.isComposing())
    }

    @Test
    fun `commitPreeditThenInsertExternal when idle emits plain commitText`() {
        val manager = newManager()
        val ic = RecordingInputConnection()

        manager.commitPreeditThenInsertExternal("😀", ic)

        assertEquals(
            listOf(IcCall.CommitText("😀", 1)),
            ic.calls,
        )
        assertTrue(ic.calls.none { it is IcCall.FinishComposingText })
    }

    @Test
    fun `commitPreeditThenInsertExternal with empty text is noop and preserves preedit`() {
        val manager = newManager()
        val ic = RecordingInputConnection()
        manager.appendCharacter("a", ic)
        ic.calls.clear()

        manager.commitPreeditThenInsertExternal("", ic)

        assertTrue("Empty external insert must not touch InputConnection", ic.calls.isEmpty())
        assertTrue("Active preedit must be preserved", manager.isComposing())
    }

    // MARK: - INVARIANT_composing_external_region_clear_discards_state
    // Host editor cleared the composing region externally (cursor move,
    // selection change). Binding must zero internal state without touching
    // InputConnection — the region is already gone.

    @Test
    fun test_INVARIANT_composing_external_region_clear_discards_state() {
        val manager = newManager()
        val ic = RecordingInputConnection()
        manager.appendCharacter("a", ic)
        ic.calls.clear()

        manager.onExternalComposingRegionCleared()

        assertFalse("State must exit to idle after external region clear", manager.isComposing())
        assertTrue(
            "Hook must not touch InputConnection — region already cleared by host",
            ic.calls.isEmpty(),
        )
    }

    @Test
    fun `onExternalComposingRegionCleared when idle is noop`() {
        val manager = newManager()
        val ic = RecordingInputConnection()

        manager.onExternalComposingRegionCleared()

        assertFalse(manager.isComposing())
        assertTrue(ic.calls.isEmpty())
    }

    @Test
    fun `onExternalComposingRegionCleared short-circuits subsequent commit via idle state`() {
        // Root-cause regression pin for `composing-state-boundary.md` §11.10
        // divergence #3: before this hook existed, an external region clear
        // left `state.isComposing` true, so a later `commitComposition(ic)`
        // would (fast-path) issue a stale `finishComposingText()` OR
        // (slow-path) commit derived text at the new cursor position. After
        // the hook, commitComposition early-returns via the `isComposing`
        // guard. This test pins the idle-state short-circuit — cache clear
        // is pinned separately by `…clears cachedDerivedDisplay field`.
        val manager = newManager()
        val ic = RecordingInputConnection()
        manager.appendCharacter("a", ic)
        manager.applyDerivedDisplay("a", ic)
        ic.calls.clear()

        manager.onExternalComposingRegionCleared()
        manager.commitComposition(ic)

        assertTrue(
            "commit after external clear must not touch InputConnection",
            ic.calls.isEmpty(),
        )
        assertFalse(manager.isComposing())
    }

    @Test
    fun `onExternalComposingRegionCleared clears cachedDerivedDisplay field`() {
        // Reflection-based pin: no public API exposes cachedDerivedDisplay
        // independently of `state.isComposing` (getComposingText() returns
        // null when state is idle; every dispatch() re-zeroes the cache).
        // Without this direct pin, a future refactor that resets only
        // `state` and forgets `cachedDerivedDisplay` would leak stale
        // derived text through `applyDerivedDisplay` → `getComposingText`
        // if `dispatch()`'s own cache reset is ever removed.
        val manager = newManager()
        val ic = RecordingInputConnection()
        manager.appendCharacter("a", ic)
        manager.applyDerivedDisplay("á", ic)
        val cacheField = ComposingManager::class.java.getDeclaredField("cachedDerivedDisplay")
        cacheField.isAccessible = true
        assertEquals("á", cacheField.get(manager) as String)

        manager.onExternalComposingRegionCleared()

        assertEquals(
            "hook must clear cachedDerivedDisplay, not only state",
            "",
            cacheField.get(manager) as String,
        )
    }

    @Test
    fun `hostReportsNoComposingRegion encodes the -1,-1 policy`() {
        // Policy extracted as a top-level function so TextInputManager's
        // onUpdateSelection override can delegate without pulling
        // android.* classes into unit tests. Both coordinates must be -1.
        assertTrue(hostReportsNoComposingRegion(candidatesStart = -1, candidatesEnd = -1))

        assertFalse(hostReportsNoComposingRegion(candidatesStart = 0, candidatesEnd = 1))
        assertFalse(hostReportsNoComposingRegion(candidatesStart = 5, candidatesEnd = 8))
        assertFalse(hostReportsNoComposingRegion(candidatesStart = -1, candidatesEnd = 5))
        assertFalse(hostReportsNoComposingRegion(candidatesStart = 5, candidatesEnd = -1))
    }

    @Test
    fun `appendCharacter shows raw keystrokes in preedit (Android divergence)`() {
        // Android emits rawInput for UpdatePreedit on live-typing intents;
        // iOS emits the tone-marked derived form synchronously. Both platforms
        // observe the same final committed text — see composing-state-boundary.md
        // §11 Android addendum for the rationale.
        val manager = newManager()
        val ic = RecordingInputConnection()

        manager.appendCharacter("g", ic)
        manager.appendCharacter("u", ic)
        manager.appendCharacter("a", ic)
        manager.appendCharacter("2", ic)

        // Last preedit update carries the raw buffer, not the derived form.
        val lastSetComposing = ic.calls.last { it is IcCall.SetComposingText } as IcCall.SetComposingText
        assertEquals("gua2", lastSetComposing.text)
    }
}

private sealed interface IcCall {
    data class SetComposingText(
        val text: String,
        val newCursorPosition: Int,
    ) : IcCall

    data class CommitText(
        val text: String,
        val newCursorPosition: Int,
    ) : IcCall

    object FinishComposingText : IcCall
}

/**
 * Hand-rolled `InputConnection` test double. Records the calls the
 * composing-buffer invariants depend on; every other method returns a
 * default value (no-op).
 *
 * Avoids pulling Mockito/mockk into a project that uses plain JUnit 4.
 */
private class RecordingInputConnection : InputConnection {
    val calls: MutableList<IcCall> = mutableListOf()

    override fun setComposingText(
        text: CharSequence?,
        newCursorPosition: Int,
    ): Boolean {
        calls += IcCall.SetComposingText(text?.toString().orEmpty(), newCursorPosition)
        return true
    }

    override fun commitText(
        text: CharSequence?,
        newCursorPosition: Int,
    ): Boolean {
        calls += IcCall.CommitText(text?.toString().orEmpty(), newCursorPosition)
        return true
    }

    override fun finishComposingText(): Boolean {
        calls += IcCall.FinishComposingText
        return true
    }

    // Below: required interface methods we do not exercise. Defaults
    // match android.view.inputmethod.InputConnection's typical no-op
    // returns; never throw, so unrelated calls cannot break the test.
    override fun getTextBeforeCursor(
        n: Int,
        flags: Int,
    ): CharSequence? = ""

    override fun getTextAfterCursor(
        n: Int,
        flags: Int,
    ): CharSequence? = ""

    override fun getSelectedText(flags: Int): CharSequence? = null

    override fun getCursorCapsMode(reqModes: Int): Int = 0

    override fun getExtractedText(
        request: ExtractedTextRequest?,
        flags: Int,
    ): ExtractedText? = null

    override fun deleteSurroundingText(
        beforeLength: Int,
        afterLength: Int,
    ): Boolean = true

    override fun deleteSurroundingTextInCodePoints(
        beforeLength: Int,
        afterLength: Int,
    ): Boolean = true

    override fun setComposingRegion(
        start: Int,
        end: Int,
    ): Boolean = true

    override fun commitCompletion(text: CompletionInfo?): Boolean = true

    override fun commitCorrection(correctionInfo: CorrectionInfo?): Boolean = true

    override fun setSelection(
        start: Int,
        end: Int,
    ): Boolean = true

    override fun performEditorAction(editorAction: Int): Boolean = true

    override fun performContextMenuAction(id: Int): Boolean = true

    override fun beginBatchEdit(): Boolean = true

    override fun endBatchEdit(): Boolean = true

    override fun sendKeyEvent(event: KeyEvent?): Boolean = true

    override fun clearMetaKeyStates(states: Int): Boolean = true

    override fun reportFullscreenMode(enabled: Boolean): Boolean = true

    override fun performPrivateCommand(
        action: String?,
        data: Bundle?,
    ): Boolean = true

    override fun requestCursorUpdates(cursorUpdateMode: Int): Boolean = true

    override fun getHandler(): android.os.Handler? = null

    override fun closeConnection() = Unit

    override fun commitContent(
        inputContentInfo: InputContentInfo,
        flags: Int,
        opts: Bundle?,
    ): Boolean = true
}

/**
 * Minimal [EngineSettingsProvider] for tests. TL mode, toggles off — matches
 * the old hardcoded ctor defaults (`inputMode = TL`, `enableDoubleTapOO = false`,
 * `enableDoubleTapNN = false`) so tests predating A4-impl retain identical
 * behavior.
 */
private class FakeSettingsProvider : EngineSettingsProvider {
    override val current: EngineSettings = FakeEngineSettings()
}

private class FakeEngineSettings : EngineSettings {
    override val inputMode: String = "tl"
    override val isAutoCap: Boolean = false
    override val isTranslateSwapped: Boolean = false
    override val isAssociationRecordingEnabled: Boolean = false
    override val toneToggles: ToneToggles =
        ToneToggles(isDoubleTapOOEnabled = false, isDoubleTapNNEnabled = false)
    override val isCustomDictEnabled: Boolean = false
    override val isTpsOrMappedToER: Boolean = false
    override val isMoeDictEnabled: Boolean = false
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
