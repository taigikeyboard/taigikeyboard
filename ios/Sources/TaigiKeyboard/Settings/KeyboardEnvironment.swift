// 中文: keyboard extension UI 端設定 protocol。是 EngineSettingsProvider 的非引擎對偶 —
// 中文: 涵蓋外觀、inputMode 寫入、full-access 狀態、以及給 didChangeNotification 用的 App Group UserDefaults。

import Foundation
import SwiftUI

/// Keyboard-extension UI-facing settings contract.
///
/// Non-engine counterpart to `EngineSettingsProvider`: covers appearance,
/// input-mode writes, full-access writes, and exposes the App Group
/// `UserDefaults` the controller binds to `UserDefaults.didChangeNotification`
/// for cross-process settings sync.
///
/// ### Live-read semantics
/// Each property access reads the backing store fresh, so a setting change
/// in the host app becomes visible on the next read inside the keyboard
/// extension without the controller being reconstructed. This mirrors the
/// contract `EngineSettingsProvider` documents for engine-layer reads.
///
/// ### Consistency boundary
/// `snapshot(for:)` is the per-render-cycle consistency anchor. Hot render
/// paths should call it once and read the returned struct, rather than
/// referencing the live properties multiple times in the same body evaluation.
///
/// ### Scope
/// This is keyboard-extension / UI-facing only. Engine-layer code must not
/// depend on this protocol — it remains on `EngineSettings` /
/// `EngineSettingsProvider` (Foundation-only, read-only).
// 中文: UI 層設定 protocol。讀取為 live-read,渲染熱路徑請改用 snapshot(for:) 取一致快照。
// 中文: 引擎層禁止依賴此 protocol,改走 EngineSettings/EngineSettingsProvider (Foundation-only)。
protocol KeyboardEnvironment: AnyObject {
    // 中文: 目前輸入模式 (UI 端可寫入,host app 設定頁也走這個 setter)。
    var inputMode: InputMode { get set }
    // 中文: Full Access 狀態旗標 (持久化於 App Group)。
    var isFullAccessEnabled: Bool { get set }
    // 中文: 鍵盤六個顏色面 (背景 / 文字 / 鍵帽 / 候選列 等)。外觀編輯器讀寫此 buffer。
    var colorSettings: KeyboardColorSettings { get }
    // 中文: 渲染端消費的解析外觀(選定主題 + colorScheme → 6 顏色 + 陰影 + 5 尺寸)。字型為全域設定,不在此包內。
    // 中文: "default" 走全域外觀;built-in 依 colorScheme 取 light/dark。colorScheme 來源 = keyboardContext.colorScheme。
    func resolvedAppearance(for colorScheme: ColorScheme) -> ThemeAppearance
    // 中文: 鍵帽文字大小縮放係數,預設 1.0。
    var keyFontSizeScale: CGFloat { get }
    // 中文: 鍵帽邊框寬度,預設 0。
    var keyBorderWidth: CGFloat { get }
    // 中文: 候選列文字大小縮放係數,預設 1.0。
    var candidateTextSizeScale: CGFloat { get }
    // 中文: 鍵盤字體選用 (system / openHuninn / iansui / genYoMin / genYoGothic)。全域設定,非每主題。
    // 中文: 供 callout / KeyboardFonts.globalFont 等無 per-render snapshot 的呼叫點直接讀取。
    var fontType: FontType { get }
    // 中文: 鍵盤排版 (phahTaigi / qwerty / tps / moe1 / moe2)。
    var keyboardLayoutType: KeyboardLayoutType { get }

    /// App Group `UserDefaults` used as the notification filter for
    /// `UserDefaults.didChangeNotification`. The controller observes changes
    /// on this object to pick up writes made by the host app.
    // 中文: App Group 共享的 UserDefaults。controller 對它註冊 didChangeNotification,
    // 中文: 收到 host app 寫設定後即時刷新 keyboard extension 狀態。
    var settingsUserDefaults: UserDefaults { get }

    // 中文: 取得當下渲染週期的設定一致性快照,避免渲染 ~50 個鍵時重複讀 UserDefaults。
    // 中文: colorSettings 欄位依 colorScheme 解析(built-in 主題的 light/dark 在此決定)。
    func snapshot(for colorScheme: ColorScheme) -> SettingsSnapshot
}

extension SharedSettings: KeyboardEnvironment {
    // 中文: 將 SharedSettings 持有的 App Group store 暴露給 protocol 觀察者。
    var settingsUserDefaults: UserDefaults {
        Self.sharedUserDefaults
    }
}
