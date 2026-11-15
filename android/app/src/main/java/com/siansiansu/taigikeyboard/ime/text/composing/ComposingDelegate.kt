package com.siansiansu.taigikeyboard.ime.text.composing

import android.view.inputmethod.InputConnection
import com.siansiansu.taigikeyboard.engine.ComposingTransition

/**
 * Platform adapter that executes a [ComposingTransition.Effect]
 * against the host [InputConnection]. Mirrors iOS
 * `Input/Composing/ComposingDelegate.swift`.
 *
 * Binding contract: `composing-state-boundary.md` §2.2 (cross-platform table)
 * + §11.2 (Android-specific rules: zero-then-finish for clear-preedit;
 * atomic `commitText` for replace-preedit).
 *
 * Engine-side effects (`ClearCandidates` / `RefreshCandidates` /
 * `ResetCandidateContext`) are no-ops at this layer — the existing
 * [com.siansiansu.taigikeyboard.ime.text.CandidateUpdateCoordinator] flow
 * handles candidate / autocomplete updates triggered from the wrapper's
 * caller.
 */
fun interface ComposingDelegate {
    fun execute(
        effect: ComposingTransition.Effect,
        ic: InputConnection,
    )
}

object DefaultComposingDelegate : ComposingDelegate {
    override fun execute(
        effect: ComposingTransition.Effect,
        ic: InputConnection,
    ) {
        when (effect) {
            is ComposingTransition.Effect.UpdatePreedit -> {
                ic.setComposingText(effect.display, 1)
            }

            ComposingTransition.Effect.ClearPreeditWithoutCommit -> {
                // INVARIANT_composing_clear_preedit_does_not_commit:
                // finishComposingText alone commits the active region; zero first.
                ic.setComposingText("", 1)
                ic.finishComposingText()
            }

            is ComposingTransition.Effect.CommitTextReplacingPreedit -> {
                // Atomic — `commitText` replaces the composing region in one call.
                ic.commitText(effect.text, 1)
            }

            ComposingTransition.Effect.ClearCandidates,
            ComposingTransition.Effect.RefreshCandidates,
            ComposingTransition.Effect.ResetCandidateContext,
            -> {
                Unit
            }

            is ComposingTransition.Effect.NextWordUpdateLastSelectedWord,
            is ComposingTransition.Effect.NextWordWordSelected,
            ComposingTransition.Effect.NextWordClearForNewComposing,
            -> {
                // NextWord-shaped effects flow through the sibling
                // `NextWordEffectRouter` injected into ComposingManager, not
                // the InputConnection-bound delegate. Listed exhaustively so
                // a future Effect variant fails compile here.
                Unit
            }
        }
    }
}

/**
 * Sibling of [ComposingDelegate] for v3.5.8 Phase 4 NextWord-shaped Effects
 * (`NextWordUpdateLastSelectedWord` / `NextWordWordSelected` /
 * `NextWordClearForNewComposing`). Decoupled because these targets are
 * `NextWordController`, not [InputConnection].
 *
 * [ComposingManager.applyTransition] dispatches each NextWord-shaped Effect
 * here in proto-list order, sandwiched alongside [DefaultComposingDelegate]
 * calls for the InputConnection-bound effects in the same Effect[] response.
 *
 * Default implementation = [NoopNextWordEffectRouter]; tests + non-IME
 * callers don't need to wire NextWord plumbing.
 */
fun interface NextWordEffectRouter {
    fun route(effect: ComposingTransition.Effect)
}

object NoopNextWordEffectRouter : NextWordEffectRouter {
    override fun route(effect: ComposingTransition.Effect) {
        // intentional no-op — see KDoc on NextWordEffectRouter
    }
}
