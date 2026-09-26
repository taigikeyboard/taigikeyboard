import Foundation

/// Platform executor for NextWord prediction on iOS — post-v3.5.5 Rust slice.
///
/// Decision logic + persisted state moved into `engine/nextword/` (Rust); this
/// controller is the iOS-side platform executor:
/// - serializes intents through `RustEngineBridge.nextword*`,
/// - interprets the returned `NextWordDecideResult.Effect` list against
///   platform resources (Timer, main-thread UI callbacks),
/// - caches `isShowing` echoed back from the engine for
///   sync read access by `ActionHandler`,
/// - pushes UI visibility back into the engine via `nextwordSetIsShowing`
///   after async predict() results render.
///
/// **Public surface** preserved from the pre-Rust controller so call sites
/// (`ActionHandler`, `KeyboardViewController`) do not change:
/// - `process(text:roman:requireRomanMode:triggerPrediction:)`
/// - `rePredictAfterBackspace(lastChar:)`
/// - `resetAndClearUI()`
/// - `clearDisplay()`
/// - `updateLastSelectedWord(text:roman:)` (v3.5.8 Phase 4 mid-commit handshake)
/// - `isShowing` (read-only)
final class NextWordController {
    let logger = DebugLogger(category: "NextWord")

    // MARK: - Dependencies

    private let settingsProvider: EngineSettingsProvider
    weak var contextUpdater: AutocompleteContextUpdater?

    init(settingsProvider: EngineSettingsProvider = SharedSettings.shared) {
        self.settingsProvider = settingsProvider
    }

    // MARK: - Cached state (echoed from Rust)

    /// Mirrors `state.is_showing`. Set locally by `handleQueryResult` after
    /// rendering, then pushed to the engine via `nextwordSetIsShowing` so
    /// downstream clear / reset paths gate `clearPredictionsUI` correctly.
    private var cachedIsShowing: Bool = false

    private var contextTimeoutTimer: Timer?

    /// Per-IME-session envelope generation. Engine `EngineHandle` resets state
    /// on mismatch BEFORE applying the request (composing-slice precedent —
    /// see `ComposingManager.bumpGeneration`). Called by
    /// `KeyboardViewController` lifecycle hooks on real input-context
    /// changes.
    private var envelopeGen: UInt64 = 1

    /// Whether NextWord predictions are currently displayed.
    var isShowing: Bool {
        cachedIsShowing
    }

    func bumpEnvelopeGeneration() {
        envelopeGen &+= 1
        // Cross-field IME-session boundary. Rust engine state will be wiped
        // on the next bridge call (envelope mismatch sets is_showing=false
        // before the request processes), so a follow-up ResetFull /
        // ClearForNewComposing cannot emit ClearPredictionsUI through the
        // engine's was_showing gate. Force-clear platform-side cached state
        // + UI here so cross-field stale suggestions don't linger.
        // Codex post-impl PR #198 r3171935009.
        stopContextTimeoutTimer()
        if cachedIsShowing {
            contextUpdater?.resetNextWordSuggestions()
        }
        cachedIsShowing = false
    }

    // MARK: - Public API (preserved from pre-Rust controller)

    func process(text: String, roman: String, requireRomanMode: Bool = false, triggerPrediction: Bool = true) {
        let settings = settingsProvider.current
        let result = RustEngineBridge.nextwordWordSelected(
            text: text,
            roman: roman,
            requireRomanMode: requireRomanMode,
            triggerPrediction: triggerPrediction,
            nowMs: Self.currentTimestampMs,
            mode: settings.inputMode,
            translateSwapped: settings.isTranslateSwapped,
            generation: envelopeGen,
        )
        applyDecideResult(result)
    }

    func rePredictAfterBackspace(lastChar: String) {
        let settings = settingsProvider.current
        let result = RustEngineBridge.nextwordBackspace(
            lastChar: lastChar,
            nowMs: Self.currentTimestampMs,
            mode: settings.inputMode,
            translateSwapped: settings.isTranslateSwapped,
            generation: envelopeGen,
        )
        applyDecideResult(result)
    }

