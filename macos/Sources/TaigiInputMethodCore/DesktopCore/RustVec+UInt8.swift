// swift-bridge `RustVec<UInt8>` → Swift `[UInt8]` decoding for FFI byte buffers.

import Foundation
import RustTaigiSwift

extension RustVec where T == UInt8 {
    /// Copies the Rust-owned buffer into Swift storage before this `RustVec`
    /// deinitializes and frees it. Bulk-copies through `as_ptr()` rather than
    /// the per-element `get(index:)` loop the iOS bridge uses, which costs one
    /// FFI call per byte; both produce the same bytes.
    func toArray() -> [UInt8] {
        [UInt8](UnsafeBufferPointer(start: as_ptr(), count: len()))
    }
}
