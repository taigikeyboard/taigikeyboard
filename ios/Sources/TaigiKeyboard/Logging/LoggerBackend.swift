// shared-core 候選邏輯使用的記錄器抽象。
// iOS 端由 DebugLogger 實作(DEBUG 走 OSLog,Release no-op);
// 訊息以 @autoclosure 傳遞,確保 backend 真的需要時才組字串。

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
// shared-core 使用的 logger protocol;訊息走 @autoclosure 避免無謂 string formatting。
protocol LoggerBackend: Sendable {
    func debug(_ message: @autoclosure () -> String)
    func info(_ message: @autoclosure () -> String)
    func warning(_ message: @autoclosure () -> String)
    func error(_ message: @autoclosure () -> String)
}

/// No-op backend used as the factory default before the platform backend
/// is installed, and for shared-core unit tests that do not exercise logging.
// 平台 backend 安裝前的預設 no-op;shared-core 單元測試不寫 log 時也用這個。
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
// LoggerBackend 的 thread-safe 工廠;每次呼叫 make() 都拿當前 factory,確保不會卡在過期 backend。
enum LoggerFactory {
    private static let lock = NSLock()
    private nonisolated(unsafe) static var factory: @Sendable (_ category: String) -> LoggerBackend = { _ in
        NullLoggerBackend()
    }

    /// Install the platform-specific backend factory. Idempotent.
    // 安裝平台特化的 backend factory;冪等。
    static func install(_ factory: @Sendable @escaping (_ category: String) -> LoggerBackend) {
        lock.withLock { self.factory = factory }
    }

    /// Create a logger for the given category using the currently installed
    /// factory (or `NullLoggerBackend` if none has been installed yet).
    // 用當前 factory 建立指定 category 的 logger;factory 還沒裝的話回 NullLoggerBackend。
    static func make(category: String) -> LoggerBackend {
        lock.withLock { factory(category) }
    }
}
