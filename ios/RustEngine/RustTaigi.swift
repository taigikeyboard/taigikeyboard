import RustTaigi

public func process_request_bytes(_ bytes: UnsafeBufferPointer<UInt8>) -> RustVec<UInt8> {
    RustVec(ptr: __swift_bridge__$process_request_bytes(bytes.toFfiSlice()))
}
public func install_logger_sink(_ sink: SwiftLoggerSink) {
    __swift_bridge__$install_logger_sink(Unmanaged.passRetained(sink).toOpaque())
}
public func panic_for_test() -> RustVec<UInt8> {
    RustVec(ptr: __swift_bridge__$panic_for_test())
}
@_cdecl("__swift_bridge__$SwiftLoggerSink$log")
func __swift_bridge__SwiftLoggerSink_log (_ this: UnsafeMutableRawPointer, _ level: UInt8, _ category: UnsafeMutableRawPointer, _ message: UnsafeMutableRawPointer) {
    Unmanaged<SwiftLoggerSink>.fromOpaque(this).takeUnretainedValue().log(level: level, category: RustString(ptr: category), message: RustString(ptr: message))
}


@_cdecl("__swift_bridge__$SwiftLoggerSink$_free")
func __swift_bridge__SwiftLoggerSink__free (ptr: UnsafeMutableRawPointer) {
    let _ = Unmanaged<SwiftLoggerSink>.fromOpaque(ptr).takeRetainedValue()
}



