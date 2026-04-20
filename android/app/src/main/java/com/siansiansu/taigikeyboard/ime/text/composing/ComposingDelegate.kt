// NOTE: Not shared-core — platform adapter binding ComposingTransition.Effect
// values to android.view.inputmethod.InputConnection. Owns the Android-specific
// zero-then-finish semantics for clear-preedit; see
// docs/architecture/composing-state-boundary.md §11.2.
package com.siansiansu.taigikeyboard.ime.text.composing

import android.view.inputmethod.InputConnection

/**
 * Platform adapter that executes a [ComposingTransition.Effect] against
 * the host [InputConnection].
 *
 * Mirrors iOS `Input/Composing/ComposingDelegate.swift` — both platforms
 * translate the platform-neutral Effect enum; iOS against `UITextDocumentProxy`,
 * Android against [InputConnection].
 *
 * Binding contract lives in `docs/architecture/composing-state-boundary.md`
 * §2.2 (cross-platform table) + §11.2 (Android-specific rules: zero-then-finish
 * for clear-preedit, atomic `commitText` for replace-preedit, document ops
 * after preedit ops).
 *
 * Platform-side (imports [InputConnection]) — NOT a shared-core candidate.
 * Engine-side effects (`ResetAutocomplete` / `PerformAutocomplete` /
 * `ResetAutocompleteContext`) are no-ops at this layer — the existing
 * [com.siansiansu.taigikeyboard.ime.text.CandidateUpdateCoordinator] flow
 * handles candidate/autocomplete updates triggered from the wrapper's
 * caller.
 */
fun interface ComposingDelegate {
    fun execute(
        effect: ComposingTransition.Effect,
        ic: InputConnection,
    )
}

/**
 * Default [ComposingDelegate] binding [ComposingTransition.Effect] values
 * to [InputConnection] calls per the §2.2 mapping table and §11.2
 * Android-specific rules.
 */
object DefaultComposingDelegate : ComposingDelegate {
    override fun execute(
        effect: ComposingTransition.Effect,
        ic: InputConnection,
    ) {
        when (effect) {
            is ComposingTransition.Effect.UpdatePreedit -> {
                ic.setComposingText(effect.text, 1)
            }

            ComposingTransition.Effect.ClearPreeditWithoutCommit -> {
                // INVARIANT_composing_clear_preedit_does_not_commit:
                // finishComposingText() commits the active composing region,
                // so zero it first (see composing-state-boundary.md §11.2
                // rule 1 + behavioral-invariants.md §13).
                ic.setComposingText("", 1)
                ic.finishComposingText()
            }

            is ComposingTransition.Effect.CommitTextReplacingPreedit -> {
                // Atomic commit — do NOT pre-finish (§11.2 rule 2).
                // `commitText` replaces the composing region and clears it
                // in one call.
                ic.commitText(effect.text, 1)
            }

            ComposingTransition.Effect.DeleteBackwardFromDocument -> {
                ic.deleteSurroundingText(1, 0)
            }

            // Engine-side effects — no InputConnection calls. The wrapper's
            // caller (TextInputManager / CandidateClickHandler) triggers the
            // autocomplete / next-word updates after the dispatch returns.
            ComposingTransition.Effect.ResetAutocomplete,
            ComposingTransition.Effect.PerformAutocomplete,
            ComposingTransition.Effect.ResetAutocompleteContext,
            -> {
                Unit
            }
        }
    }
}
