import Foundation

/// Platform executor for NextWord prediction on iOS — post-v3.5.5 Rust slice.
///
/// Decision logic + persisted state moved into `engine/nextword/` (Rust); this
/// controller is the iOS-side platform executor:
/// - serializes intents through `RustEngineBridge.nextword*`,
/// - interprets the returned `NextWordDecideResult.Effect` list against
///   platform resources (Timer, main-thread UI callbacks),
/// - caches `predictionsVisible` echoed back from the engine for
///   sync read access by `ActionHandler`,
/// - pushes UI visibility back into the engine via `nextwordSetPredictionsVisible`
///   after async predict() results render.
///
/// **Public surface** preserved from the pre-Rust controller so call sites
/// (`ActionHandler`, `KeyboardViewController`) do not change:
/// - `process(text:roman:requireRomanMode:triggerPrediction:)`
/// - `rePredictAfterBackspace(lastChar:)`
/// - `resetAndClearUI()`
/// - `clearDisplay()`
/// - `requestContext()` / `applyComposingDecision(_:)` (the composing engine applies its
///   next-word handshakes in its own call under this envelope; the controller runs the answer)
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

    /// Mirrors `state.predictions_visible`. Set locally by `handleQueryResult` after
    /// rendering, then pushed to the engine via `nextwordSetPredictionsVisible` so
    /// downstream clear / reset paths gate `clearPredictionsUI` correctly.
    private var cachedPredictionsVisible: Bool = false

    private var contextTimeoutTimer: Timer?

    /// Per-IME-session envelope generation. Engine `EngineHandle` resets state
    /// on mismatch BEFORE applying the request (composing-slice precedent —
    /// see `ComposingManager.bumpGeneration`). Called by
    /// `KeyboardViewController` lifecycle hooks on real input-context
    /// changes.
    private var envelopeGen: UInt64 = 1

    /// Whether NextWord predictions are currently displayed.
    var isShowing: Bool {
        cachedPredictionsVisible
    }

    func bumpEnvelopeGeneration() {
        envelopeGen &+= 1
        // Cross-field IME-session boundary. Rust engine state will be wiped
        // on the next bridge call (envelope mismatch sets predictions_visible=false
        // before the request processes), so a follow-up ResetAll /
        // ClearForNewComposing cannot emit ClearPredictionsUI through the
        // engine's was_visible gate. Force-clear platform-side cached state
        // + UI here so cross-field stale suggestions don't linger.
        // Codex post-impl PR #198 r3171935009.
        stopContextTimeoutTimer()
        if cachedPredictionsVisible {
            contextUpdater?.resetNextWordSuggestions()
        }
        cachedPredictionsVisible = false
    }

    /// Keyboard dismissed: stops the context Timer and drops in-flight prediction answers, so
    /// this controller — which UIKit may keep until its host app returns — does not fire
    /// `ContextTimeoutFired` into the process-shared engine later. No UI reset: the keyboard is gone.
    func tearDown() {
        envelopeGen &+= 1
        stopContextTimeoutTimer()
        cachedPredictionsVisible = false
    }

    // MARK: - Public API (preserved from pre-Rust controller)

    func process(
        text: String,
        roman: String,
        requireRomanMode: Bool = false,
        triggerPrediction: Bool = true,
    ) {
        let settings = settingsProvider.current
        let result = RustEngineBridge.nextwordWordSelected(
            text: text,
            roman: roman,
            requireRomanMode: requireRomanMode,
            triggerPrediction: triggerPrediction,
            nowMs: Self.currentTimestampMs,
            mode: settings.inputMode,
            hanjiFirst: settings.isHanjiFirst,
            generation: envelopeGen,
        )
        applyDecideResult(result)
    }

    /// Whether a character committed outside a composition is reported to the
    /// next-word engine as context. Letters are excluded because a letter
    /// starts a composition rather than reaching the host on its own;
    /// whitespace because it can never be sentence-end punctuation and would
    /// cost an engine round-trip per space bar press. Whether the character
    /// ends the sentence or is noise is the engine's call
    /// (`engine/nextword/src/decide.rs`).
    ///
    /// CROSS-PLATFORM INVARIANT — mirrors Android
    /// `isContextCharacterOutsideComposition` (`TextInputKeyHandler.kt`) and desktop-core
    /// `ComposingManager::note_character_typed_outside_composition`. Drift causes
    /// silent divergence.
    static func isContextCharacterOutsideComposition(_ character: String) -> Bool {
        !character.isEmpty && !character.contains(where: { $0.isLetter || $0.isWhitespace })
    }

    /// A character the user typed straight into the document, outside any
    /// composition (punctuation, a symbol). Forwarded as a commit with no
    /// reading and no prediction so the engine can end the context on
    /// sentence-end punctuation — what stops the last word of one sentence
    /// being learned as the predecessor of the first word of the next
    /// (`decide.rs` sentence-end rule). Mirrors desktop-core
    /// `ComposingManager::note_character_typed_outside_composition`.
    func noteCharacterTypedOutsideComposition(_ character: String) {
        process(text: character, roman: "", triggerPrediction: false)
    }

    func rePredictAfterBackspace(lastChar: String) {
        let settings = settingsProvider.current
        let result = RustEngineBridge.nextwordBackspace(
            lastChar: lastChar,
            nowMs: Self.currentTimestampMs,
            mode: settings.inputMode,
            hanjiFirst: settings.isHanjiFirst,
            generation: envelopeGen,
        )
        applyDecideResult(result)
    }

    func resetAndClearUI() {
        let settings = settingsProvider.current
        let result = RustEngineBridge.nextwordResetAll(
            nowMs: Self.currentTimestampMs,
            mode: settings.inputMode,
            hanjiFirst: settings.isHanjiFirst,
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
            hanjiFirst: settings.isHanjiFirst,
            generation: envelopeGen,
        )
        applyDecideResult(result)
    }

    /// The next-word envelope every composing request that can commit, nail or
    /// reset carries, so the engine applies the composition's next-word
    /// handshake in that same call (roadmap E2). One clock read per request.
    func requestContext() -> Taigi_Engine_NextWordContext {
        var context = Taigi_Engine_NextWordContext()
        context.generation = envelopeGen
        context.nowMs = Self.currentTimestampMs
        return context
    }

    /// Runs the decision a composing answer carries (`.nextWordDecided`) at the
    /// position the engine put it — a commit's learning and prediction query,
    /// or an abort's clear.
    func applyComposingDecision(_ result: RustEngineBridge.NextWordDecideResult) {
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
        cachedPredictionsVisible = result.predictionsVisible
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
        // Read before the hop, so a `tearDown()` or field switch that lands first drops the
        // query instead of sending it under the newer envelope.
        let envelope = envelopeGen
        // A `@MainActor` hop so the render lands after the effect loop that
        // queued it, as it always has.
        Task { @MainActor in
            guard envelope == envelopeGen else { return }
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
                    hanjiFirst: settings.isHanjiFirst,
                    candidateDisplayMode: settings.candidateDisplayMode,
                    syllableSeparator: settings.syllableSeparator,
                    isKhinsiannInHanjiEnabled: settings.isKhinsiannInHanjiEnabled,
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
    /// the new `predictions_visible` value back into engine state via
    /// `nextwordSetPredictionsVisible` — required so subsequent
    /// `ClearForNewComposing` / sentence-end / context-timeout / `ResetAll`
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
        let synced = RustEngineBridge.nextwordSetPredictionsVisible(
            nowShowing,
            mode: settings.inputMode,
            hanjiFirst: settings.isHanjiFirst,
            generation: envelopeGen,
        )
        cachedPredictionsVisible = synced.predictionsVisible
    }

    /// Clear is synchronous to match the pre-Rust controller's behavior:
    /// `clearDisplay` / `resetAndClearUI` always cleared without a main-queue
    /// hop. Routing through `DispatchQueue.main.async` would open a race
    /// where a stale clear runs after a newer prediction query has rendered.
    /// The main-thread invariant (above) keeps this safe.
    private func clearPredictionsUIEffect() {
        contextUpdater?.resetNextWordSuggestions()
        cachedPredictionsVisible = false
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
            hanjiFirst: settings.isHanjiFirst,
            generation: envelopeGen,
        )
        applyDecideResult(result)
    }

    // MARK: - Clock

    static var currentTimestampMs: Int64 {
        Int64(Date().timeIntervalSince1970 * 1000)
    }
}
