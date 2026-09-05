import Foundation

// MARK: - swift-bridge interop helpers

/// `internal` (default) so every `RustEngineBridge+*` extension can decode
/// the FFI byte buffer the same way.
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
