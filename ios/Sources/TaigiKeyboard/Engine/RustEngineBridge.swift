// 中文: Rust shared-core FFI 的 Swift 端薄包裝層。
// 中文: 對應 engine/swift-ffi/src/lib.rs;Composing / NextWord / Lexicon / case-transform
// 中文: 等切片各自有獨立的 RustEngineBridge+*.swift extension 檔。

import Foundation
import SwiftProtobuf

// MARK: - RustEngineBridge

/// Thin Swift wrapper around the Rust shared-core FFI exposed by
/// `engine/swift-ffi/src/lib.rs`.
///
/// D9.4 surface: 15 typed phonetics methods + lazy `toneVariations` cache +
/// structured error visibility. Composing / NextWord / Lexicon / case-transform
/// methods live in dedicated `RustEngineBridge+*.swift` extensions. Retired-op
/// history available via git log on `engine/protos/proto/phonetics.proto`.
///
/// Per `feedback_codex_review_sandwich.md` Codex v2 §7: every method
/// requiring AppConfig (currently NormalizeTone for POJ preprocessing)
/// takes the necessary fields as mandatory parameters — no global default.
///
/// Per Codex v2 §8 + v3 §7: error visibility is hardened. DEBUG asserts on
/// failure; release returns a graceful fallback + logs + records a
/// structured `DiagnosticsEntry` in a bounded in-memory queue accessible
/// via `diagnostics()` for dogfood inspection.
// 中文: Rust 引擎橋接層的對外型別。所有 phonetics / composing / lexicon /
// 中文: case-transform / nextword 操作都從這個 enum 進入。
// 中文: AppConfig 採每呼叫顯式傳入策略,不留全域預設。
// 中文: 錯誤路徑於 DEBUG 會 assertionFailure;Release 走 fallback + 寫入
// 中文: 上限 32 筆的診斷環形緩衝,供 diagnostics() 讀取。
public enum RustEngineBridge {
    private static let installLock = NSLock()
    private static var installed = false

    /// Idempotent. Registers a logger sink that forwards every Rust
    /// `log::warn!` (and friends) into the platform `LoggerBackend`.
    /// In DEBUG builds, also bumps Rust `log::max_level` to `Debug` so
    /// dogfood traces are visible. Release stays at default `Warn` so
    /// `log::debug!`/`log::info!` macros short-circuit before format —
    /// no FFI cost for the no-op render path on `DebugLogger`.
    // 中文: 安裝 Rust log sink + DEBUG 時調整 max_level。冪等,內部用 NSLock 防重入。
    // 中文: Release 維持 Warn 等級,debug! / info! 巨集短路,不付 FFI 成本。
    public static func install() {
        installLock.lock()
        defer { installLock.unlock() }
        guard !installed else { return }
        install_logger_sink(SwiftLoggerSink())
        #if DEBUG
            set_log_level(4) // 4 = Debug per set_log_level Rust-side mapping (engine/swift-ffi)
        #endif
        installed = true
    }

    // MARK: Phonetics core (8 ops)

    /// `Method::NormalizeTone` — input + AppConfig.input_mode + ToneToggles →
    /// tone-marked string. Caller MUST supply `ToneToggles`; engine reads
    /// them per request (live-read invariant).
    // 中文: 把數字調 ASCII 輸入轉為帶調符字串。ToneToggles 必填,引擎每次呼叫時讀取。
    public static func normalizeTone(
        _ input: String,
        mode: InputMode,
        toggles: ToneToggles,
    ) -> String {
        var payload = Taigi_Engine_NormalizeTone()
        payload.input = input
        return stringDispatch(
            method: .normalizeTone(payload),
            input: input,
            op: "normalizeTone",
            config: appConfig(mode: mode, toggles: toggles),
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
            config: nil,
        )
    }

    public static func normalizeInput(_ input: String) -> String {
        var payload = Taigi_Engine_NormalizeInput()
        payload.input = input
        return stringDispatch(
            method: .normalizeInput(payload),
            input: input,
            op: "normalizeInput",
            config: nil,
        )
    }

