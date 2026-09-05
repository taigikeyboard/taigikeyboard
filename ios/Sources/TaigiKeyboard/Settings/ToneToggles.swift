// POJ 預處理的兩個 Bool 開關打包成一個值型別,讓 ComposingState / ToneConverter 不必直接讀設定。
// Foundation-only,shared-core 候選。

import Foundation

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.

/// POJ preprocessing toggles the composing engine needs when deriving display text.
///
/// Carried as an explicit value so `ComposingState` / `ToneConverter` stay
/// Foundation-pure. The wrapper reads the booleans from
/// `EngineSettingsProvider.current` at call time (live read — see
/// `EngineSettingsProvider`), then passes them through.
// ToneToggles 是引擎呼叫 phonetics 時必填的 POJ 預處理參數。
// 由 EngineSettingsProvider live-read 取出,再以值型別方式傳入引擎。
public struct ToneToggles: Equatable {
    // 雙擊 OO 開關 (POJ ↔ TL 互轉時的 oo 處理)。
    public let isDoubleTapOOEnabled: Bool
    // 雙擊 NN 開關 (鼻化音 nn 的處理)。
    public let isDoubleTapNNEnabled: Bool

    public init(isDoubleTapOOEnabled: Bool, isDoubleTapNNEnabled: Bool) {
        self.isDoubleTapOOEnabled = isDoubleTapOOEnabled
        self.isDoubleTapNNEnabled = isDoubleTapNNEnabled
    }
}
