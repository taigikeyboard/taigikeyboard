// 中文: RustEngineBridge 的 NextWord 切片擴充(v3.5.5,v3.5.8 Phase 7A 補 updateLastSelectedWord)。
// 中文: 含 6 個 decide intent + filter / boost / queryState + setIsShowing。

import Foundation
import SwiftProtobuf

// MARK: - RustEngineBridge NextWord surface (v3.5.5)

/// NextWord slice extension for `RustEngineBridge`. Mirrors the pattern
/// established in v3.5.4 for the composing slice — proto roundtrip
/// helpers + Swift-friendly synthesized value types.
///
/// `nextwordUpdateLastSelectedWord` was an Android-only Space-path intent
/// pre-v3.5.8 (per `nextword-engine-boundary.md` §13 + `nextword-slice-audit.md`
/// §5 #5). v3.5.8 Phase 4 introduced a continuous-input mid-commit handshake
/// that emits the matching `Effect.nextWordUpdateLastSelectedWord` from the
/// composing engine; iOS now needs the wrapper to forward the effect through
/// `NextWordController.updateLastSelectedWord`.
// 中文: NextWord 切片的 bridge 擴充入口。
// 中文: v3.5.8 Phase 4 後 iOS 也需要 updateLastSelectedWord(連續輸入 mid-commit handshake)。
public extension RustEngineBridge {
    // MARK: - Synthesized value types

    /// Bridge-synthesized companion to the proto `DecideResult`. Consumed
    /// by `NextWordController`; effect-list executes in order on the iOS
    /// platform-executor side.
    // 中文: 對應 proto DecideResult 的 Swift struct,NextWordController 會依 effects 的順序執行。
    struct NextWordDecideResult: Equatable {
        public enum Effect: Equatable {
            case rescheduleContextTimeout(afterMs: UInt64)
            case cancelContextTimeout
            case recordAssociation(NextWordAssociationPair)
            case recordCompoundAssociations([NextWordAssociationPair])
            /// `nowMs` is reused by the platform's predict() call so the
            /// engine's association-window clock and the user-row decay
            /// scoring see ONE consistent "now" per intent. Per
            /// `nextword-engine-boundary.md` §13.3.
            case queryPredictions(word: String, roman: String, generation: UInt64, nowMs: Int64)
            case clearPredictionsUI(generation: UInt64)
        }

        public let effects: [Effect]
        public let currentGeneration: UInt64
        public let isShowing: Bool
        /// `nil` when the engine has no last-selected word; otherwise the
        /// echo of `state.last_selected_word`. Empty wire string maps to
        /// `nil` per proto contract.
        public let lastSelectedWord: String?

        public static let noop = NextWordDecideResult(
            effects: [],
            currentGeneration: 0,
            isShowing: false,
            lastSelectedWord: nil,
        )
    }

    /// Bigram association pair surfaced through `RecordAssociation` /
    /// `RecordCompoundAssociations` effects. Consumed by
    /// `NextWordService.recordAssociation`.
    // 中文: bigram 詞組關聯對,由 RecordAssociation / RecordCompoundAssociations effect 帶出。
    struct NextWordAssociationPair: Equatable {
        public let prev: String
        public let prevTl: String
        public let next: String
        public let nextTl: String
    }

    /// UI-ready prediction value. `subtitle` is `nil` when the wire
    /// string is empty (filter contract — happens iff roman is empty).
    // 中文: UI 直接可用的 NextWord 預測項。subtitle 在 wire 為空字串時轉成 nil(roman 為空才會發生)。
    struct NextWordEnginePrediction: Equatable {
        public let text: String
        public let subtitle: String?
        public let hanzi: String
        public let tl: String
        /// Merged score. iOS does not currently consume this field
        /// (predictions render in array order); Android maps to
        /// `TaigiWord.lengthScore`. Kept for parity + diagnostics.
        public let score: Double
    }

    /// Filter+merge+sort+limit result. `wasStale=true` indicates the
    /// platform-supplied `queryGeneration` did not match the engine's
    /// current generation — late async result; predictions are empty.
    // 中文: filter + merge + sort + limit 的結果。wasStale=true 表示 queryGeneration 不符,
    // 中文: 是來不及處理的舊 async 結果,predictions 必為空。
    struct NextWordFilterResult: Equatable {
        public let predictions: [NextWordEnginePrediction]
        public let wasStale: Bool
    }

