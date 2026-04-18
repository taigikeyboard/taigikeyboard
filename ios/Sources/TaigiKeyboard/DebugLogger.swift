import Foundation

#if DEBUG
    import OSLog

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
            let msg = message()
            logger.debug("\(msg, privacy: .public)")
        }

        func info(_ message: @autoclosure () -> String) {
            let msg = message()
            logger.info("\(msg, privacy: .public)")
        }

        func warning(_ message: @autoclosure () -> String) {
            let msg = message()
            logger.warning("\(msg, privacy: .public)")
        }

        func error(_ message: @autoclosure () -> String) {
            let msg = message()
            logger.error("\(msg, privacy: .public)")
        }
    }
#else
    struct DebugLogger: LoggerBackend {
        init(category _: String) {}
        @inline(__always) func debug(_: @autoclosure () -> String) {}
        @inline(__always) func info(_: @autoclosure () -> String) {}
        @inline(__always) func warning(_: @autoclosure () -> String) {}
        @inline(__always) func error(_: @autoclosure () -> String) {}
    }
#endif
