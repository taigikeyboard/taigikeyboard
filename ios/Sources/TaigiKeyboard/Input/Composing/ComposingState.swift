import Foundation

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.

/// Platform-neutral composing-buffer state machine.
///
/// `rawInput` (original keystrokes, e.g. `"gua2"`) is the single source of
/// truth; `derivedDisplay(...)` recomputes the tone-marked form via
/// `ToneConverter`. The wrapper layer (`ComposingManager` on iOS) owns
/// `@Published` fan-out, delegate side effects, and settings reads — this
/// type stays Foundation-pure so Android can reuse it verbatim.
///
/// Every intent returns a `ComposingTransition` whose `effects` the wrapper
/// must execute in the given order. No hidden side channels.
struct ComposingState: Equatable {
    enum Phase: Equatable {
        case idle
        case composing(raw: String)
    }

    enum Intent: Equatable {
        /// Begin a new composition buffer with the given text (caret reset).
        case start(String)

        /// Append `char` to the buffer; if idle, falls through to `start`.
        case append(String)

        /// Append `"-"` — convenience alias for TPS / romanization hyphenation.
        case appendHyphen

        /// Replace the last raw-input character (TPS auto-correct). Intentionally
        /// preserves `selectedCandidateIndex` — unlike `append`, which resets it.
        case replaceLast(String)

        /// Delete one grapheme. Exits to idle when the buffer empties.
        case deleteBackward

        /// Commit the `derivedDisplay` form (tone-marked) to the document.
        case commitDerived

        /// Commit the literal `rawInput` (bypass conversion). Used for
        /// Enter-at-index-0 / English passthrough.
        case commitRaw

        /// Commit the supplied suggestion text to the document, atomically
        /// replacing the current preedit.
        case selectSuggestion(String)

        /// Commit the current preedit (if any) and insert an externally
        /// supplied text atomically in one document write. Used by
        /// non-Taigi input surfaces — emoji palette, clipboard paste —
        /// so composing state never leaks a silent finish-composing.
        /// Idle → behaves as a plain insert of `text`.
        case commitPreeditThenInsertExternal(String)

        /// Clear all state (e.g. keyboard teardown / mode switch).
        case reset
    }

    private(set) var phase: Phase = .idle

    /// `-1` in idle; `0` when a fresh composition begins; preserved across
    /// `replaceLast`. UI never writes this directly — transitions are the
    /// only mutation path (see `composing-state-boundary.md` §2.1).
    private(set) var selectedCandidateIndex: Int = -1

    /// Externally override `selectedCandidateIndex` (e.g. candidate-bar tap
    /// or keyboard-arrow gesture). Kept as an explicit mutator rather than
    /// a writable property so all state changes flow through one of two
    /// paths — `apply(_:mode:toneToggles:)` for buffer intents, this method
    /// for UI-driven selection — and there is no way to desync the pure
    /// state from the platform wrapper.
    mutating func setSelectedCandidateIndex(_ index: Int) {
        selectedCandidateIndex = index
    }

    var isComposing: Bool {
        if case .idle = phase { return false }
        return true
    }

    var rawInput: String {
        if case let .composing(raw) = phase { return raw }
        return ""
    }

    /// Derive the display form for the current `rawInput`. TPS symbols are
    /// already display-ready; POJ/TL route through `ToneConverter` with the
    /// supplied toggles.
    func derivedDisplay(mode: InputMode, toneToggles: ToneToggles) -> String {
        let raw = rawInput
        guard !raw.isEmpty else { return "" }
        if RustEngineBridge.containsTPS(raw) { return raw }
        // Match the pre-D9.4 `ToneConverter.convertToToneMarks` pipeline:
        // normalize tones via the engine, then post-process the nasal
        // marker case so `ⁿ` / `ᴺ` follow the preceding letter's case
        // (uppercase before → `ᴺ`, otherwise `ⁿ`).
        let toneMarked = RustEngineBridge.normalizeTone(raw, mode: mode, toggles: toneToggles)
        return ToneUtilities.adjustNasalMarkerCase(toneMarked)
    }

