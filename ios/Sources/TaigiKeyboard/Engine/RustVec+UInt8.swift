// swift-bridge `RustVec<UInt8>` → Swift `[UInt8]` 的解碼 helper,FFI byte buffer 共用。

import Foundation

// MARK: - swift-bridge interop helpers

/// `internal` (default) so every `RustEngineBridge+*` extension can decode
/// the FFI byte buffer the same way.
// 預設 internal,所有 RustEngineBridge 切片擴充共用同一條 byte buffer 解碼路徑。
extension RustVec where T == UInt8 {
    func toArray() -> [UInt8] {
        let count = Int(len())
        var out = [UInt8]()
        out.reserveCapacity(count)
        for i in 0 ..< count {
            out.append(get(index: UInt(i)) ?? 0)
        }
        return out
    }
}
