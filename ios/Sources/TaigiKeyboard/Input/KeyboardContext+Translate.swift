//
//  KeyboardContext+Translate.swift
//  TaigiKeyboard
//
//  Created by Claude Code on 2025-08-30.
//

import KeyboardKit
import SwiftUI

// MARK: - KeyboardContext Extension for Translate State

public extension KeyboardContext {
    /// Whether the translate display is swapped (hanzi ↔ roman)
    var isTranslateSwapped: Bool {
        get {
            SharedSettings.shared.isTranslateSwapped
        }
        set {
            SharedSettings.shared.isTranslateSwapped = newValue
            // 手動觸發 SwiftUI 更新
            DispatchQueue.main.async {
                self.objectWillChange.send()
            }
        }
    }

    /// Toggle the translate display mode
    func toggleTranslateSwapped() {
        isTranslateSwapped.toggle()
    }
}
