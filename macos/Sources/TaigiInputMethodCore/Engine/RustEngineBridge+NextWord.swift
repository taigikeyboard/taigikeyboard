// Next-word slice of the engine bridge: the learning intents macOS sends.

import Foundation

/// The next-word intents macOS uses, which is three of the engine's eight.
///
/// macOS learns but does not predict, so the whole read half of the slice —
/// `FilterPredictions`, `SetIsShowing` — has no caller here and is not
/// wrapped. `Backspace` is absent for the same
/// reason: it exists to re-issue a prediction query against the character left
/// behind, and it records nothing.
///
/// `ContextTimeoutFired` is absent too, and that one is worth stating plainly
/// because it means macOS runs no context timer. The timeout's job is to expire
/// a stale context so an old word stops seeding predictions; recording is
/// already fenced by a strict 10-second window inside the engine
/// (`engine/nextword/src/decide.rs` `should_record_association`), so with no predictions on screen
/// a fired timeout would change nothing an observer could see.
///
/// macOS acts on none of the answer's effects: the engine records the bigrams
/// it decides on itself (`docs/architecture/user-data-engine-roadmap.md` P9b),
/// and the rest are about a prediction UI macOS does not have.
extension RustEngineBridge {
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
    ) {
        var payload = Taigi_Engine_WordSelected()
        payload.text = text
        payload.roman = roman
        // `require_roman_mode` gates Enter-commits-raw-romanization paths, which
        // reach the engine as ordinary commits on macOS.
        payload.requireRomanMode = false
        payload.triggerPrediction = false
        payload.input = decisionInput(nowMs: nowMs)
        decide(
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
    ) {
        var payload = Taigi_Engine_UpdateLastSelectedWord()
        payload.text = text
        payload.roman = roman
        payload.input = decisionInput(nowMs: nowMs)
        decide(
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
    ) {
        var payload = Taigi_Engine_ResetFull()
        payload.input = decisionInput(nowMs: nowMs)
        decide(
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

    /// `appConfig` plus the field the decide table reads: the swap flag, which
    /// suppresses recording for raw-romanization commits (`decide.rs:86`).
    private static func nextwordConfig(_ settings: EngineSettings) -> Taigi_Engine_AppConfig {
        var config = appConfig(settings)
        config.isTranslateSwapped = settings.isTranslateSwapped
        return config
    }

    private static func decide(
        _ method: Taigi_Engine_NextWordRequest.OneOf_Method,
        op: String,
        settings: EngineSettings,
        generation: UInt64,
    ) {
        var request = Taigi_Engine_NextWordRequest()
        request.method = method
        guard let payload = roundtrip(
            payload: .nextword(request),
            op: op,
            generation: generation,
            config: nextwordConfig(settings),
        ) else {
            return
        }
        guard case let .nextword(response) = payload else {
            recordFailure(op: op, message: "expected a nextword payload, got \(payload)")
            return
        }
        guard case .decide? = response.result else {
            recordFailure(op: op, message: "expected a decide result")
            return
        }
    }
}