    /// Pre-merge un-scored row from the platform `NextWordService.predict`
    /// SQL pipeline. Crosses the bridge to the Rust filter step.
    // 中文: NextWordService.predict 的 SQL pipeline 出來、尚未計分的 row,送進 Rust filter 計分用。
    struct NextWordRawRow: Equatable {
        public enum Source { case dict, user }

        public let hanzi: String
        public let tl: String
        public let count: Int64
        public let lastUsedMs: Int64
        public let source: Source

        public init(hanzi: String, tl: String, count: Int64, lastUsedMs: Int64, source: Source) {
            self.hanzi = hanzi
            self.tl = tl
            self.count = count
            self.lastUsedMs = lastUsedMs
            self.source = source
        }
    }

    /// Engine-state read for `SelectionContextProvider` / executor lookup.
    // 中文: 讀取 NextWord engine 當前狀態 — 給 SelectionContextProvider 與 executor 用。
    struct NextWordStateSnapshot: Equatable {
        public let lastSelectedWord: String?
        public let isShowing: Bool
        public let currentGeneration: UInt64
    }

    // MARK: - Decide intents (6)

    /// v3.5.8 Phase 4 — continuous-input mid-commit handshake. Updates
    /// `state.last_selected_word` + `last_selection_time_ms` without
    /// bumping `current_generation`, no timer effects, emits compound-only
    /// `RecordCompoundAssociations` effect. Pre-v3.5.8 this was Android-only;
    /// the Phase 4 effect-based handshake brought iOS into the call site.
    // 中文: 連續輸入 mid-commit 用 — 只更新 last_selected_word/time,不 bump generation,
    // 中文: 不發 timer effects,只發 RecordCompoundAssociations。Phase 4 後 iOS 也走這條路徑。
    static func nextwordUpdateLastSelectedWord(
        text: String,
        roman: String,
        nowMs: Int64,
        mode: InputMode,
        translateSwapped: Bool,
        associationRecordingEnabled: Bool,
        generation: UInt64,
    ) -> NextWordDecideResult {
        var payload = Taigi_Engine_UpdateLastSelectedWord()
        payload.text = text
        payload.roman = roman
        payload.input = decisionInput(nowMs: nowMs)
        return decideDispatch(
            method: .updateLastSelectedWord(payload),
            op: "nextwordUpdateLastSelectedWord",
            generation: generation,
            config: nextwordConfig(mode: mode, translateSwapped: translateSwapped, associationRecordingEnabled: associationRecordingEnabled),
        )
    }

    static func nextwordWordSelected(
        text: String,
        roman: String,
        requireRomanMode: Bool,
        triggerPrediction: Bool,
        nowMs: Int64,
        mode: InputMode,
        translateSwapped: Bool,
        associationRecordingEnabled: Bool,
        generation: UInt64,
    ) -> NextWordDecideResult {
        var payload = Taigi_Engine_WordSelected()
        payload.text = text
        payload.roman = roman
        payload.requireRomanMode = requireRomanMode
        payload.triggerPrediction = triggerPrediction
        payload.input = decisionInput(nowMs: nowMs)
        return decideDispatch(
            method: .wordSelected(payload),
            op: "nextwordWordSelected",
            generation: generation,
            config: nextwordConfig(mode: mode, translateSwapped: translateSwapped, associationRecordingEnabled: associationRecordingEnabled),
        )
    }

    static func nextwordBackspace(
        lastChar: String,
        nowMs: Int64,
        mode: InputMode,
        translateSwapped: Bool,
        associationRecordingEnabled: Bool,
        generation: UInt64,
    ) -> NextWordDecideResult {
        var payload = Taigi_Engine_Backspace()
        payload.lastChar = lastChar
        payload.input = decisionInput(nowMs: nowMs)
        return decideDispatch(
            method: .backspace(payload),
            op: "nextwordBackspace",
            generation: generation,
            config: nextwordConfig(mode: mode, translateSwapped: translateSwapped, associationRecordingEnabled: associationRecordingEnabled),
        )
    }

