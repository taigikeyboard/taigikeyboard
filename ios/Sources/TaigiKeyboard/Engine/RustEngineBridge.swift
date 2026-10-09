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
/// - `RustEngineBridge+Composing.swift` — composing + continuous-input
///   ops + `ComposingTransition` / `ContinuousCandidate`
///   / `ContinuousFetchResult` synthesized types.
/// - `RustEngineBridge+Lexicon.swift` — install / search / assoc /
///   dictionary-filters / isHanji reads.
/// - `RustEngineBridge+NextWord.swift` — 6 decide intents + setPredictionsVisible
///   + filter.
/// - `RustEngineBridge+CaseTransform.swift` — per-char / per-word case
///   transforms.
///
/// Retired-op history available via git log on
/// `engine/protos/proto/phonetics.proto`.
///
/// Every method requiring AppConfig (composing, case transform) takes the
/// necessary fields as mandatory parameters — no global default.
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
            set_log_level(4) // 4 = Debug per set_log_level Rust-side mapping (engine/swift-ffi)
        #endif
        installed = true
    }

    // MARK: Diagnostics (Codex v2 §8 / v3 §7)

    /// One diagnostics record: timestamp, op name, error code, and message on failure.
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

    // MARK: FFI dispatch

    /// Encode `request`, cross the FFI seam, and decode the reply. Returns
    /// `nil` (recording the failure under `op`) when the encode or the decode
    /// fails; callers still check `response.error` and the payload variant.
    static func send(_ request: Taigi_Engine_Request, op: String) -> Taigi_Engine_Response? {
        guard let bytes = encodeRequest(request, op: op) else { return nil }
        return dispatch(bytes, op: op)
    }

    /// Serialize `request`; `nil` (recorded under `op`) when encoding fails.
    /// Split from `send` so a caller can log between encode and dispatch.
    static func encodeRequest(_ request: Taigi_Engine_Request, op: String) -> [UInt8]? {
        do {
            return try Array(request.serializedData())
        } catch {
            recordFailure(op: op, message: "encode failed: \(error)")
            return nil
        }
    }

    /// Cross the FFI seam with already-encoded `bytes` and decode the reply;
    /// `nil` (recorded under `op`) when the reply does not decode.
    static func dispatch(_ bytes: [UInt8], op: String) -> Taigi_Engine_Response? {
        let responseBytes = bytes.withUnsafeBufferPointer { buf in
            process_request_bytes(buf).toArray()
        }
        guard let response = try? Taigi_Engine_Response(
            serializedBytes: Data(responseBytes),
        ) else {
            recordFailure(op: op, message: "response decode failed")
            return nil
        }
        return response
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

    /// The one `AppConfig` builder: every request this bridge sends carries a
    /// config built here, so the mode mapping exists once.
    /// Composing passes the live settings (`continuousAppConfig`), nextword
    /// the swap (plus the display fields on its predict request), case
    /// transform the nasal-marker switch; a field a request family does not
    /// read keeps its proto default. `pojMarkers == nil` leaves the three POJ
    /// marker fields at theirs (no folds, the marker follows the case).
    ///
    /// TPS goes out as `"tps"` with the swap and Syllable Separator exactly as the
    /// settings hold them: the engine applies the TPS fold itself
    /// (`AppConfig::renders_hanji_first` / `rendered_syllable_joiner`,
    /// `engine/protos/src/lib.rs`).
    // CROSS-PLATFORM INVARIANT — mirrors android/app/src/main/java/com/siansiansu/taigikeyboard/engine/EngineAppConfig.kt appConfig.
    // Drift causes silent divergence (one platform renders TPS or the swap differently).
    static func appConfig(
        mode: InputMode,
        pojMarkers: PojMarkerOptions? = nil,
        isHanjiFirst: Bool = false,
        isOutputBothScripts: Bool = false,
        candidateDisplayMode: CandidateDisplayMode = .sideBySide,
        syllableSeparator: SyllableSeparator = .hyphen,
        isTpsOrMappedToER: Bool = false,
    ) -> Taigi_Engine_AppConfig {
        var cfg = Taigi_Engine_AppConfig()
        // The raw values are the engine's `input_mode` strings.
        cfg.inputMode = mode.rawValue
        if let pojMarkers {
            cfg.ooDoubletapEnabled = pojMarkers.isDoubleTapOOEnabled
            cfg.nnDoubletapEnabled = pojMarkers.isDoubleTapNNEnabled
            // Inverted on the wire (proto default = the marker follows the case, §53).
            cfg.forceLowercaseNasalMarker = !pojMarkers.isNasalMarkerUppercaseEnabled
        }
        cfg.isHanjiFirst = isHanjiFirst
        cfg.outputBothScripts = isOutputBothScripts
        cfg.candidateDisplayMode = candidateDisplayMode.engineValue
        cfg.syllableSeparator = syllableSeparator.engineValue
        cfg.tpsOrMapsToEr = isTpsOrMappedToER
        return cfg
    }
}

extension SyllableSeparator {
    /// Explicit proto enum (never `.unspecified`), like `CandidateDisplayMode.engineValue`.
    var engineValue: Taigi_Engine_SyllableSeparator {
        switch self {
        case .hyphen: .hyphen
        case .space: .space
        case .noSeparator: .none
        }
    }
}

extension CandidateDisplayMode {
    /// Explicit proto enum (never `.unspecified`) so the engine's single
    /// normalization helper sees the platform's actual choice.
    var engineValue: Taigi_Engine_CandidateDisplayMode {
        switch self {
        case .sideBySide: .sideBySide
        case .romanOnly: .romanOnly
        case .combined: .combined
        }
    }
}
