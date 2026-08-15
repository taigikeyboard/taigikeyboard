// Debug-only logging wrapper. Release builds emit nothing.

#if DEBUG
    import Foundation
    import OSLog

    /// Matches `CFBundleIdentifier`, so `make log` filters the whole process on
    /// one subsystem. Info.plist stays the only place the real bundle ID is
    /// written; the fallback only ever applies under the test runner, which owns
    /// the main bundle, so it is deliberately not that ID.
    private let loggingSubsystem = Bundle.main.bundleIdentifier ?? "TaigiInputMethodTests"

    /// Wrapper around `os.Logger` that is a complete no-op in release builds,
    /// per `.claude/rules/security-rules.md` (release builds emit zero logs).
    /// Call sites therefore need no `#if DEBUG` of their own, and `@autoclosure`
    /// keeps the message string from being built at all in release.
    ///
    /// Mirrors the iOS `DebugLogger` contract (`ios/Sources/TaigiKeyboard/DebugLogger.swift`)
    /// minus its trace-id plumbing, which macOS has no caller for.
    /// The `let text = message()` in each method is required, not incidental:
    /// `os.Logger`'s string interpolation takes an *escaping* autoclosure, and
    /// a non-escaping parameter cannot be forwarded into one.
    struct DebugLogger {
        private let logger: Logger

        init(category: String) {
            logger = Logger(subsystem: loggingSubsystem, category: category)
        }

        func debug(_ message: @autoclosure () -> String) {
            let text = message()
            logger.debug("\(text, privacy: .public)")
        }

        func info(_ message: @autoclosure () -> String) {
            let text = message()
            logger.info("\(text, privacy: .public)")
        }

        func warning(_ message: @autoclosure () -> String) {
            let text = message()
            logger.warning("\(text, privacy: .public)")
        }

        func error(_ message: @autoclosure () -> String) {
            let text = message()
            logger.error("\(text, privacy: .public)")
        }
    }
#else
    struct DebugLogger {
        init(category _: String) {}
        @inline(__always) func debug(_: @autoclosure () -> String) {}
        @inline(__always) func info(_: @autoclosure () -> String) {}
        @inline(__always) func warning(_: @autoclosure () -> String) {}
        @inline(__always) func error(_: @autoclosure () -> String) {}
    }
#endif
