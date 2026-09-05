import Foundation

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.

/// Logging contract consumed by shared-core candidates.
///
/// Platform implementations conform: iOS uses `DebugLogger` (OSLog-backed
/// in DEBUG, no-op in release). Future Android shared-core extraction will
/// supply its own conformance.
///
/// Messages are passed as `@autoclosure` so the string is only constructed
/// when the backend actually does work (matches `os.Logger` ergonomics).
protocol LoggerBackend: Sendable {
    func debug(_ message: @autoclosure () -> String)
    func info(_ message: @autoclosure () -> String)
    func warning(_ message: @autoclosure () -> String)
    func error(_ message: @autoclosure () -> String)
}

/// No-op backend used as the factory default before the platform backend
/// is installed, and for shared-core unit tests that do not exercise logging.
struct NullLoggerBackend: LoggerBackend {
    @inline(__always) func debug(_: @autoclosure () -> String) {}
    @inline(__always) func info(_: @autoclosure () -> String) {}
    @inline(__always) func warning(_: @autoclosure () -> String) {}
    @inline(__always) func error(_: @autoclosure () -> String) {}
}

/// Thread-safe provider for `LoggerBackend` instances.
///
/// Shared-core candidates obtain loggers via `LoggerFactory.make(category:)`
/// at each log call (not cached), so the result always reflects the
/// currently installed factory. Before `install` is called the factory
/// produces `NullLoggerBackend`, so early callers are never persistently
/// locked to a stale backend.
///
/// The iOS app and keyboard extension install a `DebugLogger`-producing
/// factory via `LoggerFactory.install(_:)` at startup.
enum LoggerFactory {
    private static let lock = NSLock()
    private nonisolated(unsafe) static var factory: @Sendable (_ category: String) -> LoggerBackend = { _ in
        NullLoggerBackend()
    }

    /// Install the platform-specific backend factory. Idempotent.
    static func install(_ factory: @Sendable @escaping (_ category: String) -> LoggerBackend) {
        lock.withLock { self.factory = factory }
    }

    /// Create a logger for the given category using the currently installed
    /// factory (or `NullLoggerBackend` if none has been installed yet).
    static func make(category: String) -> LoggerBackend {
        lock.withLock { factory(category) }
    }
}
