// 引擎層讀取設定的進入點。實作端必須回傳 live-reading EngineSettings (每次讀都是新值)。

import Foundation

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.

/// Supplies the engine with an `EngineSettings` that reflects the current
/// state of the underlying store.
///
/// `current` is expected to return a *live-reading* implementation — each
/// call chain like `provider.current.inputMode` reads the most recent
/// `UserDefaults` value. This preserves the keyboard extension's
/// live-update behavior: the user changes a setting in the host app,
/// `UserDefaults.didChangeNotification` fires, and the next engine query
/// sees the new value without the controller being reconstructed.
///
/// Do NOT return a one-shot snapshot from `current`. If a caller needs
/// consistency across multiple reads, it should capture a local copy.
// 引擎側設定 provider。current 回傳值必須 live-read,而非快照。
// 這是 keyboard extension 跨程序設定同步的核心契約 — host app 寫設定,extension 下一次讀就生效。
// 若需多次讀取一致,呼叫端自行 capture 區域變數。
protocol EngineSettingsProvider: AnyObject {
    // 取得當下設定。每次 access 都直接讀 UserDefaults,不可回傳一次性 snapshot。
    var current: EngineSettings { get }
}
