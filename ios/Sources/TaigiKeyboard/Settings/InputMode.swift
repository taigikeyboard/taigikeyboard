// 中文: 輸入模式 (InputMode) 領域型別。POJ / TL / English / TPS 四種。
// 中文: Foundation-only 純值型別,shared-core 候選;displayName 在 SettingsModels.swift 以 extension 提供。

import Foundation

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.

/// Input mode selected by the user: POJ (Pe̍h-ōe-jī), TL (Tâi-lô),
/// English passthrough, or TPS (Taiwanese Phonetic Symbols).
///
/// The enum is a pure value type over `String` so it can live in shared core;
/// the `displayName` localization sibling stays platform-side and is provided
/// via an extension in `SettingsModels.swift`.
// 中文: 使用者目前選用的輸入法種類。rawValue 與 UserDefaults 持久化欄位對齊。
public enum InputMode: String, CaseIterable {
    // 中文: Pe̍h-ōe-jī 白話字。
    case poj // Pe̍h-ōe-jī
    // 中文: Tâi-lô 教育部臺羅。
    case tl // Tâi-lô
    // 中文: 英文直通模式,不走音標處理。
    case english
    // 中文: TPS 台語注音符號 (Taiwanese Phonetic Symbols)。
    case tps // Taiwanese Phonetic Symbols
}
