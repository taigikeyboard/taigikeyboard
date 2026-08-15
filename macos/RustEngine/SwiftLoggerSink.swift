// Swift-side sink for Rust `log::*` records crossing the swift-bridge seam.

import Foundation

/// Receives every Rust `log::Record` forwarded through `install_logger_sink`
/// and hands it to the closure the app installs.
///
/// Hand-written, despite living beside generated code — see README.md in this
/// directory for why it has to compile in this module.
///
/// It only converts and forwards: which levels are emitted, and the rule that
/// release builds log nothing, belong to the app's logger and cannot be reached
/// from this module, which sits below it.
public final class SwiftLoggerSink {
    /// `(level, category, message)`. Levels follow the Rust-side mapping in
    /// `engine/swift-ffi`: 0 error, 1 warning, 2 info, 3 debug, 4 trace.
    public typealias RecordHandler = (UInt8, String, String) -> Void

    private let handleRecord: RecordHandler

    public init(handleRecord: @escaping RecordHandler) {
        self.handleRecord = handleRecord
    }

    public func log(level: UInt8, category: RustString, message: RustString) {
        handleRecord(level, category.toString(), message.toString())
    }
}