    static func nextwordContextTimeoutFired(
        nowMs: Int64,
        mode: InputMode,
        translateSwapped: Bool,
        associationRecordingEnabled: Bool,
        generation: UInt64,
    ) -> NextWordDecideResult {
        var payload = Taigi_Engine_ContextTimeoutFired()
        payload.input = decisionInput(nowMs: nowMs)
        return decideDispatch(
            method: .contextTimeoutFired(payload),
            op: "nextwordContextTimeoutFired",
            generation: generation,
            config: nextwordConfig(mode: mode, translateSwapped: translateSwapped, associationRecordingEnabled: associationRecordingEnabled),
        )
    }

    static func nextwordClearForNewComposing(
        nowMs: Int64,
        mode: InputMode,
        translateSwapped: Bool,
        associationRecordingEnabled: Bool,
        generation: UInt64,
    ) -> NextWordDecideResult {
        var payload = Taigi_Engine_ClearForNewComposing()
        payload.input = decisionInput(nowMs: nowMs)
        return decideDispatch(
            // SwiftProtobuf appends `_p` to disambiguate `clearForNewComposing`
            // from a generated property name; not a typo.
            method: .clearForNewComposing_p(payload),
            op: "nextwordClearForNewComposing",
            generation: generation,
            config: nextwordConfig(mode: mode, translateSwapped: translateSwapped, associationRecordingEnabled: associationRecordingEnabled),
        )
    }

    static func nextwordResetFull(
        nowMs: Int64,
        mode: InputMode,
        translateSwapped: Bool,
        associationRecordingEnabled: Bool,
        generation: UInt64,
    ) -> NextWordDecideResult {
        var payload = Taigi_Engine_ResetFull()
        payload.input = decisionInput(nowMs: nowMs)
        return decideDispatch(
            method: .resetFull(payload),
            op: "nextwordResetFull",
            generation: generation,
            config: nextwordConfig(mode: mode, translateSwapped: translateSwapped, associationRecordingEnabled: associationRecordingEnabled),
        )
    }

    /// Platform → engine UI visibility sync. Call after rendering an async
    /// predict() result so the engine's state.is_showing stays accurate;
    /// downstream `nextwordClearForNewComposing` / sentence-end / context
    /// timeout / resetFull paths gate `clearPredictionsUI` emission on it.
    /// No effects, no current_generation bump.
    // 中文: 平台 → engine 同步 UI 顯示狀態。在渲染完 async predict 結果後呼叫,
    // 中文: 讓 engine 的 state.is_showing 維持正確,後續 clearForNewComposing 等路徑才知道要不要清 UI。
    static func nextwordSetIsShowing(
        _ isShowing: Bool,
        mode: InputMode,
        translateSwapped: Bool,
        associationRecordingEnabled: Bool,
        generation: UInt64,
    ) -> NextWordDecideResult {
        var payload = Taigi_Engine_SetIsShowing()
        payload.isShowing = isShowing
        return decideDispatch(
            method: .setIsShowing(payload),
            op: "nextwordSetIsShowing",
            generation: generation,
            config: nextwordConfig(mode: mode, translateSwapped: translateSwapped, associationRecordingEnabled: associationRecordingEnabled),
        )
    }

    // MARK: - Filter / Boost / QueryState

    static func nextwordFilter(
        raw: [NextWordRawRow],
        queryGeneration: UInt64,
        nowMs: Int64,
        limit: Int32,
        mode: InputMode,
        translateSwapped: Bool,
        associationRecordingEnabled: Bool,
        generation: UInt64,
    ) -> NextWordFilterResult {
        var payload = Taigi_Engine_FilterPredictions()
        payload.raw = raw.map { row in
            var p = Taigi_Engine_RawNextWordPrediction()
            p.hanzi = row.hanzi
            p.tl = row.tl
            p.count = row.count
            p.lastUsedMs = row.lastUsedMs
            p.source = row.source == .dict ? .dict : .user
            return p
        }
        payload.queryGeneration = queryGeneration
        payload.nowMs = nowMs
        payload.limit = limit

        guard let resp = nextwordDispatch(
            method: .filterPredictions(payload),
            op: "nextwordFilter",
            generation: generation,
            config: nextwordConfig(mode: mode, translateSwapped: translateSwapped, associationRecordingEnabled: associationRecordingEnabled),
        ) else {
            return NextWordFilterResult(predictions: [], wasStale: false)
        }
        guard case let .filter(filter)? = resp.result else {
            recordFailure(op: "nextwordFilter", message: "missing filter result")
            return NextWordFilterResult(predictions: [], wasStale: false)
        }
        let predictions = filter.predictions.map { p in
            NextWordEnginePrediction(
                text: p.text,
                subtitle: p.subtitle.isEmpty ? nil : p.subtitle,
                hanzi: p.hanzi,
                tl: p.tl,
                score: p.score,
            )
        }
        return NextWordFilterResult(predictions: predictions, wasStale: filter.wasStale)
    }

