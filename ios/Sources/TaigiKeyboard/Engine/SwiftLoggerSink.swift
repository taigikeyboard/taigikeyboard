import Foundation

// MARK: - SwiftLoggerSink

/// Swift-side implementation of the Rust log sink. Forwards every Rust
/// `log::warn!` / `log::error!` / `log::info!` / `log::debug!` call into
/// the platform `LoggerBackend`. Registered once by
/// `RustEngineBridge.install()` via `install_logger_sink`.
public final class SwiftLoggerSink {
    static let levelError: UInt8 = 0
    static let levelWarn: UInt8 = 1
    static let levelInfo: UInt8 = 2

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
