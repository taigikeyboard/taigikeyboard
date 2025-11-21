import Foundation
import KeyboardKit
import ObjectiveC
import SwiftUI

/// 擴展 KeyboardContext 以支援組字狀態
extension KeyboardContext {
    /// 關聯對象的 key
    private static var isComposingTextKey: UInt8 = 0

    /// 是否正在組字中
    var isComposingText: Bool {
        get {
            objc_getAssociatedObject(self, &Self.isComposingTextKey) as? Bool ?? false
        }
        set {
            let currentValue = isComposingText
            if newValue != currentValue {
                objc_setAssociatedObject(self, &Self.isComposingTextKey, newValue, .OBJC_ASSOCIATION_RETAIN_NONATOMIC)

                // 在主執行緒觸發更新，關閉動畫
                DispatchQueue.main.async { [weak self] in
                    var transaction = Transaction()
                    transaction.disablesAnimations = true
                    withTransaction(transaction) {
                        self?.objectWillChange.send()
                    }
                }
            }
        }
    }
}
