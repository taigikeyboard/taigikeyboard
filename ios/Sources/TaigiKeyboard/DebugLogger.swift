import Foundation

#if DEBUG
    import OSLog

    enum TraceId {
        static let untraced = "untraced"

        private static let lock = NSLock()
        private static var seq: UInt64 = 0

        static func next() -> String {
            lock.lock()
            seq &+= 1
            let nextSeq = seq
            lock.unlock()

            let timestampMs = UInt64(Date().timeIntervalSince1970 * 1000)
            return "\(timestampMs)-\(nextSeq)"
        }
    }

    enum TraceContext {
        @TaskLocal static var current: String? = nil

        @discardableResult
        static func with<T>(_ id: String, _ body: () throws -> T) rethrows -> T {
            try $current.withValue(id) { try body() }
        }

        @discardableResult
        static func with<T>(_ id: String, _ body: () async throws -> T) async rethrows -> T {
            try await $current.withValue(id) { try await body() }
        }
    }

    /// Debug-only logger wrapper around os.Logger.
    ///
    /// All messages use `privacy: .public` for Xcode console visibility.
    /// In release builds, every method is a no-op and `@autoclosure` ensures
    /// the message string is never constructed.
    struct DebugLogger: LoggerBackend {
        private let logger: Logger

        init(category: String) {
            logger = Logger(
                subsystem: LexiconConstants.Logging.subsystem,
                category: category,
            )
        }

        func debug(_ message: @autoclosure () -> String) {
            let msg = Self.prefixed(message())
            logger.debug("\(msg, privacy: .public)")
        }

        func info(_ message: @autoclosure () -> String) {
            let msg = Self.prefixed(message())
            logger.info("\(msg, privacy: .public)")
        }

        func warning(_ message: @autoclosure () -> String) {
            let msg = Self.prefixed(message())
            logger.warning("\(msg, privacy: .public)")
        }

        func error(_ message: @autoclosure () -> String) {
            let msg = Self.prefixed(message())
            logger.error("\(msg, privacy: .public)")
        }

        private static func prefixed(_ message: String) -> String {
            guard let trace = TraceContext.current else { return message }
            return "[trace=\(trace)] \(message)"
        }
    }
#else
    enum TraceId {
        static let untraced = ""
        @inline(__always) static func next() -> String {
            ""
        }
    }

    enum TraceContext {
        static var current: String? {
            nil
        }

        @inline(__always) static func with<T>(_: String, _ body: () throws -> T) rethrows -> T {
            try body()
        }

        @inline(__always) static func with<T>(_: String, _ body: () async throws -> T) async rethrows -> T {
            try await body()
        }
    }

    struct DebugLogger: LoggerBackend {
        init(category _: String) {}
        @inline(__always) func debug(_: @autoclosure () -> String) {}
        @inline(__always) func info(_: @autoclosure () -> String) {}
        @inline(__always) func warning(_: @autoclosure () -> String) {}
        @inline(__always) func error(_: @autoclosure () -> String) {}
    }
#endif

/// v3.5.8 Phase 9 Bug 3 instrumentation (PR1, instrumentation-only — zero
/// behavior change; removed in the Bug 3 fix PR). Bounded snapshot of the
/// document text just before the cursor, used to pinpoint which call
/// finalizes a stale composing region between a mid-commit and the next
/// final-commit (`taiuantaigi` → tap 臺灣 → tap 台語 yields the spurious
/// "臺灣taigi台語"). Only ever evaluated inside `DebugLogger.debug`'s
/// `@autoclosure` (a no-op in release), and reports at most the last 16
/// characters so no aggregate user text is logged (`rules/security-rules.md`
/// §Logging). Mirrors Android `InputConnection.bug3Tail()`.
func bug3Tail(_ text: String?) -> String {
    let tail = String((text ?? "").suffix(16))
    return "len=\(tail.count) tail='\(tail)'"
}
