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