    func resetAndClearUI() {
        let settings = settingsProvider.current
        let result = RustEngineBridge.nextwordResetFull(
            nowMs: Self.currentTimestampMs,
            mode: settings.inputMode,
            translateSwapped: settings.isTranslateSwapped,
            generation: envelopeGen,
        )
        applyDecideResult(result)
    }

    // Clears the shown predictions but keeps last_selected_word for the next boost.
    func clearDisplay() {
        let settings = settingsProvider.current
        let result = RustEngineBridge.nextwordClearForNewComposing(
            nowMs: Self.currentTimestampMs,
            mode: settings.inputMode,
            translateSwapped: settings.isTranslateSwapped,
            generation: envelopeGen,
        )
        applyDecideResult(result)
    }

    /// v3.5.8 Phase 4 — continuous-input mid-commit handshake. Emitted by
    /// the composing engine via `Effect.nextWordUpdateLastSelectedWord`
    /// when a `Phase::Continuous` mid-commit lands a segment. Updates
    /// `state.last_selected_word` + `last_selection_time_ms` without
    /// bumping `current_generation`, no timer effects.
    ///
    /// Distinct from `process(...)`: a mid-commit segment is not a
    /// "user selected this word" event — `WordSelected` would record a
    /// `prev → this` association and (optionally) trigger prediction;
    /// `UpdateLastSelectedWord` only updates the context for the *next*
    /// mid-commit's compound association.
    func updateLastSelectedWord(text: String, roman: String) {
        let settings = settingsProvider.current
        let result = RustEngineBridge.nextwordUpdateLastSelectedWord(
            text: text,
            roman: roman,
            nowMs: Self.currentTimestampMs,
            mode: settings.inputMode,
            translateSwapped: settings.isTranslateSwapped,
            generation: envelopeGen,
        )
        applyDecideResult(result)
    }

    // MARK: - Effect interpretation

    /// Mirror engine state echo, then run effects in the order the engine emitted.
    ///
    /// **Threading invariant** (inherited from pre-split controller): call
    /// sites must be on the main thread. `Timer` fires on the main run-loop,
    /// `@MainActor handleQueryResult` stays on main, keyboard action handlers
    /// run on main. No synchronization on cached state — the main-thread
    /// invariant is the contract.
    private func applyDecideResult(_ result: RustEngineBridge.NextWordDecideResult) {
        cachedIsShowing = result.isShowing
        for effect in result.effects {
            execute(effect)
        }
    }

    private func execute(_ effect: RustEngineBridge.NextWordDecideResult.Effect) {
        switch effect {
        case let .rescheduleContextTimeout(afterMs):
            startContextTimeoutTimer(afterMs: afterMs)
        case .cancelContextTimeout:
            stopContextTimeoutTimer()
        // The engine wrote the bigrams into `user_association.db` itself and
        // leaves these out of its answer once the user data is open (roadmap
        // P3c / P7b); the cases stay until the effects are retired (U9, P9).
        case .recordAssociation, .recordCompoundAssociations:
            break
        case let .queryPredictions(word, roman, generation, nowMs):
            dispatchPredictionQuery(word: word, roman: roman, generation: generation, nowMs: nowMs)
        case .clearPredictionsUI:
            clearPredictionsUIEffect()
        }
    }

    // MARK: - Predictions

    private func dispatchPredictionQuery(word: String, roman: String, generation: UInt64, nowMs: Int64) {
        logger.debug("[TRIGGER] querying for word='\(word)' gen=\(generation)")

        // One settings snapshot at query start — the bundled lookup and the
        // rendering answer for the settings the query began under.
        let settings = settingsProvider.current
        let toggles = RustEngineBridge.DictionaryToggles(from: settings)
        // A `@MainActor` hop so the render lands after the effect loop that
        // queued it, as it always has.
        Task { @MainActor in
            let envelope = envelopeGen
            // Off the main thread: the engine reads the learned rows from its
            // own `user_association.db` inside this call (roadmap P7b).
            let filterResult = await Task.detached {
                RustEngineBridge.nextwordPredictNext(
                    word: word,
                    roman: roman,
                    toggles: toggles,
                    queryGeneration: generation,
                    nowMs: nowMs,
                    limit: 30,
                    mode: settings.inputMode,
                    translateSwapped: settings.isTranslateSwapped,
                    candidateDisplayMode: settings.candidateDisplayMode,
                    hyphenlessRoman: settings.isHyphenlessRomanEnabled,
                    generation: envelope,
                )
            }.value
            // A context change meanwhile makes this answer another context's.
            guard envelope == envelopeGen else { return }
            handleQueryResult(filterResult, queryGeneration: generation, settings: settings)
        }
    }