    static func nextwordBoostCandidates(
        words: [String],
        predictedFirstChars: Set<String>,
        mode: InputMode,
        translateSwapped: Bool,
        associationRecordingEnabled: Bool,
        generation: UInt64,
    ) -> [String] {
        var payload = Taigi_Engine_BoostCandidates()
        payload.words = words
        payload.predictedFirstChars = Array(predictedFirstChars)

        guard let resp = nextwordDispatch(
            method: .boostCandidates(payload),
            op: "nextwordBoostCandidates",
            generation: generation,
            config: nextwordConfig(mode: mode, translateSwapped: translateSwapped, associationRecordingEnabled: associationRecordingEnabled),
        ) else {
            return words
        }
        guard case let .boost(boost)? = resp.result else {
            recordFailure(op: "nextwordBoostCandidates", message: "missing boost result")
            return words
        }
        return boost.words
    }

    static func nextwordQueryState(
        mode: InputMode,
        translateSwapped: Bool,
        associationRecordingEnabled: Bool,
        generation: UInt64,
    ) -> NextWordStateSnapshot {
        let payload = Taigi_Engine_NextWordQueryState()
        guard let resp = nextwordDispatch(
            method: .queryState(payload),
            op: "nextwordQueryState",
            generation: generation,
            config: nextwordConfig(mode: mode, translateSwapped: translateSwapped, associationRecordingEnabled: associationRecordingEnabled),
        ) else {
            return NextWordStateSnapshot(lastSelectedWord: nil, isShowing: false, currentGeneration: 0)
        }
        guard case let .stateSnapshot(snapshot)? = resp.result else {
            recordFailure(op: "nextwordQueryState", message: "missing state snapshot")
            return NextWordStateSnapshot(lastSelectedWord: nil, isShowing: false, currentGeneration: 0)
        }
        return NextWordStateSnapshot(
            lastSelectedWord: snapshot.lastSelectedWord.isEmpty ? nil : snapshot.lastSelectedWord,
            isShowing: snapshot.isShowing,
            currentGeneration: snapshot.currentGeneration,
        )
    }

    // MARK: - Private helpers

    /// Build the `DecisionInput` proto field shared by every decide intent.
    // 中文: 組出每個 decide intent 共用的 DecisionInput proto 欄位。
    private static func decisionInput(nowMs: Int64) -> Taigi_Engine_DecisionInput {
        var input = Taigi_Engine_DecisionInput()
        input.nowMs = nowMs
        return input
    }

    /// Build an `AppConfig` populated for the NextWord engine. iOS bridge
    /// always sets `platform_id = .ios`; tone toggles default to false (the
    /// NextWord engine does not read them, but the field is required).
    // 中文: 為 NextWord engine 組 AppConfig。iOS bridge 一律 platform_id = .ios,
    // 中文: tone toggles 預設關閉(NextWord engine 不讀,但欄位必填)。
    private static func nextwordConfig(
        mode: InputMode,
        translateSwapped: Bool,
        associationRecordingEnabled: Bool,
    ) -> Taigi_Engine_AppConfig {
        var cfg = Taigi_Engine_AppConfig()
        switch mode {
        case .poj: cfg.inputMode = "poj"
        case .tl: cfg.inputMode = "tl"
        case .english: cfg.inputMode = "english"
        case .tps: cfg.inputMode = "tl" // TPS is a layout, not an engine mode
        }
        cfg.ooDoubletapEnabled = false
        cfg.nnDoubletapEnabled = false
        cfg.isTranslateSwapped = translateSwapped
        cfg.isAssociationRecordingEnabled = associationRecordingEnabled
        cfg.platformID = .ios
        return cfg
    }