    /// Apply an intent and emit the resulting `Transition`. The receiver is
    /// mutated in place; the returned transition describes the effects the
    /// wrapper must execute against the platform adapter.
    @discardableResult
    mutating func apply(
        _ intent: Intent,
        mode: InputMode,
        toneToggles: ToneToggles,
    ) -> ComposingTransition {
        switch intent {
        case let .start(text):
            return enterComposing(raw: text, mode: mode, toneToggles: toneToggles)

        case let .append(char):
            if case .idle = phase {
                return enterComposing(raw: char, mode: mode, toneToggles: toneToggles)
            }
            return enterComposing(raw: rawInput + char, mode: mode, toneToggles: toneToggles)

        case .appendHyphen:
            return apply(.append("-"), mode: mode, toneToggles: toneToggles)

        case let .replaceLast(replacement):
            guard case .composing = phase, !rawInput.isEmpty else {
                return noopTransition()
            }
            let newRaw = String(rawInput.dropLast()) + replacement
            // Replacement preserves selectedCandidateIndex — it is a correction
            // on top of an in-progress selection, not a fresh composition step.
            phase = .composing(raw: newRaw)
            let display = derivedDisplay(mode: mode, toneToggles: toneToggles)
            return ComposingTransition(
                newPhase: phase,
                newSelectedIndex: selectedCandidateIndex,
                effects: [.updatePreedit(display), .performAutocomplete],
                derivedDisplay: display,
            )

        case .deleteBackward:
            guard case .composing = phase, !rawInput.isEmpty else {
                return noopTransition()
            }
            let newRaw = String(rawInput.dropLast())
            if newRaw.isEmpty {
                return exitToIdle(
                    effects: [
                        .clearPreeditWithoutCommit,
                        .resetAutocomplete,
                        .deleteBackwardFromDocument,
                    ],
                )
            }
            // Keep engine state in sync with the transition's newSelectedIndex
            // (Codex P2 r3106434856): without this, a later `replaceLast` would
            // resurrect a pre-delete external selection because the engine's
            // stored value still holds the stale index.
            phase = .composing(raw: newRaw)
            selectedCandidateIndex = 0
            let display = derivedDisplay(mode: mode, toneToggles: toneToggles)
            return ComposingTransition(
                newPhase: phase,
                newSelectedIndex: 0,
                effects: [.updatePreedit(display), .performAutocomplete],
                derivedDisplay: display,
            )

        case .commitDerived:
            guard case .composing = phase else { return noopTransition() }
            let display = derivedDisplay(mode: mode, toneToggles: toneToggles)
            guard !display.isEmpty else { return noopTransition() }
            return exitToIdle(
                effects: [
                    .commitTextReplacingPreedit(display),
                    .resetAutocomplete,
                    .resetAutocompleteContext,
                ],
            )

        case .commitRaw:
            guard case .composing = phase, !rawInput.isEmpty else {
                return noopTransition()
            }
            let text = rawInput
            return exitToIdle(
                effects: [
                    .commitTextReplacingPreedit(text),
                    .resetAutocomplete,
                    .resetAutocompleteContext,
                ],
            )

        case let .selectSuggestion(text):
            guard case .composing = phase else { return noopTransition() }
            return exitToIdle(
                effects: [
                    .commitTextReplacingPreedit(text),
                    .resetAutocomplete,
                    .resetAutocompleteContext,
                ],
            )

        case let .commitPreeditThenInsertExternal(externalText):
            guard !externalText.isEmpty else { return noopTransition() }
            if case .composing = phase {
                let derived = derivedDisplay(mode: mode, toneToggles: toneToggles)
                return exitToIdle(
                    effects: [
                        .commitTextReplacingPreedit(derived + externalText),
                        .resetAutocomplete,
                        .resetAutocompleteContext,
                    ],
                )
            }
            // Idle: behave as a plain insert. Reuse `commitTextReplacingPreedit`
            // because it already maps to `clearMarkedText + insertText` on iOS
            // and `commitText` on Android; both are no-op-on-empty-preedit safe.
            return ComposingTransition(
                newPhase: .idle,
                newSelectedIndex: -1,
                effects: [.commitTextReplacingPreedit(externalText)],
                derivedDisplay: "",
            )

        case .reset:
            if case .idle = phase { return noopTransition() }
            return exitToIdle(effects: [.clearPreeditWithoutCommit, .resetAutocomplete])
        }
    }

    // MARK: - Private Helpers

    /// Enter (or update) the composing phase with the given raw buffer. A
    /// fresh composition step resets `selectedCandidateIndex` to `0`.
    private mutating func enterComposing(
        raw: String,
        mode: InputMode,
        toneToggles: ToneToggles,
    ) -> ComposingTransition {
        phase = .composing(raw: raw)
        selectedCandidateIndex = 0
        let display = derivedDisplay(mode: mode, toneToggles: toneToggles)
        return ComposingTransition(
            newPhase: phase,
            newSelectedIndex: 0,
            effects: [.updatePreedit(display), .performAutocomplete],
            derivedDisplay: display,
        )
    }

    /// Transition to idle, zeroing the selected index invariant, with the
    /// caller-specified effects (ordering depends on the intent).
    private mutating func exitToIdle(effects: [ComposingTransition.Effect]) -> ComposingTransition {
        phase = .idle
        selectedCandidateIndex = -1
        return ComposingTransition(
            newPhase: .idle,
            newSelectedIndex: -1,
            effects: effects,
            derivedDisplay: "",
        )
    }

    private func noopTransition() -> ComposingTransition {
        // Noop paths are reachable only from guard-idle branches, so the
        // derived display is empty by invariant. Wrapper keeps its last
        // published `composingText` untouched when `effects` is empty.
        ComposingTransition(
            newPhase: phase,
            newSelectedIndex: selectedCandidateIndex,
            effects: [],
            derivedDisplay: "",
        )
    }
}