    /// Render an async prediction query's answer — `nextwordPredictNext` read
    /// the learned rows and added the bundled rows for the word, then merged +
    /// scored + sorted + truncated + dropped on stale generation — then push
    /// the new `is_showing` value back into engine state via
    /// `nextwordSetIsShowing` — required so subsequent
    /// `ClearForNewComposing` / sentence-end / context-timeout / `ResetFull`
    /// paths can emit `clearPredictionsUI` when there is UI to clear.
    @MainActor
    private func handleQueryResult(
        _ filterResult: RustEngineBridge.NextWordFilterResult,
        queryGeneration: UInt64,
        settings: EngineSettings,
    ) {
        if filterResult.wasStale {
            logger.debug("[TRIGGER] dropping stale result gen=\(queryGeneration)")
            return
        }

        let nowShowing = !filterResult.predictions.isEmpty
        if nowShowing {
            contextUpdater?.setNextWordPredictions(filterResult.predictions)
            startContextTimeoutTimer(afterMs: Self.contextTimeoutMs)
        } else {
            contextUpdater?.resetNextWordSuggestions()
        }

        // Push the rendered visibility back into engine state.
        let synced = RustEngineBridge.nextwordSetIsShowing(
            nowShowing,
            mode: settings.inputMode,
            translateSwapped: settings.isTranslateSwapped,
            generation: envelopeGen,
        )
        cachedIsShowing = synced.isShowing
    }

    /// Clear is synchronous to match the pre-Rust controller's behavior:
    /// `clearDisplay` / `resetAndClearUI` always cleared without a main-queue
    /// hop. Routing through `DispatchQueue.main.async` would open a race
    /// where a stale clear runs after a newer prediction query has rendered.
    /// `generation` from the effect is informational; main-thread invariant
    /// (above) keeps this safe.
    private func clearPredictionsUIEffect() {
        contextUpdater?.resetNextWordSuggestions()
        cachedIsShowing = false
    }

    // MARK: - Timer

    /// Mirrors `engine/nextword/src/decide.rs` `CONTEXT_TIMEOUT_MS = 30_000`.
    /// CROSS-PLATFORM INVARIANT: changing this value requires a paired update
    /// in the Rust crate + an `INVARIANT_*` parity-test mirror.
    static let contextTimeoutMs: UInt64 = 30000

    private func startContextTimeoutTimer(afterMs: UInt64) {
        stopContextTimeoutTimer()
        let interval = TimeInterval(afterMs) / 1000.0
        let captured = TraceContext.current
        contextTimeoutTimer = Timer.scheduledTimer(
            withTimeInterval: interval,
            repeats: false,
        ) { [weak self] _ in
            TraceContext.with(captured ?? TraceId.untraced) {
                self?.handleContextTimeout()
            }
        }
    }

    private func stopContextTimeoutTimer() {
        contextTimeoutTimer?.invalidate()
        contextTimeoutTimer = nil
    }

    private func handleContextTimeout() {
        logger.debug("[TIMEOUT] Context timeout - resetting")
        let settings = settingsProvider.current
        let result = RustEngineBridge.nextwordContextTimeoutFired(
            nowMs: Self.currentTimestampMs,
            mode: settings.inputMode,
            translateSwapped: settings.isTranslateSwapped,
            generation: envelopeGen,
        )
        applyDecideResult(result)
    }

    // MARK: - Clock

    static var currentTimestampMs: Int64 {
        Int64(Date().timeIntervalSince1970 * 1000)
    }
}
