package com.siansiansu.taigikeyboard.ime.text.composing

/**
 * Platform-neutral description of what [ComposingState.apply] decided.
 *
 * The wrapper ([ComposingManager]) consumes this in two strictly-ordered
 * phases (see `composing-state-boundary.md` §2.4 / §11 Android addendum):
 *
 * 1. swap in [newPhase] + [newSelectedIndex],
 * 2. execute [effects] in order via [ComposingDelegate].
 *
 * [Effect] names describe **behavior**, not iOS/Android APIs. The iOS
 * adapter maps them to `UITextDocumentProxy`; Android maps to
 * `InputConnection` (binding contract — `composing-state-boundary.md`
 * §2.2 table + §11.2 Android-specific rules).
 *
 * Android-specific text-content divergence for [Effect.UpdatePreedit]: iOS
 * emits the tone-marked derived form synchronously, Android emits the raw
 * keystrokes and re-derives off the main thread via
 * [ComposingManager.applyDerivedDisplay]. See `composing-state-boundary.md`
 * §11 Android addendum for rationale.
 */
data class ComposingTransition(
    val newPhase: ComposingState.Phase,
    val newSelectedIndex: Int,
    val effects: List<Effect>,
    val derivedDisplay: String,
) {
    sealed class Effect {
        /** Show [text] as the current preedit (marked text region). */
        data class UpdatePreedit(
            val text: String,
        ) : Effect()

        /**
         * Clear the preedit region WITHOUT committing its current contents
         * to the text document. Android binding requires zero-then-finish
         * (see `composing-state-boundary.md` §11.2 rule 1).
         */
        object ClearPreeditWithoutCommit : Effect()

        /**
         * Atomically replace the current preedit with [text] in the text
         * document. Android binding calls [android.view.inputmethod.InputConnection.commitText]
         * directly — do NOT pre-finish (see §11.2 rule 2).
         */
        data class CommitTextReplacingPreedit(
            val text: String,
        ) : Effect()

        /** Delete one grapheme backward from the backing text document. */
        object DeleteBackwardFromDocument : Effect()

        /** Engine-side: clear the autocomplete suggestion list. */
        object ResetAutocomplete : Effect()

        /** Engine-side: run a fresh autocomplete query for the current raw input. */
        object PerformAutocomplete : Effect()

        /** Engine-side: clear the autocomplete context (selection / bigram history). */
        object ResetAutocompleteContext : Effect()
    }
}
