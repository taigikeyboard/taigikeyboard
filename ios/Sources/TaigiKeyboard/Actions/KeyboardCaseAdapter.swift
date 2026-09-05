import Foundation
import KeyboardKit

/// Adapter bridging KeyboardKit's `Keyboard.KeyboardCase` to the
/// `RustEngineBridge.CaseTransformLetterCase` carried over the FFI seam.
/// Lives in `Actions/` because that's where KK types are allowed; the
/// engine side never sees `Keyboard.KeyboardCase`.
extension Keyboard.KeyboardCase {
    var asLetterCase: RustEngineBridge.CaseTransformLetterCase {
        switch self {
        case .capsLocked: .capsLocked
        case .uppercased: .uppercased
        case .lowercased: .lowercased
        @unknown default: .lowercased
        }
    }
}
