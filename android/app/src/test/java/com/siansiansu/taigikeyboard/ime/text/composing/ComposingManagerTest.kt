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
import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels
import org.junit.Assert.assertEquals
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
    private fun newManager(): ComposingManager =
        ComposingManager(
            inputMode = ToneConverterModels.InputMode.TL,
            enableDoubleTapOO = false,
            enableDoubleTapNN = false,
        )

    // MARK: - INVARIANT_composing_clear_preedit_does_not_commit

    @Test
    fun `reset on composing pre-zeros before finishComposingText`() {
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

    // MARK: - INVARIANT_composing_reset_when_idle_is_noop

    @Test
    fun `reset when idle issues no InputConnection calls`() {
        val manager = newManager()
        val ic = RecordingInputConnection()

        manager.reset(ic)

        assertTrue("Idle reset must not touch InputConnection", ic.calls.isEmpty())
    }

    // MARK: - INVARIANT_composing_clear_preedit_does_not_commit (startComposing path)

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
