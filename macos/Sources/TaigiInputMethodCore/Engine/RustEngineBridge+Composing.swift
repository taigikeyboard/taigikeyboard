// Composing slice of the engine bridge: the intents macOS sends and the
// decoding of what comes back.

import Foundation

/// The composing intents macOS uses. This is a subset of the engine's sixteen:
///
/// - `AppendHyphen` is skipped because it is a pure alias for `Append("-")`
///   (`engine/composing/src/transition.rs:63-69`), and a hyphen is an ordinary
///   character on a Mac keyboard rather than a dedicated key as it is on iOS.
/// - `ReplaceLast` is skipped because it exists for TPS auto-correct, and macOS
///   ships TL and POJ only.
/// - `CommitDerived`, `QueryState` and `ResetContinuous` have no caller here:
///   `Reset` already covers aborting a continuous composition
///   (`transition.rs:554`).
/// - `SetSelectedCandidateIndex` and `CommitContinuous` arrive with the
///   candidate window, which is a later slice.
extension RustEngineBridge {
    // MARK: - Composing

    /// Begins a fresh composition from `text`.
    static func composingStart(
        _ text: String,
        settings: EngineSettings,
        generation: UInt64
    ) -> ComposingTransition {
        var start = Taigi_Engine_Start()
        start.text = text
        return dispatchComposing(
            .start(start),
            op: "composingStart",
            generation: generation,
            config: appConfig(settings)
        )
    }

    /// Appends one typed character to the raw buffer.
    static func composingAppend(
        _ character: String,
        settings: EngineSettings,
        generation: UInt64
    ) -> ComposingTransition {
        var append = Taigi_Engine_Append()
        append.char = character
        return dispatchComposing(
            .append(append),
            op: "composingAppend",
            generation: generation,
            config: appConfig(settings)
        )
    }

    /// Drops the last character of the raw buffer.
    static func composingDeleteBackward(
        settings: EngineSettings,
        generation: UInt64
    ) -> ComposingTransition {
        dispatchComposing(
            .deleteBackward(Taigi_Engine_DeleteBackward()),
            op: "composingDeleteBackward",
            generation: generation,
            config: appConfig(settings)
        )
    }

    /// Commits the whole composition as the engine renders it. Under the
    /// continuous phase that is `Σ nailed.display_text + derived(pending)`, not
    /// the literal keystrokes (`transition.rs:443`) — use
    /// `composingSelectSuggestion` when the literal is what the user asked for.
    static func composingCommitRaw(
        settings: EngineSettings,
        generation: UInt64
    ) -> ComposingTransition {
        dispatchComposing(
            .commitRaw(Taigi_Engine_CommitRaw()),
            op: "composingCommitRaw",
            generation: generation,
            config: continuousAppConfig(settings)
        )
    }

    /// Commits `text` verbatim, keeping any nailed prefix in front of it.
    static func composingSelectSuggestion(
        _ text: String,
        settings: EngineSettings,
        generation: UInt64
    ) -> ComposingTransition {
        var select = Taigi_Engine_SelectSuggestion()
        select.text = text
        return dispatchComposing(
            .selectSuggestion(select),
            op: "composingSelectSuggestion",
            generation: generation,
            config: continuousAppConfig(settings)
        )
    }

    /// Finalizes the composition and appends `text` after it, as one engine
    /// step. Used for the space bar: committing and then inserting separately
    /// would be two host mutations for one keystroke.
    static func composingCommitPreeditThenInsertExternal(
        _ text: String,
        settings: EngineSettings,
        generation: UInt64
    ) -> ComposingTransition {
        var insert = Taigi_Engine_CommitPreeditThenInsertExternal()
        insert.text = text
        return dispatchComposing(
            .commitPreeditThenInsertExternal(insert),
            op: "composingCommitPreeditThenInsertExternal",
            generation: generation,
            config: continuousAppConfig(settings)
        )
    }

    /// Abandons the composition without writing anything to the document.
    static func composingReset(generation: UInt64) -> ComposingTransition {
        dispatchComposing(
            .reset(Taigi_Engine_Reset()),
            op: "composingReset",
            generation: generation,
            config: nil
        )
    }

    // MARK: - Continuous input

    /// Promotes an active composition into the continuous phase, where the
    /// engine segments the whole buffer instead of one syllable. Safe to send
    /// unconditionally: the engine no-ops on an empty buffer and on a
    /// composition that is already continuous (`transition.rs:496-502`), so the
    /// platform needs no eligibility rule of its own.
    static func composingEnterContinuous(
        settings: EngineSettings,
        generation: UInt64
    ) -> ComposingTransition {
        dispatchComposing(
            .enterContinuous(Taigi_Engine_EnterContinuous()),
            op: "composingEnterContinuous",
            generation: generation,
            config: appConfig(settings)
        )
    }

