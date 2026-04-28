import Foundation
import SwiftProtobuf

// MARK: - RustEngineBridge

/// Thin Swift wrapper around the Rust shared-core FFI exposed by
/// `engine/swift-ffi/src/lib.rs`.
///
/// D9.4 surface: 17 typed methods + lazy `toneVariations` cache + structured
/// error visibility. Production phonetics call sites route through these
/// methods; legacy `Phonetics/*` and `Input/TPS/*` modules are deleted in
/// later commits.
///
/// Per `feedback_codex_review_sandwich.md` Codex v2 §7: every method
/// requiring AppConfig (currently NormalizeTone for POJ preprocessing)
/// takes the necessary fields as mandatory parameters — no global default.
///
/// Per Codex v2 §8 + v3 §7: error visibility is hardened. DEBUG asserts on
/// failure; release returns a graceful fallback + logs + records a
/// structured `DiagnosticsEntry` in a bounded in-memory queue accessible
/// via `diagnostics()` for dogfood inspection.
public enum RustEngineBridge {
    private static let installLock = NSLock()
    private static var installed = false

    /// Idempotent. Registers a logger sink that forwards every Rust
    /// `log::warn!` (and friends) into the platform `LoggerBackend`.
    /// In DEBUG builds, also bumps Rust `log::max_level` to `Debug` so
    /// dogfood traces are visible. Release stays at default `Warn` so
    /// `log::debug!`/`log::info!` macros short-circuit before format —
    /// no FFI cost for the no-op render path on `DebugLogger`.
    public static func install() {
        installLock.lock()
        defer { installLock.unlock() }
        guard !installed else { return }
        install_logger_sink(SwiftLoggerSink())
        #if DEBUG
        set_log_level(4) // 4 = Debug, see SwiftLoggerSink level constants
        #endif
        installed = true
    }

    // MARK: Phonetics core (9 ops)

    /// `Method::NormalizeTone` — input + AppConfig.input_mode + ToneToggles →
    /// tone-marked string. Caller MUST supply `ToneToggles`; engine reads
    /// them per request (live-read invariant).
    public static func normalizeTone(
        _ input: String,
        mode: InputMode,
        toggles: ToneToggles
    ) -> String {
        var payload = Taigi_Engine_NormalizeTone()
        payload.input = input
        return stringDispatch(
            method: .normalizeTone(payload),
            input: input,
            op: "normalizeTone",
            config: appConfig(mode: mode, toggles: toggles)
        )
    }

    public static func stripTone(_ input: String) -> (bare: String, tone: String) {
        var payload = Taigi_Engine_StripTone()
        payload.input = input
        let resp = dispatch(method: .stripTone(payload), op: "stripTone", config: nil)
        guard case let .stripToneResult(r)? = resp?.result else {
            recordFailure(op: "stripTone", message: "missing result")
            return (input, "")
        }
        return (r.bare, r.tone)
    }

    public static func pojToTl(_ input: String) -> String {
        var payload = Taigi_Engine_PojToTl()
        payload.input = input
        return stringDispatch(method: .pojToTl(payload), input: input, op: "pojToTl", config: nil)
    }

    public static func tlToPoj(_ input: String) -> String {
        var payload = Taigi_Engine_TlToPoj()
        payload.input = input
        return stringDispatch(method: .tlToPoj(payload), input: input, op: "tlToPoj", config: nil)
    }

    public static func normalizeToTl(_ input: String) -> String {
        var payload = Taigi_Engine_NormalizeToTl()
        payload.input = input
        return stringDispatch(
            method: .normalizeToTl(payload),
            input: input,
            op: "normalizeToTl",
            config: nil
        )
    }

    public static func normalizeInput(_ input: String) -> String {
        var payload = Taigi_Engine_NormalizeInput()
        payload.input = input
        return stringDispatch(
            method: .normalizeInput(payload),
            input: input,
            op: "normalizeInput",
            config: nil
        )
    }

