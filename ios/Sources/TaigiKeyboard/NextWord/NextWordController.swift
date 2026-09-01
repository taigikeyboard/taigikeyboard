// 中文: NextWord 平台執行器 — v3.5.5 後決策邏輯與持久狀態都搬到 Rust。
// 中文: 本檔負責序列化 intents 給 RustEngineBridge.nextword*,並把回傳的 Effect
// 中文: 翻譯成 Timer / SQLite / 主執行緒 UI 動作。

import Foundation

/// Platform executor for NextWord prediction on iOS — post-v3.5.5 Rust slice.
///
/// Decision logic + persisted state moved into `engine/nextword/` (Rust); this
/// controller is the iOS-side platform executor:
/// - serializes intents through `RustEngineBridge.nextword*`,
/// - interprets the returned `NextWordDecideResult.Effect` list against
///   platform resources (Timer, SQLite service, main-thread UI callbacks),
/// - caches `lastSelectedWord` / `isShowing` echoed back from the engine for
///   sync read access by `ActionHandler` / `TaigiAutocompleteService`,
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
/// - `isShowing`, `lastSelectedWord` (read-only)
// 中文: 對外 API 與 Rust 化前完全相同(Phase 4 加 updateLastSelectedWord 給連續輸入 mid-commit 用)。
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
    // 中文: 鏡射引擎回傳的 last_selected_word,給 SelectionContextProvider 同步讀取。
    private var cachedLastSelectedWord: String?

    /// Mirrors `state.is_showing`. Set locally by `handleQueryResult` after
    /// rendering, then pushed to the engine via `nextwordSetIsShowing` so
    /// downstream clear / reset paths gate `clearPredictionsUI` correctly.
    // 中文: 鏡射 is_showing,讓後續的 clear / reset 能正確判斷是否要發 clearPredictionsUI。
    private var cachedIsShowing: Bool = false

    private var contextTimeoutTimer: Timer?

    /// Per-IME-session envelope generation. Engine `EngineHandle` resets state
    /// on mismatch BEFORE applying the request (composing-slice precedent —
    /// see `ComposingManager.bumpGeneration`). Called by
    /// `KeyboardViewController` lifecycle hooks on real input-context
    /// changes.
    // 中文: 每個 IME session 用獨立的 envelope generation。引擎側 generation 不符
    // 中文: 時會先重置狀態再處理請求,參考 ComposingManager.bumpGeneration 規範。
    private var envelopeGen: UInt64 = 1

    /// `SelectionContextProvider` conformance. The autocomplete context-boost
    /// consumer was retired in v3.5.8 Item 13; the property still mirrors the
    /// engine's last-selected word for the NextWord pipeline.
    // 中文: SelectionContextProvider 屬性;autocomplete context-boost consumer 已於
    // 中文: Item 13 退役,此值仍鏡射引擎 last-selected word 供 NextWord 用。
    var lastSelectedWord: String? {
        cachedLastSelectedWord
    }

    /// Whether NextWord predictions are currently displayed.
    // 中文: 目前 NextWord 預測是否顯示中。
    var isShowing: Bool {
        cachedIsShowing
    }

    /// `SelectionContextProvider` conformance. Its autocomplete consumer was
    /// retired in v3.5.8 Item 13; `envelopeGen` is still owned and used by
    /// the NextWord pipeline itself.
    // 中文: SelectionContextProvider 屬性;autocomplete consumer 已 Item 13 退役,
    // 中文: envelopeGen 仍由 NextWord pipeline 自身擁有與使用。
    var nextwordEnvelopeGeneration: UInt64 {
        envelopeGen
    }

    // 中文: 跨欄位切換 IME session 時呼叫 — 推進 generation 並強制清除快取與 UI,
    // 中文: 避免引擎 was_showing gate 已被 envelope 重置而吞掉 ClearPredictionsUI。
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
        cachedLastSelectedWord = nil
    }

    // MARK: - Public API (preserved from pre-Rust controller)

    // 中文: 使用者選詞後的主入口 — 把字串、模式、generation 都送給 Rust decide。
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

    // 中文: 倒退鍵之後重新預測 — 把最後一個字交回 Rust 做 backspace decide。
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

    // 中文: 完整重置 — 清空 NextWord 狀態與 UI(例如 IME session 結束)。
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

    // 中文: 開始新組字時清掉舊預測,但保留 last_selected_word 給後續 boost。
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
    // 中文: Phase 4 連續輸入 mid-commit handshake。只更新 last_selected_word /
    // 中文: time,不 bump generation、不發 timer effects;與 process(...) 語意不同 —
    // 中文: 後者會記錄 prev→this 關聯並可觸發預測,本方法只更新上下文。
    func updateLastSelectedWord(text: String, roman: String) {
        let settings = settingsProvider.current
        let result = RustEngineBridge.nextwordUpdateLastSelectedWord(
            text: text,
            roman: roman,
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
    // 中文: 鏡射引擎回傳的 state,然後依 effects 順序逐一執行。
    // 中文: 執行緒不變式:呼叫端必須在主執行緒,快取狀態不另外加鎖。
    private func applyDecideResult(_ result: RustEngineBridge.NextWordDecideResult) {
        cachedLastSelectedWord = result.lastSelectedWord
        cachedIsShowing = result.isShowing
        for effect in result.effects {
            execute(effect)
        }
    }

    // 中文: 把單一 Effect 分派到對應的平台動作(timer / SQLite / UI)。
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

    // 中文: 把單一 (prev, next) 關聯非同步寫進 user_association.db。
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
    // 中文: 多筆關聯依序寫入,避免 SQLite UNIQUE(prev_word, next_word) 競爭。
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

    // 中文: 觸發非同步預測查詢 — 走 NextWordService.predict,結果交給 handleQueryResult。
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
    // 中文: 處理非同步預測結果 — 把原始 rows 餵給 Rust filter 做 score / merge / sort,
    // 中文: 渲染後再用 nextwordSetIsShowing 把可見狀態同步回引擎。
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
            candidateDisplayMode: settings.candidateDisplayMode,
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
    // 中文: 清除預測 UI 同步執行,避免 main.async 跳脫造成過期 clear 蓋掉新結果。
    private func clearPredictionsUIEffect() {
        contextUpdater?.resetNextWordSuggestions()
        cachedIsShowing = false
    }

    // MARK: - Timer

    /// Mirrors `engine/nextword/src/decide.rs` `CONTEXT_TIMEOUT_MS = 30_000`.
    /// CROSS-PLATFORM INVARIANT: changing this value requires a paired update
    /// in the Rust crate + an `INVARIANT_*` parity-test mirror.
    // 中文: 與 Rust decide.rs 對齊的 30 秒 context timeout。修改需同步更新 Rust 與 parity test。
    static let contextTimeoutMs: UInt64 = 30000

    // 中文: 啟動 context timeout 計時器 — 帶上 trace id 以便日誌串接。
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

    // 中文: timer 觸發後通知 Rust 引擎做 context timeout 決策,通常會清掉 last_selected_word。
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

    // 中文: 統一的時鐘來源 — 給 Rust decide / filter 用的 epoch 毫秒。
    static var currentTimestampMs: Int64 {
        Int64(Date().timeIntervalSince1970 * 1000)
    }
}
