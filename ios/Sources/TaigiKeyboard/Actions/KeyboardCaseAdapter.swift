import Foundation
import KeyboardKit

/// Adapter bridging KeyboardKit's `Keyboard.KeyboardCase` to the engine-layer
/// `LetterCase`. Lives in `Actions/` because that's where KK types are
/// allowed; the engine side (`Input/CaseTransformer.swift`) stays Foundation-
/// only and never sees `Keyboard.KeyboardCase`.
extension Keyboard.KeyboardCase {
    var asLetterCase: LetterCase {
        switch self {
        case .capsLocked: .capsLocked
        case .uppercased: .uppercased
        case .lowercased: .lowercased
        @unknown default: .lowercased
        }
    }
}