    public static func restoreTone(_ text: String) -> String? {
        var payload = Taigi_Engine_RestoreTone()
        payload.text = text
        let resp = dispatch(method: .restoreTone(payload), op: "restoreTone", config: nil)
        guard case let .optionalStringResult(r)? = resp?.result else {
            recordFailure(op: "restoreTone", message: "missing result")
            return nil
        }
        return r.present ? r.output : nil
    }

    public static func hasToneMarks(_ text: String) -> Bool {
        var payload = Taigi_Engine_HasToneMarks()
        payload.text = text
        return boolDispatch(method: .hasToneMarks_p(payload), op: "hasToneMarks")
    }

    /// Lazy-init cache for `Method::GetToneVariations`. Swift `static let`
    /// initializer is dispatch_once-equivalent — thread-safe by construction.
    public static let toneVariations: ToneVariationsCache = {
        let resp = dispatch(method: .getToneVariations(Taigi_Engine_GetToneVariations()),
                            op: "getToneVariations",
                            config: nil)
        guard case let .toneVariationsResult(r)? = resp?.result else {
            recordFailure(op: "getToneVariations", message: "missing result")
            return ToneVariationsCache(poj: [:], tl: [:])
        }
        return ToneVariationsCache(
            poj: r.pojVariations.mapValues { $0.variations },
            tl: r.tlVariations.mapValues { $0.variations }
        )
    }()

    // MARK: Derivation (2 ops)

    public static func deriveNotone(_ roman: String) -> String {
        var payload = Taigi_Engine_DeriveNotone()
        payload.roman = roman
        return stringDispatch(method: .deriveNotone(payload), input: roman, op: "deriveNotone", config: nil)
    }

    public static func deriveAbbrev(_ roman: String) -> String {
        var payload = Taigi_Engine_DeriveAbbrev()
        payload.roman = roman
        return stringDispatch(method: .deriveAbbrev(payload), input: roman, op: "deriveAbbrev", config: nil)
    }

    // MARK: TPS (6 ops)

    public static func containsTPS(_ text: String) -> Bool {
        var payload = Taigi_Engine_ContainsTps()
        payload.text = text
        return boolDispatch(method: .containsTps(payload), op: "containsTps")
    }

    public static func tpsToTL(_ text: String) -> String {
        var payload = Taigi_Engine_TpsToTl()
        payload.text = text
        return stringDispatch(method: .tpsToTl(payload), input: text, op: "tpsToTl", config: nil)
    }

    public static func tlNumericToTPS(_ text: String, orMapsToER: Bool) -> String {
        var payload = Taigi_Engine_TlNumericToTps()
        payload.text = text
        payload.orMapsToEr = orMapsToER
        return stringDispatch(
            method: .tlNumericToTps(payload),
            input: text,
            op: "tlNumericToTps",
            config: nil
        )
    }

    public static func tlDisplayToTPS(_ text: String, orMapsToER: Bool) -> String {
        var payload = Taigi_Engine_TlDisplayToTps()
        payload.text = text
        payload.orMapsToEr = orMapsToER
        return stringDispatch(
            method: .tlDisplayToTps(payload),
            input: text,
            op: "tlDisplayToTps",
            config: nil
        )
    }

    public static func isTPSToneMark(_ char: Character) -> Bool {
        var payload = Taigi_Engine_IsTpsToneMark()
        payload.char = String(char)
        return boolDispatch(method: .isTpsToneMark(payload), op: "isTpsToneMark")
    }

    public static func tpsInputAdjust(
        incoming: String,
        rawInput: String
    ) -> (adjusted: String, replaceLast: String?) {
        var payload = Taigi_Engine_TpsInputAdjust()
        payload.incoming = incoming
        payload.rawInput = rawInput
        let resp = dispatch(method: .tpsInputAdjust(payload), op: "tpsInputAdjust", config: nil)
        guard case let .tpsAdjustResult(r)? = resp?.result else {
            recordFailure(op: "tpsInputAdjust", message: "missing result")
            return (incoming, nil)
        }
        let replace = r.hasReplaceLast && r.replaceLast.present ? r.replaceLast.output : nil
        return (r.adjusted, replace)
    }