    /// Replaces platform `TaigiUnicode.nfdPreprocessed(_:)`. Lookup-side
    /// NFD prep used by `ExternalLookupURLBuilder` before tone stripping.
    /// Distinct semantics from `normalizeInput` — this preserves tone
    /// diacritics; only nasal markers (ⁿ / ᴺ → "nn") and standalone
    /// `\u{0358}` → `o` are rewritten.
    // 中文: 查詢用 NFD 前處理 — 保留調符,僅改寫鼻音標記與孤立 \u{0358}。
    // 中文: 與 normalizeInput 語意不同,後者會脫掉調符。
    public static func nfdPreprocessForLookup(_ input: String) -> String {
        var payload = Taigi_Engine_NfdPreprocessForLookup()
        payload.input = input
        return stringDispatch(
            method: .nfdPreprocessForLookup(payload),
            input: input,
            op: "nfdPreprocessForLookup",
            config: nil,
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

    /// Lazy-init cache for `Method::GetToneVariations`. Swift `static let`
    /// initializer is dispatch_once-equivalent — thread-safe by construction.
    // 中文: 調符變體表的延遲初始化快取 — 首次存取時才從 Rust 拉資料。
    // 中文: Swift 的 static let 初始化等同 dispatch_once,天然 thread-safe。
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
            tl: r.tlVariations.mapValues { $0.variations },
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

    // MARK: TPS (5 ops)

    public static func containsTPS(_ text: String) -> Bool {
        var payload = Taigi_Engine_ContainsTps()
        payload.text = text
        return boolDispatch(method: .containsTps(payload), op: "containsTps")
    }

    public static func tlNumericToTPS(_ text: String, orMapsToER: Bool) -> String {
        var payload = Taigi_Engine_TlNumericToTps()
        payload.text = text
        payload.orMapsToEr = orMapsToER
        return stringDispatch(
            method: .tlNumericToTps(payload),
            input: text,
            op: "tlNumericToTps",
            config: nil,
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
            config: nil,
        )
    }

    public static func isTPSToneMark(_ char: Character) -> Bool {
        var payload = Taigi_Engine_IsTpsToneMark()
        payload.char = String(char)
        return boolDispatch(method: .isTpsToneMark(payload), op: "isTpsToneMark")
    }

    public static func tpsInputAdjust(
        incoming: String,
        rawInput: String,
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
    /// Cold-start callers that lack a connected user-frequency DB pass
    /// `mergeOrderOnly: true` so engine dedup runs without scoring +
    /// sorting — the score-sort is deterministic but reorders candidates
    /// against the legacy iOS "merged-order on cold-start"
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
    // 中文: 候選詞排序管線 — dedup → score → sort → 可選 TPS display-dedup,全在 Rust 端 atomic 執行。
    // 中文: tpsDedupEnabled 由平台決定(看是否為 TPS layout),不從 AppConfig 推導。
    // 中文: nowMs 由 caller 提供,讓 recency 視窗運算在測試中可重現。
    // 中文: DEBUG 模式會額外要求 ScoreBreakdown 並寫入 log,Release 跳過該欄位節省序列化成本。
    public static func processCandidates(
        raw: [TaigiWord],
        normalizedInput: String,
        tpsDedupEnabled: Bool,
        frequencyData: [String: FrequencyData],
        nowMs: Int64,
        mergeOrderOnly: Bool = false,
    ) -> [TaigiWord] {
        #if DEBUG
            let detailed = processCandidatesDetailed(
                raw: raw,
                normalizedInput: normalizedInput,
                tpsDedupEnabled: tpsDedupEnabled,
                frequencyData: frequencyData,
                nowMs: nowMs,
                includeBreakdown: true,
                mergeOrderOnly: mergeOrderOnly,
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
                mergeOrderOnly: mergeOrderOnly,
            ).ranked
        #endif
    }

    /// Per-candidate score breakdown returned alongside `ranked` when the
    /// caller opts in. Six fields sum to the engine's sort key.
    // 中文: 單一候選詞的分數細項。六個欄位加總即引擎的 sort key。
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
    // 中文: 排序管線的複合回傳值。Production 通常只用 ranked,測試用 breakdowns 鎖住分數運算。
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
    // 中文: 測試專用接口 — 與 processCandidates 同一條 FFI 呼叫,但會回傳分數細項。
    // 中文: 給 RustEngineBridgeRankingTests 鎖住引擎側的分數算法,Production 用上面那個版本。
    public static func processCandidatesDetailed(
        raw: [TaigiWord],
        normalizedInput: String,
        tpsDedupEnabled: Bool,
        frequencyData: [String: FrequencyData],
        nowMs: Int64,
        includeBreakdown: Bool,
        mergeOrderOnly: Bool = false,
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
        payload.mergeOrderOnly = mergeOrderOnly

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

    /// Raw-list fallback on the FFI error path. When the Rust lexicon
    /// dispatch fails (encode error, decode error, non-OK engine
    /// response, or missing payload variant), return the input list
    /// unchanged. Simplified in the v3.5.3 follow-up (PR #192) —
    /// previously this delegated to the Swift
    /// `CandidateProcessor.removeDuplicates` /
    /// `removeDisplayDuplicates` helpers as defense-in-depth dedup, but
    /// that silently masked Rust dispatch bugs. `tpsDedupEnabled` is
    /// kept on the signature for caller-shape parity with the Android
    /// mirror (Codex audit § 1 Q3).
    // 中文: FFI 失敗時的 fallback — 直接回傳原始清單。tpsDedupEnabled 參數保留,
    // 中文: 純粹是為了與 Android 的 caller shape 對齊。
    private static func fallbackRanked(raw: [TaigiWord], tpsDedupEnabled _: Bool) -> [TaigiWord] {
        raw
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
            sourceBitmask: proto.hasSourceBitmask ? UInt16(truncatingIfNeeded: proto.sourceBitmask) : nil,
        )
    }

    // MARK: Composing slice (12 ops) — v3.5.4

    /// Bridge-synthesized companion to the proto `ComposingResponse`.
    /// Consumed by `ComposingManager` and its delegate.
    // 中文: 對應 proto ComposingResponse 的 Swift 端 struct,由 bridge 解碼後組成。
    // 中文: ComposingManager 與其 delegate 用這個型別決定要對輸入框做什麼動作。
    public struct ComposingTransition: Equatable {
        public enum Effect: Equatable {
            case updatePreedit(String)
            case clearPreeditWithoutCommit
            case commitTextReplacingPreedit(String)
            case deleteBackwardFromDocument
            case resetAutocomplete
            case performAutocomplete
            case resetAutocompleteContext
        }

        public let rawInput: String
        public let displayText: String
        public let effects: [Effect]
        public let selectedCandidateIndex: Int
        public let isComposing: Bool

        public static let noop = ComposingTransition(
            rawInput: "",
            displayText: "",
            effects: [],
            selectedCandidateIndex: -1,
            isComposing: false,
        )
    }

    public static func composingStart(
        _ text: String,
        mode: InputMode,
        toggles: ToneToggles,
        generation: UInt64,
    ) -> ComposingTransition {
        var payload = Taigi_Engine_Start()
        payload.text = text
        return composingDispatch(
            method: .start(payload),
            op: "composingStart",
            generation: generation,
            config: appConfig(mode: mode, toggles: toggles),
        )
    }

    public static func composingAppend(
        _ char: String,
        mode: InputMode,
        toggles: ToneToggles,
        generation: UInt64,
    ) -> ComposingTransition {
        var payload = Taigi_Engine_Append()
        payload.char = char
        return composingDispatch(
            method: .append(payload),
            op: "composingAppend",
            generation: generation,
            config: appConfig(mode: mode, toggles: toggles),
        )
    }

    public static func composingAppendHyphen(
        mode: InputMode,
        toggles: ToneToggles,
        generation: UInt64,
    ) -> ComposingTransition {
        composingDispatch(
            method: .appendHyphen(Taigi_Engine_AppendHyphen()),
            op: "composingAppendHyphen",
            generation: generation,
            config: appConfig(mode: mode, toggles: toggles),
        )
    }

    public static func composingReplaceLast(
        _ replacement: String,
        mode: InputMode,
        toggles: ToneToggles,
        generation: UInt64,
    ) -> ComposingTransition {
        var payload = Taigi_Engine_ReplaceLast()
        payload.replacement = replacement
        return composingDispatch(
            method: .replaceLast(payload),
            op: "composingReplaceLast",
            generation: generation,
            config: appConfig(mode: mode, toggles: toggles),
        )
    }

    public static func composingDeleteBackward(
        mode: InputMode,
        toggles: ToneToggles,
        generation: UInt64,
    ) -> ComposingTransition {
        composingDispatch(
            method: .deleteBackward(Taigi_Engine_DeleteBackward()),
            op: "composingDeleteBackward",
            generation: generation,
            config: appConfig(mode: mode, toggles: toggles),
        )
    }

    public static func composingCommitDerived(
        mode: InputMode,
        toggles: ToneToggles,
        generation: UInt64,
    ) -> ComposingTransition {
        composingDispatch(
            method: .commitDerived(Taigi_Engine_CommitDerived()),
            op: "composingCommitDerived",
            generation: generation,
            config: appConfig(mode: mode, toggles: toggles),
        )
    }

    public static func composingCommitRaw(generation: UInt64) -> ComposingTransition {
        composingDispatch(
            method: .commitRaw(Taigi_Engine_CommitRaw()),
            op: "composingCommitRaw",
            generation: generation,
            config: nil,
        )
    }

    public static func composingSelectSuggestion(
        _ text: String,
        generation: UInt64,
    ) -> ComposingTransition {
        var payload = Taigi_Engine_SelectSuggestion()
        payload.text = text
        return composingDispatch(
            method: .selectSuggestion(payload),
            op: "composingSelectSuggestion",
            generation: generation,
            config: nil,
        )
    }

    public static func composingCommitPreeditThenInsertExternal(
        _ text: String,
        mode: InputMode,
        toggles: ToneToggles,
        generation: UInt64,
    ) -> ComposingTransition {
        var payload = Taigi_Engine_CommitPreeditThenInsertExternal()
        payload.text = text
        return composingDispatch(
            method: .commitPreeditThenInsertExternal(payload),
            op: "composingCommitPreeditThenInsertExternal",
            generation: generation,
            config: appConfig(mode: mode, toggles: toggles),
        )
    }

    public static func composingReset(generation: UInt64) -> ComposingTransition {
        composingDispatch(
            method: .reset(Taigi_Engine_Reset()),
            op: "composingReset",
            generation: generation,
            config: nil,
        )
    }

    public static func composingSetSelectedCandidateIndex(
        _ index: Int,
        generation: UInt64,
    ) -> ComposingTransition {
        var payload = Taigi_Engine_SetSelectedCandidateIndex()
        payload.index = Int32(index)
        return composingDispatch(
            method: .setSelectedCandidateIndex(payload),
            op: "composingSetSelectedCandidateIndex",
            generation: generation,
            config: nil,
        )
    }

    public static func composingQueryState(generation: UInt64) -> ComposingTransition {
        composingDispatch(
            method: .queryState(Taigi_Engine_QueryState()),
            op: "composingQueryState",
            generation: generation,
            config: nil,
        )
    }

    // MARK: Diagnostics (Codex v2 §8 / v3 §7)

    // 中文: 單筆診斷紀錄 — 失敗時的時間、op 名稱、錯誤碼與訊息。
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
    // 中文: 唯讀的診斷快照 — 給 debug 選單與測試看。計數器在所有 fallback
    // 中文: 路徑都會 +1,最近紀錄 ring buffer 上限 32 筆。
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
    /// `internal` (default) so the v3.5.5 `RustEngineBridge+NextWord`
    /// extension file can share the request-id sequence.
    static func nextRequestID() -> UInt32 {
        idLock.lock()
        defer { idLock.unlock() }
        nextID &+= 1
        return nextID
    }

    private static let diagnosticsLock = NSLock()
    private static var failureCounter: UInt32 = 0
    private static var recentErrorBuffer: [DiagnosticsEntry] = []

    private static let recentErrorCap = 32

    /// `internal` (default) so the v3.5.5 `RustEngineBridge+NextWord`
    /// extension file can route bridge failures through the same
    /// counter + bounded ring-buffer.
    static func recordFailure(op: String, message: String, code: Int32 = -1) {
        diagnosticsLock.lock()
        defer { diagnosticsLock.unlock() }
        failureCounter &+= 1
        let entry = DiagnosticsEntry(
            timestamp: Date(),
            op: op,
            errorCode: code,
            message: message,
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
        config: Taigi_Engine_AppConfig?,
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
            serializedBytes: Data(responseBytes),
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
        config: Taigi_Engine_AppConfig?,
    ) -> String {
        guard let resp = dispatch(method: method, op: op, config: config) else { return input }
        guard case let .stringResult(s)? = resp.result else {
            recordFailure(op: op, message: "expected StringResult")
            return input
        }
        return s.output
    }

    /// Case-transform dispatch — single FFI hop per word/char. Mode is
    /// carried via envelope `AppConfig.input_mode` (engine reads it for
    /// tone-table lookup). No `ToneToggles` needed: case-transform is
    /// independent of POJ doubletap preprocessing.
    static func caseDispatch(
        method: Taigi_Engine_CaseRequest.OneOf_Method,
        op: String,
        mode: InputMode,
    ) -> Taigi_Engine_CaseResponse? {
        var caseReq = Taigi_Engine_CaseRequest()
        caseReq.method = method

        var request = Taigi_Engine_Request()
        request.id = nextRequestID()
        request.payload = .caseTransform(caseReq)
        // Case-transform is independent of POJ doubletap preprocessing —
        // pass an explicit "all-off" snapshot so the engine `AppConfig`
        // doesn't accidentally pick up unrelated state.
        request.configSnapshot = appConfig(
            mode: mode,
            toggles: ToneToggles(isDoubleTapOOEnabled: false, isDoubleTapNNEnabled: false),
        )

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
        guard case let .caseTransform(payload) = response.payload else {
            recordFailure(op: op, message: "missing case payload")
            return nil
        }
        return payload
    }

    static func lexiconDispatch(
        method: Taigi_Engine_LexiconRequest.OneOf_Method,
        op: String,
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
            serializedBytes: Data(responseBytes),
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

    private static func composingDispatch(
        method: Taigi_Engine_ComposingRequest.OneOf_Method,
        op: String,
        generation: UInt64,
        config: Taigi_Engine_AppConfig?,
    ) -> ComposingTransition {
        let logger = LoggerFactory.make(category: "RustEngineBridge")
        var composing = Taigi_Engine_ComposingRequest()
        composing.method = method

        var request = Taigi_Engine_Request()
        request.id = nextRequestID()
        request.generation = generation
        request.payload = .composing(composing)
        if let config { request.configSnapshot = config }

        let bytes: [UInt8]
        do {
            bytes = try Array(request.serializedData())
        } catch {
            recordFailure(op: op, message: "encode failed: \(error)")
            return .noop
        }

        logger.debug("[FFI->] fn=composingDispatch op=\(op) id=\(request.id) generation=\(generation)")
        let responseBytes = bytes.withUnsafeBufferPointer { buf in
            process_request_bytes(buf).toArray()
        }
        guard let response = try? Taigi_Engine_Response(
            serializedBytes: Data(responseBytes),
        ) else {
            recordFailure(op: op, message: "response decode failed")
            return .noop
        }
        guard response.error == .ok else {
            recordFailure(op: op, message: "engine returned \(response.error)", code: Int32(response.error.rawValue))
            return .noop
        }
        guard case let .composing(payload) = response.payload else {
            recordFailure(op: op, message: "missing composing payload")
            return .noop
        }
        let transition = synthComposing(payload)
        logger.debug("[FFI<-] fn=composingDispatch op=\(op) id=\(request.id) effects=\(transition.effects.count) composing=\(transition.isComposing)")
        return transition
    }

    private static func synthComposing(_ proto: Taigi_Engine_ComposingResponse) -> ComposingTransition {
        let effects: [ComposingTransition.Effect] = proto.effect.compactMap { eff -> ComposingTransition.Effect? in
            guard let kind = eff.kind else { return nil }
            switch kind {
            case let .updatePreedit(m): return .updatePreedit(m.display)
            case .clearPreeditWithoutCommit_p: return .clearPreeditWithoutCommit
            case let .commitTextReplacingPreedit(m): return .commitTextReplacingPreedit(m.text)
            case .deleteBackwardFromDocument: return .deleteBackwardFromDocument
            case .resetAutocomplete: return .resetAutocomplete
            case .performAutocomplete: return .performAutocomplete
            case .resetAutocompleteContext: return .resetAutocompleteContext
            case .nextWordUpdateLastSelectedWord,
                 .nextWordWordSelected,
                 .nextWordClearForNewComposing:
                // TODO(Phase 7/8): dispatch as NextWordRequest with platform-injected now_ms.
                // Phase 4 (PR #253) ships only the engine-side state machine; the
                // continuous-input candidate strip + nextword wiring lands when the
                // platform UI does. Emitted here intentionally so the engine's effect
                // contract stays exhaustive on the Swift side.
                return nil
            }
        }
        return ComposingTransition(
            rawInput: proto.preedit.rawInput,
            displayText: proto.preedit.displayText,
            effects: effects,
            selectedCandidateIndex: Int(proto.selectedCandidateIndex),
            isComposing: proto.isComposing,
        )
    }

    private static func boolDispatch(
        method: Taigi_Engine_PhoneticsRequest.OneOf_Method,
        op: String,
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
// 中文: 長按 callout 用的調符變體表快取。首次存取時一次拉完 POJ + TL 兩張表,攤提 FFI 成本。
public struct ToneVariationsCache {
    public let poj: [String: [String]]
    public let tl: [String: [String]]
}

// MARK: - Logger sink

// 中文: Swift 端實作的 log sink,讓 Rust 的 log!/warn!/error! 都流回平台 LoggerBackend。
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

/// `internal` (default) so v3.5.5 `RustEngineBridge+NextWord` can decode
/// the FFI byte buffer the same way as the in-file Composing slice.
extension RustVec where T == UInt8 {
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
