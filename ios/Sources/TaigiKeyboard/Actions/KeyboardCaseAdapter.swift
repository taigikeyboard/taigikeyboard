// 中文: KeyboardKit Keyboard.KeyboardCase 與 Rust bridge CaseTransformLetterCase 之間的轉接器。
// 中文: 隔離 KeyboardKit 型別,讓引擎側不會直接接觸 Keyboard.KeyboardCase。

import Foundation
import KeyboardKit

/// Adapter bridging KeyboardKit's `Keyboard.KeyboardCase` to the
/// `RustEngineBridge.CaseTransformLetterCase` carried over the FFI seam.
/// Lives in `Actions/` because that's where KK types are allowed; the
/// engine side never sees `Keyboard.KeyboardCase`.
// 中文: KeyboardCase → CaseTransformLetterCase 的對映擴充。
extension Keyboard.KeyboardCase {
    // 中文: 把 KK 的鍵盤大小寫狀態映射成 bridge 端的 LetterCase 三態。
    var asLetterCase: RustEngineBridge.CaseTransformLetterCase {
        switch self {
        case .capsLocked: .capsLocked
        case .uppercased: .uppercased
        case .lowercased: .lowercased
        @unknown default: .lowercased
        }
    }
}