    // MARK: Lexicon ranking (1 op)

    /// `Method::ProcessCandidates` — runs the lexicon ranking pipeline
    /// (dedup → score → sort → optional TPS display-dedup) atomically in
    /// the Rust core. Mirrors `engine/ranking/src/process.rs`.
    ///
    /// Cold-start callers that lack a connected user-frequency DB should
    /// short-circuit to platform `CandidateProcessor.removeDuplicates` /
    /// `removeDisplayDuplicates` rather than calling this with an empty
    /// `frequencyData` map — the score-sort is deterministic but reorders
    /// candidates against the legacy iOS "merged-order on cold-start"
    /// behavior. See `LexiconService.search` for the gating logic.
    ///
    /// `tpsDedupEnabled` is platform-decided (audit § 3) — pass
    /// `inputMode == .tps` from the call site. The engine never derives
    /// it from `AppConfig.inputMode`.
    ///
    /// `nowMs` is caller-supplied for deterministic recency-window math
    /// in tests; production passes `Int64(Date().timeIntervalSince1970 * 1000)`.
    ///
    /// In `#if DEBUG`, requests + logs the per-candidate `ScoreBreakdown`
    /// alongside the ranked list so dogfood traces match the legacy
    /// `CandidateProcessor.logScoreDetails` output. Release builds skip
    /// the breakdown (zero serialization overhead).
    public static func processCandidates(
        raw: [TaigiWord],
        normalizedInput: String,
        tpsDedupEnabled: Bool,
        frequencyData: [String: FrequencyData],
        nowMs: Int64
    ) -> [TaigiWord] {
        #if DEBUG
            let detailed = processCandidatesDetailed(
                raw: raw,
                normalizedInput: normalizedInput,
                tpsDedupEnabled: tpsDedupEnabled,
                frequencyData: frequencyData,
                nowMs: nowMs,
                includeBreakdown: true,
            )
            if detailed.breakdowns.count == detailed.ranked.count {
                let logger = LoggerFactory.make(category: "RustEngineBridge")
                for (index, word) in detailed.ranked.enumerated() {
                    let b = detailed.breakdowns[index]
                    logger.debug("[SCORE] input='\(normalizedInput)' | \(word.roman) \(word.hanzi ?? ""): user=\(b.userFreqScore) recency=\(b.recencyBonus) exact=\(b.exactBonus) close=\(b.closenessBonus) base=\(b.baseFreqScore) completion=\(b.completionPenalty) total=\(b.total)")
                }
            }
            return detailed.ranked
        #else
            return processCandidatesDetailed(
                raw: raw,
                normalizedInput: normalizedInput,
                tpsDedupEnabled: tpsDedupEnabled,
                frequencyData: frequencyData,
                nowMs: nowMs,
                includeBreakdown: false,
            ).ranked
        #endif
    }

    /// Per-candidate score breakdown returned alongside `ranked` when the
    /// caller opts in. Six fields sum to the engine's sort key.
    public struct ScoreBreakdown: Equatable {
        public let userFreqScore: Int
        public let recencyBonus: Int
        public let exactBonus: Int
        public let completionPenalty: Int
        public let closenessBonus: Int
        public let baseFreqScore: Int

        public var total: Int {
            userFreqScore + recencyBonus + exactBonus + completionPenalty + closenessBonus + baseFreqScore
        }
    }

    /// Composite return for the lexicon ranking pipeline. Production
    /// callers typically just read `ranked`; tests inspect `breakdowns`
    /// to pin scoring math on the bridge boundary.
    public struct CandidateRanking: Equatable {
        public let ranked: [TaigiWord]
        public let breakdowns: [ScoreBreakdown]
    }

