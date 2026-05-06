import Foundation

/// Platform executor for NextWord prediction on iOS — post-v3.5.5 Rust slice.
///
/// Decision logic + persisted state moved into `engine/nextword/` (Rust); this
/// controller is the iOS-side platform executor:
/// - serializes intents through `RustEngineBridge.nextword*`,
/// - interprets the returned `NextWordDecideResult.Effect` list against
///   platform resources (Timer, SQLite service, main-thread UI callbacks),
/// - caches `lastSelectedWord` / `isShowing` echoed back from the engine for
///   sync read access by `ActionHandler` / `AutocompleteService`,
/// - pushes UI visibility back into the engine via `nextwordSetIsShowing`
///   after async predict() results render.
///
/// **Public surface** preserved from the pre-Rust controller so call sites
/// (`ActionHandler`, `KeyboardViewController`) do not change:
/// - `process(text:roman:requireRomanMode:triggerPrediction:)`
/// - `rePredictAfterBackspace(lastChar:)`
/// - `resetAndClearUI()`
/// - `clearDisplay()`
/// - `isShowing`, `lastSelectedWord` (read-only)
final class NextWordController: SelectionContextProvider {
    let logger = DebugLogger(category: "NextWord")

    // MARK: - Dependencies

    private let settingsProvider: EngineSettingsProvider
    private let nextWordService: NextWordService
    weak var contextUpdater: AutocompleteContextUpdater?

    init(
        settingsProvider: EngineSettingsProvider = SharedSettings.shared,
        nextWordService: NextWordService = CompositionRoot.nextWordService,
    ) {
        self.settingsProvider = settingsProvider
        self.nextWordService = nextWordService
    }

    // MARK: - Cached state (echoed from Rust)

    /// Mirrors `state.last_selected_word` returned by every decide call.
    /// Synchronous read for `SelectionContextProvider`.
    private var cachedLastSelectedWord: String?

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

    /// Exposed via `SelectionContextProvider` for autocomplete context boost.
    var lastSelectedWord: String? {
        cachedLastSelectedWord
    }

    /// Whether NextWord predictions are currently displayed.
    var isShowing: Bool {
        cachedIsShowing
    }

    /// Conform to the protocol so `AutocompleteService.nextwordBoostCandidates`
    /// shares the same envelope generation, avoiding spurious state resets.
    var nextwordEnvelopeGeneration: UInt64 {
        envelopeGen
    }

    public func bumpEnvelopeGeneration() {
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
        cachedLastSelectedWord = nil
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
            associationRecordingEnabled: settings.isAssociationRecordingEnabled,
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
            associationRecordingEnabled: settings.isAssociationRecordingEnabled,
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
            associationRecordingEnabled: settings.isAssociationRecordingEnabled,
            generation: envelopeGen,
        )
        applyDecideResult(result)
    }

    func clearDisplay() {
        let settings = settingsProvider.current
        let result = RustEngineBridge.nextwordClearForNewComposing(
            nowMs: Self.currentTimestampMs,
            mode: settings.inputMode,
            translateSwapped: settings.isTranslateSwapped,
            associationRecordingEnabled: settings.isAssociationRecordingEnabled,
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
        cachedLastSelectedWord = result.lastSelectedWord
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
        case let .recordAssociation(pair):
            recordAssociation(pair)
        case let .recordCompoundAssociations(pairs):
            recordCompoundAssociations(pairs)
        case let .queryPredictions(word, roman, generation, nowMs):
            dispatchPredictionQuery(word: word, roman: roman, generation: generation, nowMs: nowMs)
        case .clearPredictionsUI:
            clearPredictionsUIEffect()
        }
    }

    // MARK: - Service I/O

    private func recordAssociation(_ pair: RustEngineBridge.NextWordAssociationPair) {
        Task { [nextWordService] in
            await nextWordService.recordAssociation(
                prev: pair.prev,
                prevTl: pair.prevTl,
                nextHanzi: pair.next,
                nextTl: pair.nextTl,
            )
        }
    }

    /// Loop sequentially to avoid races on the SQLite UNIQUE constraint that
    /// protects `(prev_word, next_word)`.
    private func recordCompoundAssociations(_ pairs: [RustEngineBridge.NextWordAssociationPair]) {
        Task { [nextWordService] in
            for pair in pairs {
                await nextWordService.recordAssociation(
                    prev: pair.prev,
                    prevTl: pair.prevTl,
                    nextHanzi: pair.next,
                    nextTl: pair.nextTl,
                )
            }
        }
    }

    private func dispatchPredictionQuery(word: String, roman: String, generation: UInt64, nowMs: Int64) {
        logger.debug("[TRIGGER] querying for word='\(word)' gen=\(generation)")

        Task { @MainActor [nextWordService] in
            let raw = await nextWordService.predict(word: word, roman: roman)
            handleQueryResult(raw: raw, queryGeneration: generation, nowMs: nowMs)
        }
    }

    /// Resolve an async prediction query. Pushes raw rows back through
    /// `nextwordFilter` so the Rust engine merges + scores + sorts + truncates
    /// + drops on stale generation. Renders the resulting `NextWordEnginePrediction`s
    /// then pushes the new `is_showing` value back into engine state via
    /// `nextwordSetIsShowing` — required so subsequent
    /// `ClearForNewComposing` / sentence-end / context-timeout / `ResetFull`
    /// paths can emit `clearPredictionsUI` when there is UI to clear.
    @MainActor
    private func handleQueryResult(
        raw: [RustEngineBridge.NextWordRawRow],
        queryGeneration: UInt64,
        nowMs: Int64,
    ) {
        let settings = settingsProvider.current
        let filterResult = RustEngineBridge.nextwordFilter(
            raw: raw,
            queryGeneration: queryGeneration,
            nowMs: nowMs,
            limit: 30,
            mode: settings.inputMode,
            translateSwapped: settings.isTranslateSwapped,
            associationRecordingEnabled: settings.isAssociationRecordingEnabled,
            generation: envelopeGen,
        )
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
            associationRecordingEnabled: settings.isAssociationRecordingEnabled,
            generation: envelopeGen,
        )
        cachedIsShowing = synced.isShowing
        cachedLastSelectedWord = synced.lastSelectedWord
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
    static let contextTimeoutMs: UInt64 = 30_000

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
            associationRecordingEnabled: settings.isAssociationRecordingEnabled,
            generation: envelopeGen,
        )
        applyDecideResult(result)
    }

    // MARK: - Clock

    static var currentTimestampMs: Int64 {
        Int64(Date().timeIntervalSince1970 * 1000)
    }
}
