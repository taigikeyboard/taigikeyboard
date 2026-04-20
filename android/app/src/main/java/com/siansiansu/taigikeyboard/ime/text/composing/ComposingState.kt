package com.siansiansu.taigikeyboard.ime.text.composing

import com.siansiansu.taigikeyboard.ime.core.settings.ToneToggles
import com.siansiansu.taigikeyboard.ime.dictionary.TPSConverter
import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverter
import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels

/**
 * Platform-neutral composing-buffer state machine.
 *
 * `rawInput` (original keystrokes, e.g. `"gua2"`) is the single source of
 * truth; [derivedDisplay] recomputes the tone-marked form via
 * [ToneConverter]. The wrapper layer ([ComposingManager] on Android) owns
 * `InputConnection` side effects and settings reads — this type stays
 * Kotlin-stdlib-pure so future shared-core extraction can reuse it verbatim.
 *
 * Every intent returns `(newState, ComposingTransition)`; the transition's
 * effects list is the platform-neutral side-effect script the wrapper must
 * execute in order. No hidden side channels.
 *
 * Kotlin mirrors iOS `Input/Composing/ComposingState.swift`. The Swift
 * version is a `mutating` struct; Kotlin idiom is an immutable `data class`
 * that returns a new state — semantically equivalent (see
 * `rules/android-guidelines.md` §1 Kotlin-to-Rust shape preferences).
 */
