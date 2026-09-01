// 中文: Rust shared-core FFI 的 Swift 端薄包裝層 — 核心檔。
// 中文: 對應 engine/swift-ffi/src/lib.rs;Composing / NextWord / Lexicon /
// 中文: Phonetics / CaseTransform 等切片各自有獨立的 RustEngineBridge+*.swift extension 檔。
// 中文: 本檔僅保留跨切片共用的 install / 診斷 / 請求 ID / appConfig / 測試 seam。

import Foundation
import SwiftProtobuf

// MARK: - RustEngineBridge core

/// Thin Swift wrapper around the Rust shared-core FFI exposed by
/// `engine/swift-ffi/src/lib.rs`. This core file holds the cross-slice
/// infrastructure: log-sink install, the in-memory diagnostics ring
/// buffer, the request-id counter, the shared `AppConfig` builder, and
/// the bytes / panic test seams.
///
/// Per-slice surfaces live in dedicated extension files in this same
/// directory:
///
/// - `RustEngineBridge+Phonetics.swift` — 8 phonetics + 2 derivation + 5
///   TPS ops + lazy `toneVariations` cache + `ToneVariationsCache`.
/// - `RustEngineBridge+Composing.swift` — 12 composing + 4 continuous
///   ops + `ComposingTransition` / `CandidateMode` / `ContinuousCandidate`
///   / `ContinuousFetchResult` synthesized types.
/// - `RustEngineBridge+Lexicon.swift` — install / search / assoc /
///   classify-input / dictionary-filters / isHanzi reads + the
///   `processCandidates` ranking surface.
/// - `RustEngineBridge+NextWord.swift` — 6 decide intents + filter /
///   boost / queryState + setIsShowing.
/// - `RustEngineBridge+CaseTransform.swift` — per-char / per-word case
///   transforms.
///
/// Retired-op history available via git log on
/// `engine/protos/proto/phonetics.proto`.
///
/// Per `~/.claude/rules/round-workflow.md` § Codex review sandwich: every method
/// requiring AppConfig (currently NormalizeTone for POJ preprocessing)
/// takes the necessary fields as mandatory parameters — no global default.
///
/// Per Codex v2 §8 + v3 §7: error visibility is hardened. DEBUG asserts on
/// failure; release returns a graceful fallback + logs + records a
/// structured `DiagnosticsEntry` in a bounded in-memory queue accessible
/// via `diagnostics()` for dogfood inspection.
// 中文: Rust 引擎橋接層核心。每個切片(phonetics / composing / lexicon /
// 中文: nextword / case-transform)獨立 extension 檔。AppConfig 採每呼叫顯式傳入策略,不留全域預設。
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

    // MARK: Cross-slice helpers (request ID + AppConfig + diagnostics)

    private static let idLock = NSLock()
    private static var nextID: UInt32 = 0
    /// `internal` (default) so every `RustEngineBridge+*` extension file
    /// can share the request-id sequence.
    // 中文: 預設 internal,所有 RustEngineBridge 切片共用一條 request id 序列。
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

    /// `internal` (default) so every `RustEngineBridge+*` extension file
    /// can route bridge failures through the same counter +
    /// bounded ring-buffer.
    // 中文: 預設 internal,所有 RustEngineBridge 切片走同一條失敗計數 + ring buffer。
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

    /// Shared `AppConfig` builder. `internal` because three extension
    /// files (`+Phonetics`, `+Composing`, `+CaseTransform`) build their
    /// envelopes on top of it. Continuous-rendering callers wrap this
    /// with `continuousAppConfig` (private to `+Composing.swift`) to
    /// add the §10.2 word-boundary spacing flags.
    ///
    /// Proto field 9 (`candidateDisplayMode`) is set by the two builders whose
    /// requests the engine reads it on — `continuousAppConfig` and
    /// `nextwordConfig` — not here (mirrors Android).
    // 中文: 跨切片共用的 AppConfig builder。internal 因 +Phonetics / +Composing /
    // 中文: +CaseTransform 都會在其上組裝 envelope。連續輸入渲染用 continuousAppConfig 包裹。
    static func appConfig(mode: InputMode, toggles: ToneToggles) -> Taigi_Engine_AppConfig {
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
}

// Shared by `appConfig` and `nextwordConfig` (`+NextWord.swift`) — one mapping, never two.
extension CandidateDisplayMode {
    /// Explicit proto enum (never `.unspecified`) so the engine's single
    /// normalization helper sees the platform's actual choice.
    var engineValue: Taigi_Engine_CandidateDisplayMode {
        switch self {
        case .sideBySide: .sideBySide
        case .romanOnly: .romanOnly
        }
    }
}
