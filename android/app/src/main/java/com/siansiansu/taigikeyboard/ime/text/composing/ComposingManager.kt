package com.siansiansu.taigikeyboard.ime.text.composing

import android.view.inputmethod.InputConnection
import com.siansiansu.taigikeyboard.ime.dictionary.TPSConverter
import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverter
import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels

/**
 * Manages Taigi input composing state with rawInput as single source of truth.
 *
 * - rawInput: Original keystrokes (e.g. "gua2") — used for Trie search
 * - composingText: Derived display text (e.g. "guá") — computed via deriveDisplay() on every state change
 *
 * Architecture (aligned with iOS):
 * - rawInput is the single source of truth
 * - composingText is derived from rawInput via deriveDisplay() (run off main thread)
 * - deriveDisplay() does: tone convert raw input
 * - On keystroke: rawInput shown immediately as temporary composing text,
 *   then replaced with derived display once background computation completes
 * - On commit (space/enter): deriveDisplay() runs synchronously as safety fallback
 * - Backspace: simple rawInput.dropLast() + async recomputation
 */
class ComposingManager(
    var inputMode: ToneConverterModels.InputMode,
    var enableDoubleTapOO: Boolean = true,
    var enableDoubleTapNN: Boolean = true,
) {
    // rawInput: single source of truth (original keystrokes, e.g. "gua2si7")
    private var rawInput: String = ""
    // composingText: derived display (tone-marked, e.g. "guá sī")
    private var composingText: String = ""
    private var isComposing: Boolean = false
    // Whether composingText needs re-derivation (set true after rawInput changes)
    private var displayDirty: Boolean = false

    // Candidate selection (0 = first candidate by default, matching iOS)
    var selectedCandidateIndex: Int = 0
        private set

    /**
     * Start composing with initial character
     */
    fun startComposing(char: String, ic: InputConnection) {
        if (isComposing) {
            ic.finishComposingText()
        }
        rawInput = char
        composingText = rawInput
        displayDirty = true
        isComposing = true
        selectedCandidateIndex = 0
        updateComposingText(ic)
    }

    /**
     * Append character to composing
     */
    fun appendCharacter(char: String, ic: InputConnection) {
        if (!isComposing) {
            startComposing(char, ic)
            return
        }

        selectedCandidateIndex = 0
        rawInput += char
        composingText = rawInput
        displayDirty = true
        updateComposingText(ic)
    }

    /**
     * Replace the last character of rawInput with a new string.
     * Used by palatalization auto-correct.
     */
    fun replaceLastCharacter(replacement: String, ic: InputConnection) {
        if (!isComposing || rawInput.isEmpty()) return
        rawInput = rawInput.dropLast(1) + replacement
        composingText = rawInput
        displayDirty = true
        updateComposingText(ic)
    }

    /**
     * Append hyphen
     */
    fun appendHyphen(ic: InputConnection) {
        appendCharacter("-", ic)
    }

    /**
     * Delete last character (simple rawInput.dropLast + full recomputation)
     */
    fun deleteBackward(ic: InputConnection): Boolean {
        if (!isComposing || rawInput.isEmpty()) {
            return false
        }

        rawInput = rawInput.dropLast(1)

        if (rawInput.isEmpty()) {
            ic.setComposingText("", 1)
            reset(ic)
            return true
        }

        composingText = rawInput
        displayDirty = true
        updateComposingText(ic)
        return true
    }

    /**
     * Commit composition (finalize composing text)
     */
    fun commitComposition(ic: InputConnection) {
        if (!isComposing || rawInput.isEmpty()) {
            return
        }

        // Ensure display is fully derived before committing to the editor.
        // This is a synchronous fallback for the rare case where the async
        // derivation hasn't completed yet (e.g. very fast typing then space/enter).
        if (displayDirty) {
            composingText = deriveDisplay(rawInput)
            updateComposingText(ic)
        }

        rawInput = ""
        composingText = ""
        isComposing = false
        displayDirty = false
        selectedCandidateIndex = -1
        ic.finishComposingText()
    }

    /**
     * Select a suggestion candidate
     */
    fun selectSuggestion(suggestion: String, ic: InputConnection) {
        if (!isComposing) {
            return
        }

        rawInput = ""
        composingText = ""
        isComposing = false
        displayDirty = false
        selectedCandidateIndex = -1

        ic.setComposingText(suggestion, 1)
        ic.finishComposingText()
    }

    /**
     * Reset all composing state
     */
    fun reset(ic: InputConnection) {
        if (isComposing) {
            ic.finishComposingText()
        }
        rawInput = ""
        composingText = ""
        isComposing = false
        displayDirty = false
        selectedCandidateIndex = -1
    }

    /**
     * Get current composing text (for UI display)
     */
    fun getComposingText(): String? {
        return if (isComposing) composingText else null
    }

    /**
     * Get raw input (for Trie search)
     */
    fun getRawInput(): String? {
        return if (isComposing) rawInput else null
    }

    /**
     * Whether currently composing
     */
    fun isComposing(): Boolean = isComposing

    // MARK: - Display Derivation

    /**
     * Apply a pre-computed derived display text to composing state and InputConnection.
     * Called from TextInputManager after background display derivation completes.
     */
    internal fun applyDerivedDisplay(derivedText: String, ic: InputConnection) {
        if (!isComposing) return
        composingText = derivedText
        displayDirty = false
        updateComposingText(ic)
    }

    /**
     * Derive display text from raw input.
     *
     * Converts raw input to tone-marked form via ToneConverter.
     * ToneConverter already handles hyphen-separated syllables internally
     * (splits by "-", converts each syllable, rejoins with "-").
     *
     * Aligned with iOS ComposingManager.deriveDisplay().
     */
    internal fun deriveDisplay(raw: String): String {
        if (raw.isEmpty()) return ""

        // TPS input: display as-is (TPS symbols are already visual, no tone conversion needed)
        if (TPSConverter.containsTPS(raw)) return raw

        return ToneConverter.convertToToneMarks(raw, inputMode, enableDoubleTapOO, enableDoubleTapNN)
    }

    /**
     * Update InputConnection composing text
     */
    private fun updateComposingText(ic: InputConnection) {
        ic.setComposingText(composingText, 1)
    }

}
