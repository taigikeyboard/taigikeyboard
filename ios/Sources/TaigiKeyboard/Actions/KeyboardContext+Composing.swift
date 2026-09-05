// Adds a composing-state property to KeyboardKit's KeyboardContext via an associated object.

import Combine
import Foundation
import KeyboardKit
import ObjectiveC
import SwiftUI

extension KeyboardContext: ComposingContextSink {
    private static var isComposingTextKey: UInt8 = 0

    var isComposingText: Bool {
        get {
            objc_getAssociatedObject(self, &Self.isComposingTextKey) as? Bool ?? false
        }
        set {
            guard newValue != isComposingText else { return }
            objc_setAssociatedObject(self, &Self.isComposingTextKey, newValue, .OBJC_ASSOCIATION_RETAIN_NONATOMIC)

            // Dispatch on the main thread with animations disabled to avoid a flicker.
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
