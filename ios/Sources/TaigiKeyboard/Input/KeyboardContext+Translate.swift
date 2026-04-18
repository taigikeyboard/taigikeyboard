import Combine
import KeyboardKit
import SwiftUI

/// KeyboardContext 翻譯狀態擴展
///
/// 提供漢字／羅馬字顯示模式切換功能。
public extension KeyboardContext {
    /// 是否為漢字優先模式（true = 漢字, false = 羅馬字）
    var isTranslateSwapped: Bool {
        get { SharedSettings.shared.isTranslateSwapped }
        set {
            SharedSettings.shared.isTranslateSwapped = newValue
            DispatchQueue.main.async {
                self.objectWillChange.send()
            }
        }
    }

    /// 切換顯示模式
    func toggleTranslateSwapped() {
        isTranslateSwapped.toggle()
    }
}
