// 中文: 組字 effect 與宿主輸入表面的橋樑 protocol。
// 中文: 讓 ComposingManager 不直接綁 KeyboardKit / UITextDocumentProxy。

import Foundation

/// Platform adapter that executes a `RustEngineBridge.ComposingTransition.Effect`
/// against the host text-input surface.
///
/// Keeps `ComposingManager` independent of `_Keyboard/` — the iOS
/// implementation (in `KeyboardViewController+TextInput`) switches on the
/// effect enum and forwards to `UITextDocumentProxy`. Android's parallel
/// implementation runs against `InputConnection`.
///
/// Binding contract (iOS / Android mappings) lives in
/// `docs/architecture/composing-state-boundary.md` §2.2.
// 中文: 平台介面卡 — 把 ComposingTransition.Effect 派送到 UITextDocumentProxy / InputConnection。
protocol ComposingDelegate: AnyObject {
    func execute(_ effect: RustEngineBridge.ComposingTransition.Effect)
}