    /// Test seam — same FFI call as `processCandidates`, plus access to
    /// the per-candidate `ScoreBreakdown` payload. The Swift-side parity
    /// tests in `RustEngineBridgeRankingTests` use this to assert the
    /// engine's score arithmetic; production code stays on the public
    /// `processCandidates` method which discards the breakdown after
    /// debug logging.
    public static func processCandidatesDetailed(
        raw: [TaigiWord],
        normalizedInput: String,
        tpsDedupEnabled: Bool,
        frequencyData: [String: FrequencyData],
        nowMs: Int64,
        includeBreakdown: Bool
    ) -> CandidateRanking {
        var payload = Taigi_Engine_ProcessCandidatesRequest()
        payload.raw = raw.map(taigiWordToProto)
        payload.normalizedInput = normalizedInput
        payload.tpsDedupEnabled = tpsDedupEnabled
        payload.freq = frequencyData.map { key, value in
            var entry = Taigi_Engine_FrequencyEntry()
            entry.displayTextKey = key
            entry.count = UInt32(max(0, value.count))
            entry.lastUsedMs = value.lastUsedMillis
            return entry
        }
        payload.nowMs = nowMs
        payload.includeBreakdown = includeBreakdown

        let resp = lexiconDispatch(method: .processCandidates(payload), op: "processCandidates")
        guard case let .processCandidatesResult(result)? = resp?.result else {
            recordFailure(op: "processCandidates", message: "missing process_candidates_result")
            return CandidateRanking(
                ranked: fallbackRanked(raw: raw, tpsDedupEnabled: tpsDedupEnabled),
                breakdowns: [],
            )
        }
        let ranked = result.ranked.map(taigiWordFromProto)
        let breakdowns = result.breakdown.map(scoreBreakdownFromProto)
        return CandidateRanking(ranked: ranked, breakdowns: breakdowns)
    }

    /// Defense-in-depth ranking on the FFI error path. When the Rust
    /// lexicon dispatch fails (encode error, decode error, non-OK engine
    /// response, or missing payload variant), fall back to the retained
    /// platform `CandidateProcessor` helpers so the user still sees a
    /// deduplicated and (TPS-gated) display-deduped candidate list
    /// instead of the raw merged input. Score-sort is skipped because
    /// the bridge owns user-frequency lookups; the input list arrives
    /// pre-sorted by `lengthScore` from `LexiconService.searchWithTrie`,
    /// which preserves a "reasonable" order even on the error path.
    ///
    /// Mirrors Android `RustEngineBridge.fallbackRanked`. Audit § 8 row
    /// "FFI error path graceful degradation" documents the rationale.
    private static func fallbackRanked(raw: [TaigiWord], tpsDedupEnabled: Bool) -> [TaigiWord] {
        let deduped = CandidateProcessor.removeDuplicates(raw)
        return tpsDedupEnabled ? CandidateProcessor.removeDisplayDuplicates(deduped) : deduped
    }

    private static func scoreBreakdownFromProto(_ proto: Taigi_Engine_ScoreBreakdown) -> ScoreBreakdown {
        ScoreBreakdown(
            userFreqScore: Int(proto.userFreqScore),
            recencyBonus: Int(proto.recencyBonus),
            exactBonus: Int(proto.exactBonus),
            completionPenalty: Int(proto.completionPenalty),
            closenessBonus: Int(proto.closenessBonus),
            baseFreqScore: Int(proto.baseFreqScore),
        )
    }

    private static func taigiWordToProto(_ word: TaigiWord) -> Taigi_Engine_TaigiWord {
        var proto = Taigi_Engine_TaigiWord()
        proto.id = Int64(word.id)
        proto.roman = word.roman
        if let hanzi = word.hanzi { proto.hanji = hanzi }
        if let length = word.lengthScore { proto.lengthScore = Int32(length) }
        if let mask = word.sourceBitmask { proto.sourceBitmask = UInt32(mask) }
        return proto
    }

