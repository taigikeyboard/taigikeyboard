import Foundation
import SwiftProtobuf

// MARK: - RustEngineBridge

/// Thin Swift wrapper around the Rust shared-core FFI exposed by
/// `engine/swift-ffi/src/lib.rs`.
///
/// In D9.2 this bridge is **not** wired into the IME runtime — production
/// phonetics conversion still flows through `PhoneticsConverter`. The bridge
/// exists so unit tests can prove the Rust `.xcframework` loads and produces
/// the right answer. It will replace `PhoneticsConverter` only after D9.4
/// proves INVARIANT_* parity.
///
/// Usage:
///
/// ```
/// RustEngineBridge.install(category: "RustEngine")
/// let poj = RustEngineBridge.tlToPoj("gua2") // → "góa"
/// ```
public enum RustEngineBridge {
    private static let installLock = NSLock()
    private static var installed = false

    /// Idempotent. Registers a logger sink that forwards every Rust
    /// `log::warn!` (and friends) into the platform `LoggerBackend` via
    /// `LoggerFactory.make(category:)`. The category comes from the Rust
    /// `log::Record.target()`, not from a fixed install-time value.
    public static func install() {
        installLock.lock()
        defer { installLock.unlock() }
        guard !installed else { return }
        install_logger_sink(SwiftLoggerSink())
        installed = true
    }

    public static func tlToPoj(_ input: String) -> String {
        sendPhonetics(op: .tlToPoj, input: input)
    }

    public static func pojToTl(_ input: String) -> String {
        sendPhonetics(op: .pojToTl, input: input)
    }

    public static func normalizeTone(_ input: String) -> String {
        sendPhonetics(op: .normalizeTone, input: input)
    }

    public static func stripTone(_ input: String) -> String {
        sendPhonetics(op: .stripTone, input: input)
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
    /// linked. Returns the encoded `FAIL_INTERNAL` response from the catch
    /// arm without crashing the test process.
    static func panicForTestRaw() -> Taigi_Engine_Response? {
        let responseBytes = panic_for_test().toArray()
        return try? Taigi_Engine_Response(serializedBytes: Data(responseBytes))
    }

    // MARK: Private dispatch

    private static func sendPhonetics(
        op: Taigi_Engine_PhoneticsRequest.Op,
        input: String
    ) -> String {
        var phonetics = Taigi_Engine_PhoneticsRequest()
        phonetics.op = op
        phonetics.input = input

        var request = Taigi_Engine_Request()
        request.id = nextRequestID()
        request.payload = .phonetics(phonetics)

        let bytes: [UInt8]
        do {
            bytes = try Array(request.serializedData())
        } catch {
            return input
        }

        let responseBytes = bytes.withUnsafeBufferPointer { buf in
            process_request_bytes(buf).toArray()
        }
        guard let response = try? Taigi_Engine_Response(
            serializedBytes: Data(responseBytes)
        ) else {
            return input
        }
        guard response.error == .ok, case let .phonetics(payload) = response.payload else {
            return input
        }
        return payload.output
    }

    private static let idLock = NSLock()
    private static var nextID: UInt32 = 0
    private static func nextRequestID() -> UInt32 {
        idLock.lock()
        defer { idLock.unlock() }
        nextID &+= 1
        return nextID
    }
}

// MARK: - Logger sink

/// swift-bridge generates a base class `SwiftLoggerSink` from the
/// `extern "Swift" { type SwiftLoggerSink; }` block in `engine/swift-ffi/src/lib.rs`.
/// The Rust side calls `log(level:category:message:)` with `RustString` for
/// the two string parameters; we convert to Swift `String` and forward to
/// the platform `LoggerBackend` registered through `LoggerFactory`.
///
/// Must be `public` because the swift-bridge generated `install_logger_sink(_:)`
/// in `ios/RustEngine/RustTaigi.swift` is declared `public` and takes this
/// type as parameter — Swift forbids a public API exposing an internal type.
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
        case Self.levelError:
            backend.error(text)
        case Self.levelWarn:
            backend.warning(text)
        case Self.levelInfo:
            backend.info(text)
        default:
            backend.debug(text)
        }
    }
}

// MARK: - swift-bridge interop helpers

private extension RustVec where T == UInt8 {
    /// O(n) element-wise copy. Acceptable in D9.2 because the bridge is
    /// invoked only from unit tests; production keystroke path stays on
    /// `PhoneticsConverter`. When the engine takes the IME hot path in
    /// D9.4+, switch to a bulk `as_ptr()` + `Data(bytes:count:)` copy so a
    /// 2 MB response is a single memcpy instead of 2M virtual calls.
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