data class ComposingState(
    val phase: Phase = Phase.Idle,
    /**
     * `-1` in idle; `0` when a fresh composition begins; preserved across
     * [Intent.ReplaceLast] and candidate-bar taps. UI never writes this
     * directly — [apply] and [withSelectedIndex] are the only mutation
     * paths so the pure state cannot desync from the platform wrapper.
     */
    val selectedCandidateIndex: Int = -1,
) {
    sealed class Phase {
        object Idle : Phase()

        data class Composing(
            val raw: String,
        ) : Phase()
    }

    sealed class Intent {
        /** Begin a new composition buffer with [text] (caret reset). */
        data class Start(
            val text: String,
        ) : Intent()

        /** Append [char]. If currently idle, acts as [Start]. */
        data class Append(
            val char: String,
        ) : Intent()

        /** Convenience alias for `Append("-")` (TPS / romanization hyphenation). */
        object AppendHyphen : Intent()

        /**
         * Replace the last raw-input character (TPS auto-correct). Intentionally
         * preserves [selectedCandidateIndex] — unlike [Append], which resets
         * it to `0`.
         */
        data class ReplaceLast(
            val replacement: String,
        ) : Intent()

        /** Delete one grapheme. Exits to idle when the buffer empties. */
        object DeleteBackward : Intent()

        /** Commit the [derivedDisplay] form (tone-marked) to the document. */
        object CommitDerived : Intent()

        /**
         * Commit the literal `rawInput` (bypass tone conversion). Mirror of
         * iOS `.commitRaw`; no Android call-site triggers this today, but
         * the state supports it so pure-state tests match iOS.
         */
        object CommitRaw : Intent()

        /**
         * Commit [text] to the document, atomically replacing the current
         * preedit.
         */
        data class SelectSuggestion(
            val text: String,
        ) : Intent()

        /** Clear all state (e.g. keyboard teardown / mode switch). */
        object Reset : Intent()
    }

    val isComposing: Boolean
        get() = phase is Phase.Composing

    val rawInput: String
        get() = (phase as? Phase.Composing)?.raw ?: ""

    /**
     * Derive the display form for the current [rawInput]. TPS symbols are
     * already display-ready; POJ/TL route through [ToneConverter] with the
     * supplied toggles.
     *
     * Kept synchronous and pure so commit-path intents (see [apply]) can
     * attach the derived text to the emitted effect without extra
     * scheduling. Live-typing intents emit the raw preedit instead and
     * the wrapper re-derives off-main-thread (see
     * `composing-state-boundary.md` §11 Android addendum).
     */
    fun derivedDisplay(
        mode: ToneConverterModels.InputMode,
        toggles: ToneToggles,
    ): String {
        val raw = rawInput
        if (raw.isEmpty()) return ""
        if (TPSConverter.containsTPS(raw)) return raw
        return ToneConverter.convertToToneMarks(raw, mode, toggles)
    }

    /** Returns a new state with [selectedCandidateIndex] set to [index]. */
    fun withSelectedIndex(index: Int): ComposingState = copy(selectedCandidateIndex = index)

    /**
     * Apply an [intent] and emit the resulting transition.
     *
     * Returns `(newState, transition)`. The transition's [ComposingTransition.effects]
     * is the ordered side-effect script the wrapper executes against
     * [ComposingDelegate].
     *
     * Android-specific text-content note: [ComposingTransition.Effect.UpdatePreedit]
     * carries the RAW keystrokes for live-typing intents (Start / Append /
     * ReplaceLast / non-empty-result DeleteBackward). Commit-path intents
     * (CommitDerived / CommitRaw / SelectSuggestion) carry the synchronously
     * derived form in [ComposingTransition.Effect.CommitTextReplacingPreedit].
     * Rationale + iOS divergence documented in `composing-state-boundary.md`
     * §11 Android addendum.
     */
    fun apply(
        intent: Intent,
        mode: ToneConverterModels.InputMode,
        toggles: ToneToggles,
    ): Pair<ComposingState, ComposingTransition> =
        when (intent) {
            is Intent.Start -> {
                enterComposing(raw = intent.text, mode = mode, toggles = toggles)
            }

            is Intent.Append -> {
                if (phase is Phase.Idle) {
                    enterComposing(raw = intent.char, mode = mode, toggles = toggles)
                } else {
                    enterComposing(raw = rawInput + intent.char, mode = mode, toggles = toggles)
                }
            }

            Intent.AppendHyphen -> {
                apply(Intent.Append("-"), mode, toggles)
            }

            is Intent.ReplaceLast -> {
                if (phase !is Phase.Composing || rawInput.isEmpty()) {
                    noopTransition()
                } else {
                    // ReplaceLast preserves selectedCandidateIndex — it is a
                    // correction on top of an in-progress selection, not a
                    // fresh composition step (see iOS ComposingStateTests).
                    val newRaw = rawInput.dropLast(1) + intent.replacement
                    val newState = copy(phase = Phase.Composing(newRaw))
                    val transition =
                        ComposingTransition(
                            newPhase = newState.phase,
                            newSelectedIndex = selectedCandidateIndex,
                            effects =
                                listOf(
                                    ComposingTransition.Effect.UpdatePreedit(newRaw),
                                    ComposingTransition.Effect.PerformAutocomplete,
                                ),
                            // Live-typing: skip sync ToneConverter call on the
                            // main thread. Async derivation via
                            // CandidateUpdateCoordinator handles the derived
                            // form off Dispatchers.Default (see §11.10).
                            derivedDisplay = "",
                        )
                    newState to transition
                }
            }

            Intent.DeleteBackward -> {
                if (phase !is Phase.Composing || rawInput.isEmpty()) {
                    noopTransition()
                } else {
                    val newRaw = rawInput.dropLast(1)
                    if (newRaw.isEmpty()) {
                        exitToIdle(
                            effects =
                                listOf(
                                    ComposingTransition.Effect.ClearPreeditWithoutCommit,
                                    ComposingTransition.Effect.ResetAutocomplete,
                                    ComposingTransition.Effect.DeleteBackwardFromDocument,
                                ),
                        )
                    } else {
                        val newState = ComposingState(phase = Phase.Composing(newRaw), selectedCandidateIndex = 0)
                        val transition =
                            ComposingTransition(
                                newPhase = newState.phase,
                                newSelectedIndex = 0,
                                effects =
                                    listOf(
                                        ComposingTransition.Effect.UpdatePreedit(newRaw),
                                        ComposingTransition.Effect.PerformAutocomplete,
                                    ),
                                // Live-typing: skip sync derivation (§11.10).
                                derivedDisplay = "",
                            )
                        newState to transition
                    }
                }
            }

            Intent.CommitDerived -> {
                if (phase !is Phase.Composing) {
                    noopTransition()
                } else {
                    val derived = derivedDisplay(mode, toggles)
                    if (derived.isEmpty()) {
                        noopTransition()
                    } else {
                        exitToIdle(
                            effects =
                                listOf(
                                    ComposingTransition.Effect.CommitTextReplacingPreedit(derived),
                                    ComposingTransition.Effect.ResetAutocomplete,
                                    ComposingTransition.Effect.ResetAutocompleteContext,
                                ),
                        )
                    }
                }
            }

            Intent.CommitRaw -> {
                if (phase !is Phase.Composing || rawInput.isEmpty()) {
                    noopTransition()
                } else {
                    val text = rawInput
                    exitToIdle(
                        effects =
                            listOf(
                                ComposingTransition.Effect.CommitTextReplacingPreedit(text),
                                ComposingTransition.Effect.ResetAutocomplete,
                                ComposingTransition.Effect.ResetAutocompleteContext,
                            ),
                    )
                }
            }

            is Intent.SelectSuggestion -> {
                if (phase !is Phase.Composing) {
                    noopTransition()
                } else {
                    exitToIdle(
                        effects =
                            listOf(
                                ComposingTransition.Effect.CommitTextReplacingPreedit(intent.text),
                                ComposingTransition.Effect.ResetAutocomplete,
                                ComposingTransition.Effect.ResetAutocompleteContext,
                            ),
                    )
                }
            }

            Intent.Reset -> {
                if (phase is Phase.Idle) {
                    noopTransition()
                } else {
                    exitToIdle(
                        effects =
                            listOf(
                                ComposingTransition.Effect.ClearPreeditWithoutCommit,
                                ComposingTransition.Effect.ResetAutocomplete,
                            ),
                    )
                }
            }
        }

    private fun enterComposing(
        raw: String,
        @Suppress("UNUSED_PARAMETER") mode: ToneConverterModels.InputMode,
        @Suppress("UNUSED_PARAMETER") toggles: ToneToggles,
    ): Pair<ComposingState, ComposingTransition> {
        val newState = ComposingState(phase = Phase.Composing(raw), selectedCandidateIndex = 0)
        val transition =
            ComposingTransition(
                newPhase = newState.phase,
                newSelectedIndex = 0,
                effects =
                    listOf(
                        ComposingTransition.Effect.UpdatePreedit(raw),
                        ComposingTransition.Effect.PerformAutocomplete,
                    ),
                // Live-typing: skip sync ToneConverter call on the main
                // thread. The wrapper (ComposingManager) ignores
                // transition.derivedDisplay and the async derivation loop
                // (CandidateUpdateCoordinator.scheduleDisplayDerivation)
                // handles the tone-marked form on Dispatchers.Default. See
                // `composing-state-boundary.md` §11.10. `mode`/`toggles`
                // stay on the signature for call-site parity + future
                // shared-core alignment (iOS uses them here).
                derivedDisplay = "",
            )
        return newState to transition
    }

    private fun exitToIdle(effects: List<ComposingTransition.Effect>): Pair<ComposingState, ComposingTransition> {
        val newState = ComposingState(phase = Phase.Idle, selectedCandidateIndex = -1)
        val transition =
            ComposingTransition(
                newPhase = Phase.Idle,
                newSelectedIndex = -1,
                effects = effects,
                derivedDisplay = "",
            )
        return newState to transition
    }

    private fun noopTransition(): Pair<ComposingState, ComposingTransition> =
        this to
            ComposingTransition(
                newPhase = phase,
                newSelectedIndex = selectedCandidateIndex,
                effects = emptyList(),
                derivedDisplay = "",
            )
}