    private static func taigiWordFromProto(_ proto: Taigi_Engine_TaigiWord) -> TaigiWord {
        TaigiWord(
            id: Int(proto.id),
            roman: proto.roman,
            hanzi: proto.hasHanji ? proto.hanji : nil,
            lengthScore: proto.hasLengthScore ? Int(proto.lengthScore) : nil,
            sourceBitmask: proto.hasSourceBitmask ? UInt16(truncatingIfNeeded: proto.sourceBitmask) : nil
        )
    }

    // MARK: Diagnostics (Codex v2 §8 / v3 §7)

    public struct DiagnosticsEntry: Equatable {
        public let timestamp: Date
        public let op: String
        public let errorCode: Int32
        public let message: String
    }

    /// Read-only snapshot of in-memory failure tracking. For debug menu
    /// + test inspection. Counter increments on every fallback path
    /// (encode error, decode error, dispatch returned non-OK, missing
    /// result variant). Recent entries capped at 32 to bound memory.
    public static func diagnostics() -> (failureCount: Int, recentErrors: [DiagnosticsEntry]) {
        diagnosticsLock.lock()
        defer { diagnosticsLock.unlock() }
        return (Int(failureCounter), Array(recentErrorBuffer))
    }

    /// Test-only: clears counters so independent test cases don't bleed.
    static func resetDiagnosticsForTesting() {
        diagnosticsLock.lock()
        defer { diagnosticsLock.unlock() }
        failureCounter = 0
        recentErrorBuffer.removeAll()
    }

    // MARK: Test-only seam

    /// Sends arbitrary bytes through the FFI seam. Tests use this for T4
    /// (malformed protobuf) / T5 (oversized payload) / T7' (empty bytes).
    static func sendRawBytes(_ bytes: [UInt8]) -> Taigi_Engine_Response? {
        let responseBytes = bytes.withUnsafeBufferPointer { buf in
            process_request_bytes(buf).toArray()
        }
        return try? Taigi_Engine_Response(serializedBytes: Data(responseBytes))
    }

    /// Drives T1. Resolves to a panic inside the Rust `catch_unwind` boundary
    /// when the dev xcframework (built with `--features panic-injector`) is
    /// linked.
    static func panicForTestRaw() -> Taigi_Engine_Response? {
        let responseBytes = panic_for_test().toArray()
        return try? Taigi_Engine_Response(serializedBytes: Data(responseBytes))
    }

    // MARK: Private dispatch

    private static let idLock = NSLock()
    private static var nextID: UInt32 = 0
    private static func nextRequestID() -> UInt32 {
        idLock.lock()
        defer { idLock.unlock() }
        nextID &+= 1
        return nextID
    }

    private static let diagnosticsLock = NSLock()
    private static var failureCounter: UInt32 = 0
    private static var recentErrorBuffer: [DiagnosticsEntry] = []

    private static let recentErrorCap = 32

    private static func recordFailure(op: String, message: String, code: Int32 = -1) {
        diagnosticsLock.lock()
        defer { diagnosticsLock.unlock() }
        failureCounter &+= 1
        let entry = DiagnosticsEntry(
            timestamp: Date(),
            op: op,
            errorCode: code,
            message: message
        )
        recentErrorBuffer.append(entry)
        if recentErrorBuffer.count > recentErrorCap {
            recentErrorBuffer.removeFirst(recentErrorBuffer.count - recentErrorCap)
        }
        let logger = LoggerFactory.make(category: "RustEngineBridge")
        logger.warning("[\(op)] \(message)")
        #if DEBUG
        assertionFailure("RustEngineBridge.\(op) failed: \(message)")
        #endif
    }

    private static func appConfig(mode: InputMode, toggles: ToneToggles) -> Taigi_Engine_AppConfig {
        var cfg = Taigi_Engine_AppConfig()
        switch mode {
        case .poj: cfg.inputMode = "poj"
        case .tl: cfg.inputMode = "tl"
        case .english: cfg.inputMode = "english"
        case .tps: cfg.inputMode = "tl" // TPS is a layout, not an engine mode
        }
        cfg.ooDoubletapEnabled = toggles.isDoubleTapOOEnabled
        cfg.nnDoubletapEnabled = toggles.isDoubleTapNNEnabled
        return cfg
    }

