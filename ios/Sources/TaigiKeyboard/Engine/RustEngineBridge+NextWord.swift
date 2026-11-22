import Foundation
import SwiftProtobuf

// MARK: - RustEngineBridge NextWord surface (v3.5.5)

/// NextWord slice extension for `RustEngineBridge`. Mirrors the pattern
/// established in v3.5.4 for the composing slice — proto roundtrip
/// helpers + Swift-friendly synthesized value types.
public extension RustEngineBridge {
    // MARK: - Synthesized value types

    /// Bridge-synthesized companion to the proto `DecideResult`. Consumed
    /// by `NextWordController`; effect-list executes in order on the iOS
    /// platform-executor side.
    struct NextWordDecideResult: Equatable {
        public enum Effect: Equatable {
            case rescheduleContextTimeout(afterMs: UInt64)
            case cancelContextTimeout
            /// `nowMs` is reused by the platform's predict() call so the
            /// engine's association-window clock and the user-row decay
            /// scoring see ONE consistent "now" per intent. Per
            /// `nextword-engine-boundary.md` §13.3.
            case queryPredictions(word: String, roman: String, generation: UInt64, nowMs: Int64)
            case clearPredictionsUI(generation: UInt64)
        }

        public let effects: [Effect]
        public let currentGeneration: UInt64
        public let predictionsVisible: Bool

        public static let noop = NextWordDecideResult(
            effects: [],
            currentGeneration: 0,
            predictionsVisible: false,
        )
    }

    /// UI-ready prediction value. `subtitle` is `nil` when the wire
    /// string is empty (filter contract — happens iff roman is empty).
    struct NextWordEnginePrediction: Equatable {
        public let text: String
        public let subtitle: String?
        /// The Hanji as shown (POJ mode renders a Han-Lo word's romanization in POJ).
        public let hanji: String
        /// The stored Hanji spelling (`EnginePrediction.display_text`): the key a tap
        /// is counted under and NextWord learns.
        public let displayText: String
        public let tl: String
        /// The §42 split cell this row is (`EnginePrediction.cell_script`).
        public let cellScript: Taigi_Engine_CellScript

        public init(
            text: String,
            subtitle: String?,
            hanji: String,
            displayText: String,
            tl: String,
            cellScript: Taigi_Engine_CellScript = .paired,
        ) {
            self.text = text
            self.subtitle = subtitle
            self.hanji = hanji
            self.displayText = displayText
            self.tl = tl
            self.cellScript = cellScript
        }
    }

    /// Filter+merge+sort+limit result. `wasStale=true` indicates the
    /// platform-supplied `queryGeneration` did not match the engine's
    /// current generation — late async result; predictions are empty.
    struct NextWordFilterResult: Equatable {
        public let predictions: [NextWordEnginePrediction]
        public let wasStale: Bool
    }

    // MARK: - Decide intents (6)

