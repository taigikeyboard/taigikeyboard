import Foundation

/// Platform executor for NextWord prediction on iOS.
///
/// Delegates decision logic to `NextWordEngine`; interprets the returned
/// `Outcome.Effect`s against platform resources (Timer, SQLite service,
/// main-thread UI callbacks, generation counter).
///
/// **Public surface** preserved from the pre-split controller so call sites
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
        nextWordService: NextWordService = .shared,
    ) {
        self.settingsProvider = settingsProvider
        self.nextWordService = nextWordService
    }

    // MARK: - State

    private var persistedState: NextWordPersistedState = .initial
    private var contextTimeoutTimer: Timer?

    /// Exposed via `SelectionContextProvider` for autocomplete context boost.
    var lastSelectedWord: String? {
        persistedState.lastSelectedWord
    }

    /// Whether NextWord predictions are currently displayed.
    var isShowing: Bool {
        persistedState.isShowing
    }

    // MARK: - Public API

    /// Unified NextWord entry: validate → record association → update state → optionally predict.
    /// Called by: suggestion selection, Space (`triggerPrediction=false`), Enter (`requireRomanMode=true`).
    func process(text: String, roman: String, requireRomanMode: Bool = false, triggerPrediction: Bool = true) {
        apply(intent: .wordSelected(
            text: text,
            roman: roman,
            requireRomanMode: requireRomanMode,
            triggerPrediction: triggerPrediction,
        ))
    }

    /// Re-predict NextWord after backspace based on last remaining character.
    /// Intentionally does NOT record associations — backspace is not a word selection.
    func rePredictAfterBackspace(lastChar: String) {
        apply(intent: .backspace(lastChar: lastChar))
    }

    /// Full reset: clear all state and hide UI suggestions.
    /// Called by: backspace (empty document), textDidChange, sentence-end punctuation.
    func resetAndClearUI() {
        apply(intent: .resetFull)
    }

    /// Hide NextWord suggestions without clearing association state.
    /// Called by: digit input, new composing character (not hyphen).
    func clearDisplay() {
        apply(intent: .clearForNewComposing)
    }

    // MARK: - Intent Dispatch

    /// Lower a lifecycle event into an engine intent, apply the outcome.
    ///
    /// **Threading invariant** (inherited from pre-split controller, to be
    /// tightened in G9): call sites must be on the main thread. `Timer`
    /// fires on the main run-loop, `@MainActor handleQueryResult` stays on
    /// main; keyboard action handlers run on main. No synchronization on
    /// `persistedState` — the main-thread invariant is the contract.
    private func apply(intent: NextWordIntent) {
        let input = makeDecisionInput()
        let outcome = NextWordEngine.decide(intent: intent, state: persistedState, input: input)
        persistedState = outcome.newState
        for effect in outcome.effects {
            execute(effect)
        }
    }

    private func makeDecisionInput() -> NextWordDecisionInput {
        NextWordDecisionInput(nowMs: Self.currentTimestampMs, settings: currentEngineSettings())
    }

    /// Snapshot the settings fields the engine reads. Called per-intent AND
    /// again when a prediction query resolves, so user toggles made while a
    /// query is in-flight (e.g. POJ↔TL, Hanji swap) take effect on render.
    private func currentEngineSettings() -> NextWordEngineSettings {
        let current = settingsProvider.current
        return NextWordEngineSettings(
            inputMode: current.inputMode,
            isTranslateSwapped: current.isTranslateSwapped,
            isAssociationRecordingEnabled: current.isAssociationRecordingEnabled,
        )
    }

    // MARK: - Effect Interpreter

    private func execute(_ effect: NextWordOutcome.Effect) {
        switch effect {
        case let .rescheduleContextTimeout(after):
            startContextTimeoutTimer(after: after)
        case .cancelContextTimeout:
            stopContextTimeoutTimer()
        case let .recordAssociation(pair):
            recordAssociation(pair)
        case let .recordCompoundAssociations(pairs):
            recordCompoundAssociations(pairs)
        case let .queryPredictions(word, roman, generation):
            dispatchPredictionQuery(word: word, roman: roman, generation: generation)
        case let .clearPredictionsUI(generation):
            clearPredictionsUI(generation: generation)
        }
    }

    // MARK: - Service I/O

    private func recordAssociation(_ pair: NextWordAssociationPair) {
        Task { [nextWordService] in
            await nextWordService.recordAssociation(
                prev: pair.prev,
                prevTl: pair.prevTl,
                nextHanzi: pair.next,
                nextTl: pair.nextTl,
            )
        }
    }

    /// Loop associations sequentially to avoid races on the SQLite UNIQUE
    /// constraint that protects `(prev_word, next_word)`.
    private func recordCompoundAssociations(_ pairs: [NextWordAssociationPair]) {
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

    private func dispatchPredictionQuery(word: String, roman: String, generation: UInt64) {
        logger.debug("[TRIGGER] querying for word='\(word)' gen=\(generation)")

        Task { @MainActor [nextWordService] in
            let raw = await nextWordService.predict(word: word, roman: roman)
            handleQueryResult(raw: raw, generation: generation)
        }
    }

    /// Resolve an async prediction query. Compares the generation tagged at
    /// dispatch time against the current persisted generation; a mismatch
    /// means an invalidating intent fired while the query was in flight, so
    /// the result is dropped to avoid stale UI.
    @MainActor
    private func handleQueryResult(raw: [RawNextWordPrediction], generation: UInt64) {
        guard generation == persistedState.currentGeneration else {
            logger.debug("[TRIGGER] dropping stale result gen=\(generation) current=\(persistedState.currentGeneration)")
            return
        }

        let predictions = NextWordEngine.filterPredictions(raw, settings: currentEngineSettings())

        if predictions.isEmpty {
            persistedState.isShowing = false
            contextUpdater?.resetNextWordSuggestions()
        } else {
            contextUpdater?.setNextWordPredictions(predictions)
            persistedState.isShowing = true
            startContextTimeoutTimer(after: NextWordEngine.contextTimeoutSeconds)
        }
    }

    /// Clear is synchronous to match the pre-split controller's behavior:
    /// `clearDisplay` and `resetAndClearUI` always cleared without a main
    /// queue hop. Routing through `DispatchQueue.main.async` would open a
    /// race where a stale clear runs after a newer prediction query has
    /// already rendered fresh suggestions. Main-thread invariant documented
    /// on `apply(intent:)` keeps this safe; `generation` is informational
    /// for Kotlin/Rust ports that may need an async gate.
    private func clearPredictionsUI(generation _: UInt64) {
        contextUpdater?.resetNextWordSuggestions()
    }

    // MARK: - Timer

    private func startContextTimeoutTimer(after interval: TimeInterval) {
        stopContextTimeoutTimer()
        contextTimeoutTimer = Timer.scheduledTimer(
            withTimeInterval: interval,
            repeats: false,
        ) { [weak self] _ in
            self?.handleContextTimeout()
        }
    }

    private func stopContextTimeoutTimer() {
        contextTimeoutTimer?.invalidate()
        contextTimeoutTimer = nil
    }

    private func handleContextTimeout() {
        logger.debug("[TIMEOUT] Context timeout - resetting")
        apply(intent: .contextTimeoutFired)
    }

    // MARK: - Clock

    static var currentTimestampMs: Int64 {
        Int64(Date().timeIntervalSince1970 * 1000)
    }
}
