// 設定與使用者資料的 reset 中介層。把 SharedSettings 與 KeyboardSettings.store 兩個來源接起來,
// 並把破壞性 reset (清空使用者頻率/關聯 DB) 與單純還原預設值刻意分開兩個 entry point。

import Foundation
import KeyboardKit

/// Resets settings and user-owned data stores.
///
/// Two surfaces, intentionally separate:
/// - `resetAll()` — pure settings reset (SharedSettings + KeyboardKit defaults).
/// - `resetAllUserData()` — destructive: wipes user frequency and next-word DBs.
///
/// `SharedSettings.resetToDefaults()` can't touch the KeyboardKit store
/// directly without importing KeyboardKit in the (soon-to-be engine-only)
/// settings module. The coordinator is the composition point.
// Reset 協調器。SharedSettings 不能直接 import KeyboardKit (engine-only 模組約束),
// 因此跨 store 的 reset 邏輯統一在這裡組合。
enum SettingsResetCoordinator {
    private static let logger = DebugLogger(category: "SettingsResetCoordinator")

    /// Reset the three KeyboardKit-owned user defaults that SharedSettings
    /// does not own. Keep these in sync with `SettingsTab` "reset all" UX.
    // 還原 KeyboardKit 自有的三個 store key (自動大寫 / 音效回饋 / 震動回饋)。
    // 必須與 SettingsTab「全部重設」UX 同步。
    static func resetKeyboardKitDefaults() {
        KeyboardSettings.store.set(true, forKey: "com.keyboardkit.settings.keyboard.isAutocapitalizationEnabled")
        KeyboardSettings.store.set(true, forKey: "com.keyboardkit.settings.feedback.isAudioFeedbackEnabled")
        KeyboardSettings.store.set(true, forKey: "com.keyboardkit.settings.feedback.isHapticFeedbackEnabled")
    }

    /// Reset every setting to its default — SharedSettings + KeyboardKit.
    /// Does NOT touch user-owned databases; call `resetAllUserData()` for that.
    // 重設所有設定 (SharedSettings + KeyboardKit) 為預設值,但不動使用者資料 DB。
    static func resetAll() {
        SharedSettings.shared.resetToDefaults()
        resetKeyboardKitDefaults()
    }

    /// Delete user-owned databases (frequency + next-word association).
    /// Each deletion is attempted independently; failures are logged, never thrown,
    /// so a partial failure still clears what it can.
    // 刪除使用者頻率 DB 與 next-word 關聯 DB。兩個刪除動作獨立進行,任一失敗都只記 log,
    // 不向上拋,避免單一失敗讓另一邊也跟著失敗。
    static func resetAllUserData() {
        do {
            try CompositionRoot.userFrequencyRepository.deleteDatabase()
        } catch {
            logger.error("Failed to delete frequency database: \(error)")
        }
        do {
            try CompositionRoot.nextWordService.deleteUserDatabase()
        } catch {
            logger.error("Failed to delete association database: \(error)")
        }
    }
}