    static func nextwordWordSelected(
        text: String,
        roman: String,
        requireRomanMode: Bool,
        triggerPrediction: Bool,
        nowMs: Int64,
        mode: InputMode,
        hanjiFirst: Bool,
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
            config: appConfig(mode: mode, isHanjiFirst: hanjiFirst),
        )
    }

    static func nextwordBackspace(
        lastChar: String,
        nowMs: Int64,
        mode: InputMode,
        hanjiFirst: Bool,
        generation: UInt64,
    ) -> NextWordDecideResult {
        var payload = Taigi_Engine_Backspace()
        payload.lastChar = lastChar
        payload.input = decisionInput(nowMs: nowMs)
        return decideDispatch(
            method: .backspace(payload),
            op: "nextwordBackspace",
            generation: generation,
            config: appConfig(mode: mode, isHanjiFirst: hanjiFirst),
        )
    }

    static func nextwordContextTimeoutFired(
        nowMs: Int64,
        mode: InputMode,
        hanjiFirst: Bool,
        generation: UInt64,
    ) -> NextWordDecideResult {
        var payload = Taigi_Engine_ContextTimeoutFired()
        payload.input = decisionInput(nowMs: nowMs)
        return decideDispatch(
            method: .contextTimeoutFired(payload),
            op: "nextwordContextTimeoutFired",
            generation: generation,
            config: appConfig(mode: mode, isHanjiFirst: hanjiFirst),
        )
    }

    static func nextwordClearForNewComposing(
        nowMs: Int64,
        mode: InputMode,
        hanjiFirst: Bool,
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
            config: appConfig(mode: mode, isHanjiFirst: hanjiFirst),
        )
    }

    static func nextwordResetAll(
        nowMs: Int64,
        mode: InputMode,
        hanjiFirst: Bool,
        generation: UInt64,
    ) -> NextWordDecideResult {
        var payload = Taigi_Engine_ResetAll()
        payload.input = decisionInput(nowMs: nowMs)
        return decideDispatch(
            method: .resetAll(payload),
            op: "nextwordResetAll",
            generation: generation,
            config: appConfig(mode: mode, isHanjiFirst: hanjiFirst),
        )
    }

    /// Platform → engine UI visibility sync. Call after rendering an async
    /// predict() result so the engine's state.predictions_visible stays accurate;
    /// downstream `nextwordClearForNewComposing` / sentence-end / context
    /// timeout / resetFull paths gate `clearPredictionsUI` emission on it.
    /// No effects, no current_generation bump.
    static func nextwordSetPredictionsVisible(
        _ visible: Bool,
        mode: InputMode,
        hanjiFirst: Bool,
        generation: UInt64,
    ) -> NextWordDecideResult {
        var payload = Taigi_Engine_SetPredictionsVisible()
        payload.visible = visible
        return decideDispatch(
            method: .setPredictionsVisible(payload),
            op: "nextwordSetPredictionsVisible",
            generation: generation,
            config: appConfig(mode: mode, isHanjiFirst: hanjiFirst),
        )
    }

    // MARK: - Predict

    /// One next-word query: the engine reads the learned bigrams for `word` /
    /// `roman` from its own `user_association.db` (§24 tiers, roadmap P7b),
    /// looks up the bundled bigrams for the last character of `word` (sources
    /// from `toggles`), and scores, sorts, limits and shapes the result. A stale `queryGeneration` returns `wasStale`.
    static func nextwordPredictNext(
        word: String,
        roman: String,
        toggles: DictionaryToggles,
        queryGeneration: UInt64,
        nowMs: Int64,
        limit: Int32,
        mode: InputMode,
        hanjiFirst: Bool,
        candidateDisplayMode: CandidateDisplayMode = .sideBySide,
        syllableSeparator: SyllableSeparator = .hyphen,
        isKhinsiannInHanjiEnabled: Bool = false,
        generation: UInt64,
    ) -> NextWordFilterResult {
        var payload = Taigi_Engine_PredictNext()
        payload.word = word
        payload.roman = roman
        payload.toggles = dictionaryTogglesProto(toggles)
        payload.queryGeneration = queryGeneration
        payload.nowMs = nowMs
        payload.limit = limit

        guard let resp = nextwordDispatch(
            method: .predictNext(payload),
            op: "nextwordPredictNext",
            generation: generation,
            // The only nextword request that reads the display fields (`nextword/src/filter.rs`), so the caller
            // MUST pass the live settings here (mirrors Android); every other nextword request leaves the defaults.
            config: appConfig(
                mode: mode,
                isHanjiFirst: hanjiFirst,
                candidateDisplayMode: candidateDisplayMode,
                syllableSeparator: syllableSeparator,
                isKhinsiannInHanjiEnabled: isKhinsiannInHanjiEnabled,
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
                hanji: p.hanji,
                displayText: p.displayText,
                tl: p.tl,
                cellScript: p.cellScript,
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
            recordFailure(op: op, message: "engine returned \(response.error)")
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

    /// Also reads the decision a composing answer carries (`Effect.next_word_decided`).
    static func synthDecideResult(_ proto: Taigi_Engine_DecideResult) -> NextWordDecideResult {
        let effects: [NextWordDecideResult.Effect] = proto.effects.compactMap { eff in
            guard let kind = eff.kind else { return nil }
            switch kind {
            case let .rescheduleContextTimeout(m):
                return .rescheduleContextTimeout(afterMs: m.afterMs)
            case .cancelContextTimeout:
                return .cancelContextTimeout
            case let .queryPredictions(m):
                return .queryPredictions(word: m.word, roman: m.roman, generation: m.generation, nowMs: m.nowMs)
            case let .clearPredictionsUi_p(m):
                return .clearPredictionsUI(generation: m.generation)
            }
        }
        return NextWordDecideResult(
            effects: effects,
            currentGeneration: proto.currentGeneration,
            predictionsVisible: proto.predictionsVisible,
        )
    }
}
