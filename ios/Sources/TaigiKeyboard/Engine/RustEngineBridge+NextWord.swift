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
public extension RustEngineBridge {
    // MARK: - Synthesized value types

    /// Bridge-synthesized companion to the proto `DecideResult`. Consumed
    /// by `NextWordController`; effect-list executes in order on the iOS
    /// platform-executor side.
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
    struct NextWordAssociationPair: Equatable {
        public let prev: String
        public let prevTl: String
        public let next: String
        public let nextTl: String
    }

    /// UI-ready prediction value. `subtitle` is `nil` when the wire
    /// string is empty (filter contract — happens iff roman is empty).
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
    struct NextWordFilterResult: Equatable {
        public let predictions: [NextWordEnginePrediction]
        public let wasStale: Bool
    }

    /// Pre-merge un-scored row from the platform `NextWordService.userRows`
    /// SQL pipeline. Crosses the bridge as a `PredictNext.user_rows` entry.
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

    // MARK: - Decide intents (6)

    /// v3.5.8 Phase 4 — continuous-input mid-commit handshake. Updates
    /// `state.last_selected_word` + `last_selection_time_ms` without
    /// bumping `current_generation`, no timer effects, emits compound-only
    /// `RecordCompoundAssociations` effect. Pre-v3.5.8 this was Android-only;
    /// the Phase 4 effect-based handshake brought iOS into the call site.
    static func nextwordUpdateLastSelectedWord(
        text: String,
        roman: String,
        nowMs: Int64,
        mode: InputMode,
        translateSwapped: Bool,
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
            config: nextwordConfig(
                mode: mode,
                translateSwapped: translateSwapped,
            ),
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
            config: nextwordConfig(
                mode: mode,
                translateSwapped: translateSwapped,
            ),
        )
    }

    static func nextwordBackspace(
        lastChar: String,
        nowMs: Int64,
        mode: InputMode,
        translateSwapped: Bool,
        generation: UInt64,
    ) -> NextWordDecideResult {
        var payload = Taigi_Engine_Backspace()
        payload.lastChar = lastChar
        payload.input = decisionInput(nowMs: nowMs)
        return decideDispatch(
            method: .backspace(payload),
            op: "nextwordBackspace",
            generation: generation,
            config: nextwordConfig(
                mode: mode,
                translateSwapped: translateSwapped,
            ),
        )
    }

    static func nextwordContextTimeoutFired(
        nowMs: Int64,
        mode: InputMode,
        translateSwapped: Bool,
        generation: UInt64,
    ) -> NextWordDecideResult {
        var payload = Taigi_Engine_ContextTimeoutFired()
        payload.input = decisionInput(nowMs: nowMs)
        return decideDispatch(
            method: .contextTimeoutFired(payload),
            op: "nextwordContextTimeoutFired",
            generation: generation,
            config: nextwordConfig(
                mode: mode,
                translateSwapped: translateSwapped,
            ),
        )
    }

    static func nextwordClearForNewComposing(
        nowMs: Int64,
        mode: InputMode,
        translateSwapped: Bool,
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
            config: nextwordConfig(
                mode: mode,
                translateSwapped: translateSwapped,
            ),
        )
    }

    static func nextwordResetFull(
        nowMs: Int64,
        mode: InputMode,
        translateSwapped: Bool,
        generation: UInt64,
    ) -> NextWordDecideResult {
        var payload = Taigi_Engine_ResetFull()
        payload.input = decisionInput(nowMs: nowMs)
        return decideDispatch(
            method: .resetFull(payload),
            op: "nextwordResetFull",
            generation: generation,
            config: nextwordConfig(
                mode: mode,
                translateSwapped: translateSwapped,
            ),
        )
    }

    /// Platform → engine UI visibility sync. Call after rendering an async
    /// predict() result so the engine's state.is_showing stays accurate;
    /// downstream `nextwordClearForNewComposing` / sentence-end / context
    /// timeout / resetFull paths gate `clearPredictionsUI` emission on it.
    /// No effects, no current_generation bump.
    static func nextwordSetIsShowing(
        _ isShowing: Bool,
        mode: InputMode,
        translateSwapped: Bool,
        generation: UInt64,
    ) -> NextWordDecideResult {
        var payload = Taigi_Engine_SetIsShowing()
        payload.isShowing = isShowing
        return decideDispatch(
            method: .setIsShowing(payload),
            op: "nextwordSetIsShowing",
            generation: generation,
            config: nextwordConfig(
                mode: mode,
                translateSwapped: translateSwapped,
            ),
        )
    }

    // MARK: - Predict

    /// One next-word query: the engine looks up the bundled bigrams for the
    /// last character of `word` (sources from `toggles`), merges them with
    /// `userRows` — kept in their SQL order — and scores, sorts, limits and
    /// shapes the result. A stale `queryGeneration` returns `wasStale`.
    static func nextwordPredictNext(
        word: String,
        userRows: [NextWordRawRow],
        toggles: DictionaryToggles,
        queryGeneration: UInt64,
        nowMs: Int64,
        limit: Int32,
        mode: InputMode,
        translateSwapped: Bool,
        candidateDisplayMode: CandidateDisplayMode = .sideBySide,
        hyphenlessRoman: Bool = false,
        generation: UInt64,
    ) -> NextWordFilterResult {
        var payload = Taigi_Engine_PredictNext()
        payload.word = word
        payload.toggles = dictionaryTogglesProto(toggles)
        payload.userRows = userRows.map { row in
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
            method: .predictNext(payload),
            op: "nextwordPredictNext",
            generation: generation,
            config: nextwordConfig(
                mode: mode,
                translateSwapped: translateSwapped,
                candidateDisplayMode: candidateDisplayMode,
                hyphenlessRoman: hyphenlessRoman,
            ),
        ) else {
            return NextWordFilterResult(predictions: [], wasStale: false)
        }
        guard case let .filter(filter)? = resp.result else {
            recordFailure(op: "nextwordPredictNext", message: "missing filter result")
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

    // MARK: - Private helpers

    /// Build the `DecisionInput` proto field shared by every decide intent.
    private static func decisionInput(nowMs: Int64) -> Taigi_Engine_DecisionInput {
        var input = Taigi_Engine_DecisionInput()
        input.nowMs = nowMs
        return input
    }

    /// Build an `AppConfig` populated for the NextWord engine. iOS bridge
    /// always sets `platform_id = .ios`; tone toggles default to false (the
    /// NextWord engine does not read them, but the field is required).
    // `candidateDisplayMode` and `hyphenlessRoman` ride only `nextwordPredictNext` — the sole nextword reader of
    // fields 9 / 10 (mirrors Android); `nextwordPredictNext` MUST pass the live settings; the other entry points
    // leave the defaults.
    private static func nextwordConfig(
        mode: InputMode,
        translateSwapped: Bool,
        candidateDisplayMode: CandidateDisplayMode = .sideBySide,
        hyphenlessRoman: Bool = false,
    ) -> Taigi_Engine_AppConfig {
        var cfg = Taigi_Engine_AppConfig()
        // Proto field 9 — the nextword filter collapses same-roman predictions under 羅馬字.
        cfg.candidateDisplayMode = candidateDisplayMode.engineValue
        // Proto field 10 — 無連字符 shapes `EnginePrediction.text`; `tl` keeps the hyphen.
        cfg.hyphenlessRoman = hyphenlessRoman
        switch mode {
        case .poj: cfg.inputMode = "poj"
        case .tl: cfg.inputMode = "tl"
        case .english: cfg.inputMode = "english"
        case .tps: cfg.inputMode = "tl" // TPS is a layout, not an engine mode
        }
        cfg.ooDoubletapEnabled = false
        cfg.nnDoubletapEnabled = false
        cfg.isTranslateSwapped = translateSwapped
        cfg.platformID = .ios
        return cfg
    }

    /// Nextword-specific dispatch helper. Mirrors the composing dispatch
    /// pattern. Encodes a `Request` with `payload = .nextword(...)`,
    /// passes it through the FFI seam, decodes, returns the
    /// `NextWordResponse` payload (or nil on any failure path —
    /// `recordFailure` invoked).
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

        guard let response = send(request, op: op) else { return nil }
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
