// 中文: TaigiKeyboardView 上三個互斥 overlay 面板的狀態容器(版面 / 符號 / 設定)。

import Foundation

/// Value-type container for the three mutually-dismissable overlay panels
/// mounted on `TaigiKeyboardView` (layout / symbol / settings).
///
/// `CandidateExpandState` is tracked separately as a reference type because
/// it is shared with child views via `@EnvironmentObject`.
// 中文: 版面 / 符號 / 設定三個 overlay 的開關狀態,互斥;另一個 CandidateExpandState 因為要走 @EnvironmentObject 共享,單獨用 reference type。
struct OverlayPanelState: Equatable {
    var isLayoutExpanded = false
    var isSymbolExpanded = false
    var isSettingsExpanded = false

    // 中文: 一次關掉三個 overlay,通常在切換 keyboard type 或外點時呼叫。
    mutating func closeAll() {
        isLayoutExpanded = false
        isSymbolExpanded = false
        isSettingsExpanded = false
    }
}