    private static func dispatch(
        method: Taigi_Engine_PhoneticsRequest.OneOf_Method,
        op: String,
        config: Taigi_Engine_AppConfig?
    ) -> Taigi_Engine_PhoneticsResponse? {
        var phonetics = Taigi_Engine_PhoneticsRequest()
        phonetics.method = method

        var request = Taigi_Engine_Request()
        request.id = nextRequestID()
        request.payload = .phonetics(phonetics)
        if let config { request.configSnapshot = config }

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
            serializedBytes: Data(responseBytes)
        ) else {
            recordFailure(op: op, message: "response decode failed")
            return nil
        }
        guard response.error == .ok else {
            recordFailure(op: op, message: "engine returned \(response.error)", code: Int32(response.error.rawValue))
            return nil
        }
        guard case let .phonetics(payload) = response.payload else {
            recordFailure(op: op, message: "missing phonetics payload")
            return nil
        }
        return payload
    }

    private static func stringDispatch(
        method: Taigi_Engine_PhoneticsRequest.OneOf_Method,
        input: String,
        op: String,
        config: Taigi_Engine_AppConfig?
    ) -> String {
        guard let resp = dispatch(method: method, op: op, config: config) else { return input }
        guard case let .stringResult(s)? = resp.result else {
            recordFailure(op: op, message: "expected StringResult")
            return input
        }
        return s.output
    }

    private static func lexiconDispatch(
        method: Taigi_Engine_LexiconRequest.OneOf_Method,
        op: String
    ) -> Taigi_Engine_LexiconResponse? {
        var lexicon = Taigi_Engine_LexiconRequest()
        lexicon.method = method

        var request = Taigi_Engine_Request()
        request.id = nextRequestID()
        request.payload = .lexicon(lexicon)

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
            serializedBytes: Data(responseBytes)
        ) else {
            recordFailure(op: op, message: "response decode failed")
            return nil
        }
        guard response.error == .ok else {
            recordFailure(op: op, message: "engine returned \(response.error)", code: Int32(response.error.rawValue))
            return nil
        }
        guard case let .lexicon(payload) = response.payload else {
            recordFailure(op: op, message: "missing lexicon payload")
            return nil
        }
        return payload
    }

    private static func boolDispatch(
        method: Taigi_Engine_PhoneticsRequest.OneOf_Method,
        op: String
    ) -> Bool {
        guard let resp = dispatch(method: method, op: op, config: nil) else { return false }
        guard case let .boolResult(b)? = resp.result else {
            recordFailure(op: op, message: "expected BoolResult")
            return false
        }
        return b.value
    }
}

// MARK: - ToneVariationsCache

/// Init-bulk-pull cache for the callout tone variation tables. Loaded once
/// at first access via `RustEngineBridge.toneVariations`; both POJ + TL
/// maps live in a single payload to amortize FFI cost.
public struct ToneVariationsCache {
    public let poj: [String: [String]]
    public let tl: [String: [String]]
}

// MARK: - Logger sink

public final class SwiftLoggerSink {
    static let levelError: UInt8 = 0
    static let levelWarn: UInt8 = 1
    static let levelInfo: UInt8 = 2
    static let levelDebug: UInt8 = 3
    static let levelTrace: UInt8 = 4

    public init() {}

    public func log(level: UInt8, category: RustString, message: RustString) {
        let backend = LoggerFactory.make(category: category.toString())
        let text = message.toString()
        switch level {
        case Self.levelError: backend.error(text)
        case Self.levelWarn: backend.warning(text)
        case Self.levelInfo: backend.info(text)
        default: backend.debug(text)
        }
    }
}

// MARK: - swift-bridge interop helpers

private extension RustVec where T == UInt8 {
    func toArray() -> [UInt8] {
        let count = Int(len())
        var out = [UInt8]()
        out.reserveCapacity(count)
        for i in 0 ..< count {
            out.append(get(index: UInt(i)) ?? 0)
        }
        return out
    }
}
