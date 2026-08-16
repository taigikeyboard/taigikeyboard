// Next-word slice of the engine bridge: the learning intents macOS sends.

import Foundation

/// The next-word intents macOS uses, which is three of the engine's nine.
///
/// macOS learns but does not predict, so the whole read half of the slice —
/// `FilterPredictions`, `BoostCandidates`, `NextWordQueryState`, `SetIsShowing`
/// — has no caller here and is not wrapped. `Backspace` is absent for the same
/// reason: it exists to re-issue a prediction query against the character left
/// behind, and it records nothing.
///
/// `ContextTimeoutFired` is absent too, and that one is worth stating plainly
/// because it means macOS runs no context timer. The timeout's job is to expire
/// a stale context so an old word stops seeding predictions; recording is
/// already fenced by a strict 10-second window inside the engine
/// (`engine/nextword/src/decide.rs:306-312`), so with no predictions on screen
/// a fired timeout would change nothing an observer could see. The two timer
/// effects are decoded and ignored rather than dropped silently.
extension RustEngineBridge {
    /// What one learning intent asked the platform to write.
    ///
    /// Only the recording effects are represented. The engine's other four are
    /// about a prediction UI macOS does not have, and mapping them into cases
    /// nothing switches on would suggest otherwise.
    struct NextWordOutcome: Equatable {
        enum Effect: Equatable {
            case recordAssociation(AssociationPair)
            case recordCompoundAssociations([AssociationPair])
        }

        let effects: [Effect]
    }

    /// The user committed `text`, read as `roman`.
    ///
    /// `trigger_prediction` is forced to `false` rather than forwarded from the
    /// composing effect that produced this call. The flag only decides whether
    /// the engine appends a `QueryPredictions` effect, and macOS has nothing to
    /// answer such a query with — forwarding it verbatim would cost a wire
    /// field per commit and a decode of an effect this platform then drops.
    /// Everything else the intent does — the association window, the state
    /// update, the generation bump — is unaffected by the flag
    /// (`engine/nextword/src/decide.rs:130-180`).
    static func nextwordWordSelected(
        text: String,
        roman: String,
        nowMs: Int64,
        settings: EngineSettings,
        generation: UInt64,
    ) -> NextWordOutcome? {
        var payload = Taigi_Engine_WordSelected()
        payload.text = text
        payload.roman = roman
        // `require_roman_mode` gates Enter-commits-raw-romanization paths, which
        // reach the engine as ordinary commits on macOS.
        payload.requireRomanMode = false
        payload.triggerPrediction = false
        payload.input = decisionInput(nowMs: nowMs)
        return decide(
            .wordSelected(payload),
            op: "nextwordWordSelected",
            settings: settings,
            generation: generation,
        )
    }

    /// A continuous composition nailed a segment mid-commit: the context moves
    /// on, but nothing has been finalized into the document yet. Records the
    /// compound bigrams only, and deliberately does not bump the engine's
    /// generation (`decide.rs:253-290`).
    static func nextwordUpdateLastSelectedWord(
        text: String,
        roman: String,
        nowMs: Int64,
        settings: EngineSettings,
        generation: UInt64,
    ) -> NextWordOutcome? {
        var payload = Taigi_Engine_UpdateLastSelectedWord()
        payload.text = text
        payload.roman = roman
        payload.input = decisionInput(nowMs: nowMs)
        return decide(
            .updateLastSelectedWord(payload),
            op: "nextwordUpdateLastSelectedWord",
            settings: settings,
            generation: generation,
        )
    }

    /// Forgets the current context outright. Sent when the composition session
    /// changes hands, so the last word typed in one application cannot be
    /// learned as the predecessor of the first word typed in the next.
    static func nextwordResetFull(
        nowMs: Int64,
        settings: EngineSettings,
        generation: UInt64,
    ) -> NextWordOutcome? {
        var payload = Taigi_Engine_ResetFull()
        payload.input = decisionInput(nowMs: nowMs)
        return decide(
            .resetFull(payload),
            op: "nextwordResetFull",
            settings: settings,
            generation: generation,
        )
    }

    // MARK: - Dispatch

    private static func decisionInput(nowMs: Int64) -> Taigi_Engine_DecisionInput {
        var input = Taigi_Engine_DecisionInput()
        input.nowMs = nowMs
        return input
    }

    /// `appConfig` plus the two fields the decide table reads: the swap flag
    /// (which suppresses recording for raw-romanization commits) and the
    /// recording switch itself, which is what the user's 記錄詞語關聯 setting
    /// turns off (`decide.rs:109`, `:130`).
    private static func nextwordConfig(_ settings: EngineSettings) -> Taigi_Engine_AppConfig {
        var config = appConfig(settings)
        config.isTranslateSwapped = settings.isTranslateSwapped
        config.isAssociationRecordingEnabled = settings.isAssociationRecordingEnabled
        return config
    }

    private static func decide(
        _ method: Taigi_Engine_NextWordRequest.OneOf_Method,
        op: String,
        settings: EngineSettings,
        generation: UInt64,
    ) -> NextWordOutcome? {
        var request = Taigi_Engine_NextWordRequest()
        request.method = method
        guard let payload = roundtrip(
            payload: .nextword(request),
            op: op,
            generation: generation,
            config: nextwordConfig(settings),
        ) else {
            return nil
        }
        guard case let .nextword(response) = payload else {
            recordFailure(op: op, message: "expected a nextword payload, got \(payload)")
            return nil
        }
        guard case let .decide(result)? = response.result else {
            recordFailure(op: op, message: "expected a decide result")
            return nil
        }
        return NextWordOutcome(effects: result.effects.compactMap(decodeEffect))
    }

    /// `nil` for an effect this platform has nothing to do with. The switch is
    /// exhaustive on purpose: a next-word effect added to the engine later has
    /// to be classified here rather than silently ignored.
    private static func decodeEffect(
        _ effect: Taigi_Engine_NextWordEffect,
    ) -> NextWordOutcome.Effect? {
        switch effect.kind {
        case let .recordAssociation(payload):
            .recordAssociation(decodePair(payload.pair))
        case let .recordCompoundAssociations(payload):
            .recordCompoundAssociations(payload.pairs.map(decodePair))
        case .rescheduleContextTimeout, .cancelContextTimeout:
            // macOS runs no context timer — see this file's header.
            nil
        case .queryPredictions, .clearPredictionsUi_p:
            // Neither is reachable: queries need `trigger_prediction`, which is
            // always false here, and a UI clear needs `is_showing`, which
            // nothing on macOS ever sets true.
            nil
        case .none:
            nil
        }
    }

    private static func decodePair(_ pair: Taigi_Engine_AssociationPair) -> AssociationPair {
        AssociationPair(
            previous: pair.prev,
            previousTl: pair.prevTl,
            next: pair.next,
            nextTl: pair.nextTl,
        )
    }
}