    /// Reads the candidates for the current continuous composition.
    ///
    /// Read-only, so it must be sent under the composition's existing
    /// generation: a bumped generation resets the engine before the query runs
    /// (`engine/composing/src/handle.rs:61-66`).
    ///
    /// The user-frequency, custom-dictionary and source-toggle carriers are
    /// deliberately left at their neutral defaults — the stores that fill them
    /// are later slices, and the engine reads the proto3 zero values as "rank
    /// without them" rather than as an error.
    static func composingFetchAtPos(
        settings: EngineSettings,
        generation: UInt64
    ) -> ContinuousFetchResult {
        var fetch = Taigi_Engine_FetchAtPos()
        fetch.position = 0
        fetch.literalRomanCandidateDisabled = !settings.isLiteralRomanCandidateEnabled

        guard let response = composingResponse(
            .fetchAtPos(fetch),
            op: "composingFetchAtPos",
            generation: generation,
            config: continuousAppConfig(settings)
        ) else {
            return .noop
        }
        let candidates: [ContinuousCandidate]? = response.hasContinuous
            ? response.continuous.candidates.map(decodeCandidate)
            : nil
        return ContinuousFetchResult(
            transition: decodeTransition(response),
            candidates: candidates,
            isBridgeFailure: false
        )
    }

    // MARK: - Dispatch

    private static func dispatchComposing(
        _ method: Taigi_Engine_ComposingRequest.OneOf_Method,
        op: String,
        generation: UInt64,
        config: Taigi_Engine_AppConfig?
    ) -> ComposingTransition {
        guard let response = composingResponse(
            method,
            op: op,
            generation: generation,
            config: config
        ) else {
            return .noop
        }
        return decodeTransition(response)
    }

    private static func composingResponse(
        _ method: Taigi_Engine_ComposingRequest.OneOf_Method,
        op: String,
        generation: UInt64,
        config: Taigi_Engine_AppConfig?
    ) -> Taigi_Engine_ComposingResponse? {
        var composing = Taigi_Engine_ComposingRequest()
        composing.method = method
        guard let payload = roundtrip(
            payload: .composing(composing),
            op: op,
            generation: generation,
            config: config
        ) else {
            return nil
        }
        guard case let .composing(response) = payload else {
            recordFailure(op: op, message: "expected a composing payload, got \(payload)")
            return nil
        }
        return response
    }

    // MARK: - Decoding

    /// Not `private` so the decoding can be driven directly by tests: the two
    /// next-word effects that carry a payload are emitted only by intents the
    /// candidate window owns, and their field mapping would otherwise go
    /// unchecked until that slice lands and mislearns the user's associations.
    static func decodeTransition(
        _ response: Taigi_Engine_ComposingResponse
    ) -> ComposingTransition {
        ComposingTransition(
            rawInput: response.preedit.rawInput,
            displayText: response.preedit.displayText,
            effects: response.effect.compactMap(decodeEffect),
            selectedCandidateIndex: Int(response.selectedCandidateIndex),
            isComposing: response.isComposing
        )
    }

    /// `nil` only for an effect whose `kind` the wire left unset, which the
    /// current engine never emits. Every kind it *does* emit has a case here —
    /// the exhaustive `switch` is what makes a newly added engine effect a
    /// compile error rather than a silently dropped instruction.
    static func decodeEffect(
        _ effect: Taigi_Engine_Effect
    ) -> ComposingTransition.Effect? {
        guard let kind = effect.kind else { return nil }
        switch kind {
        case let .updatePreedit(payload):
            return .updatePreedit(payload.display)
        case .clearPreeditWithoutCommit_p:
            return .clearPreeditWithoutCommit
        case let .commitTextReplacingPreedit(payload):
            return .commitTextReplacingPreedit(payload.text)
        case .deleteBackwardFromDocument:
            return .deleteBackwardFromDocument
        case .resetAutocomplete:
            return .resetAutocomplete
        case .performAutocomplete:
            return .performAutocomplete
        case .resetAutocompleteContext:
            return .resetAutocompleteContext
        case let .nextWordUpdateLastSelectedWord(payload):
            return .nextWordUpdateLastSelectedWord(text: payload.text, roman: payload.roman)
        case let .nextWordWordSelected(payload):
            return .nextWordWordSelected(
                text: payload.text,
                roman: payload.roman,
                triggerPrediction: payload.triggerPrediction
            )
        case .nextWordClearForNewComposing:
            return .nextWordClearForNewComposing
        }
    }

    private static func decodeCandidate(
        _ message: Taigi_Engine_CandidateMessage
    ) -> ContinuousCandidate {
        ContinuousCandidate(
            consumedSpanStart: message.consumedSpanStart,
            consumedSpanEnd: message.consumedSpanEnd,
            syllableCount: message.syllableCount,
            displayText: message.displayText,
            score: message.score,
            form: message.form,
            mode: CandidateMode.decode(message.mode.rawValue),
            roman: message.roman,
            // Absent means "romanization-only candidate", which an empty string
            // would not distinguish from a present-but-blank hanji.
            hanji: message.hasHanji ? message.hanji : nil,
            canonicalTl: message.canonicalTl
        )
    }
}
