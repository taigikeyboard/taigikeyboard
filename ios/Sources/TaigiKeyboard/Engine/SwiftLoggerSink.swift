// Swift 端 log sink — 將 Rust `log!` / `warn!` / `error!` 巨集轉發到平台 LoggerBackend。

import Foundation

// MARK: - SwiftLoggerSink

/// Swift-side implementation of the Rust log sink. Forwards every Rust
/// `log::warn!` / `log::error!` / `log::info!` / `log::debug!` call into
/// the platform `LoggerBackend`. Registered once by
/// `RustEngineBridge.install()` via `install_logger_sink`.
// 安裝在 Rust 端的 log sink Swift 實作。RustEngineBridge.install() 透過 install_logger_sink 註冊一次。
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
        case Self.levelError: backend.error(text)
        case Self.levelWarn: backend.warning(text)
        case Self.levelInfo: backend.info(text)
        default: backend.debug(text)
        }
    }
}
