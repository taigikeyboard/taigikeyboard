import Foundation
import KeyboardKit
import ObjectiveC
import SwiftUI

/// KeyboardContext 組字狀態擴展
///
/// 使用 Associated Object 為 KeyboardContext 添加組字狀態屬性。
extension KeyboardContext {

    private static var isComposingTextKey: UInt8 = 0

    /// 是否正在組字中
    var isComposingText: Bool {
        get {
            objc_getAssociatedObject(self, &Self.isComposingTextKey) as? Bool ?? false
        }
        set {
            guard newValue != isComposingText else { return }
            objc_setAssociatedObject(self, &Self.isComposingTextKey, newValue, .OBJC_ASSOCIATION_RETAIN_NONATOMIC)

            // 在主執行緒觸發更新（關閉動畫避免閃爍）
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