    /// Nextword-specific dispatch helper. Mirrors the composing dispatch
    /// pattern. Encodes a `Request` with `payload = .nextword(...)`,
    /// passes it through the FFI seam, decodes, returns the
    /// `NextWordResponse` payload (or nil on any failure path —
    /// `recordFailure` invoked).
    // 中文: NextWord 專用 dispatch helper,複製 composing dispatch 的模式。
    // 中文: 任一失敗路徑都呼叫 recordFailure 並回 nil。
    private static func nextwordDispatch(
        method: Taigi_Engine_NextWordRequest.OneOf_Method,
        op: String,
        generation: UInt64,
        config: Taigi_Engine_AppConfig,
    ) -> Taigi_Engine_NextWordResponse? {
        var nextword = Taigi_Engine_NextWordRequest()
        nextword.method = method

        var request = Taigi_Engine_Request()
        request.id = nextRequestID()
        request.generation = generation
        request.payload = .nextword(nextword)
        request.configSnapshot = config

        let bytes: [UInt8]
        do {
            bytes = try Array(request.serializedData())
        } catch {
            recordFailure(op: op, message: "encode failed: \(error)")
            return nil
        }

        let responseBytes = bytes.withUnsafeBufferPointer { buf in
            process_request_bytes(buf).toArray()
        }
        guard let response = try? Taigi_Engine_Response(
            serializedBytes: Data(responseBytes),
        ) else {
            recordFailure(op: op, message: "response decode failed")
            return nil
        }
        guard response.error == .ok else {
            recordFailure(op: op, message: "engine returned \(response.error)", code: Int32(response.error.rawValue))
            return nil
        }
        guard case let .nextword(payload) = response.payload else {
            recordFailure(op: op, message: "missing nextword payload")
            return nil
        }
        return payload
    }

    /// Decide-result dispatch wrapper. Used by all 5 iOS decide entries.
    // 中文: decide-result 的 dispatch wrapper,iOS 5 個 decide intent 都共用。
    private static func decideDispatch(
        method: Taigi_Engine_NextWordRequest.OneOf_Method,
        op: String,
        generation: UInt64,
        config: Taigi_Engine_AppConfig,
    ) -> NextWordDecideResult {
        guard let resp = nextwordDispatch(method: method, op: op, generation: generation, config: config) else {
            return .noop
        }
        guard case let .decide(decide)? = resp.result else {
            recordFailure(op: op, message: "missing decide result")
            return .noop
        }
        return synthDecideResult(decide)
    }

    private static func synthDecideResult(_ proto: Taigi_Engine_DecideResult) -> NextWordDecideResult {
        let effects: [NextWordDecideResult.Effect] = proto.effects.compactMap { eff in
            guard let kind = eff.kind else { return nil }
            switch kind {
            case let .rescheduleContextTimeout(m):
                return .rescheduleContextTimeout(afterMs: m.afterMs)
            case .cancelContextTimeout:
                return .cancelContextTimeout
            case let .recordAssociation(m):
                return .recordAssociation(synthAssociationPair(m.pair))
            case let .recordCompoundAssociations(m):
                return .recordCompoundAssociations(m.pairs.map(synthAssociationPair))
            case let .queryPredictions(m):
                return .queryPredictions(word: m.word, roman: m.roman, generation: m.generation, nowMs: m.nowMs)
            case let .clearPredictionsUi_p(m):
                return .clearPredictionsUI(generation: m.generation)
            }
        }
        return NextWordDecideResult(
            effects: effects,
            currentGeneration: proto.currentGeneration,
            isShowing: proto.isShowing,
            lastSelectedWord: proto.lastSelectedWord.isEmpty ? nil : proto.lastSelectedWord,
        )
    }

    private static func synthAssociationPair(_ proto: Taigi_Engine_AssociationPair) -> NextWordAssociationPair {
        NextWordAssociationPair(
            prev: proto.prev,
            prevTl: proto.prevTl,
            next: proto.next,
            nextTl: proto.nextTl,
        )
    }
}
