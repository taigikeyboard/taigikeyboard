// 中文: TPS 連動的 re-entry guard。SharedSettings.inputMode 與 keyboardLayoutType 互相觸發 setter 時用來阻止無窮遞迴。

import Foundation

/// Re-entry guard for the TPS ↔ layout 1:1 sync in `SharedSettings`.
///
/// When the user changes `inputMode` to `.tps`, the setter also flips
/// `keyboardLayoutType` to `.tps` — and vice versa. Without a guard,
/// that cross-write triggers infinite recursion. Each setter wraps its
/// own sync block in `sync { ... }`, so only the outer call runs and
/// the inner re-entry short-circuits.
// 中文: TPS 雙向連動的 re-entry guard。inputMode setter 與 keyboardLayoutType setter 互寫時,
// 中文: 內層的 sync block 會直接 short-circuit,只跑最外層那一次。
final class TPSSyncCoordinator {
    private var isSyncing = false

    /// Run `block` once. If already inside a `sync` invocation (i.e. the
    /// current property setter was triggered by another setter), no-op.
    // 中文: 執行 block 一次。若已在另一個 sync 呼叫中 (代表是被另一個 setter 觸發進來的),no-op。
    func sync(_ block: () -> Void) {
        guard !isSyncing else { return }
        isSyncing = true
        defer { isSyncing = false }
        block()
    }
}
