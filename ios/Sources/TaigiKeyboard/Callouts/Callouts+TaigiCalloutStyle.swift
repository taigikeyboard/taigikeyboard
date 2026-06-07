// 中文: 依鍵盤字型產生對應的 KeyboardKit Callout 樣式。長按 callout 的字型與按鍵保持一致。
// 中文: 單一來源 — keyboard extension(KeyboardViewController+Setup)與外觀預覽(KeyboardPreviewPanel)共用。

import KeyboardKit

// 中文: internal(非 public)— 參數 FontType 為 internal,public 會違反存取控制。
extension Callouts.CalloutStyle {
    /// Builds the long-press callout style for a keyboard `fontType`.
    ///
    /// `.system` keeps KeyboardKit's standard callout font; every custom font
    /// uses the action/input item sizes long-tuned for Taigi callouts (20pt
    /// action, 32pt input). Single source for the keyboard extension and the
    /// appearance-editor preview, so the two never drift.
    // 中文: system 字型回 KeyboardKit 標準樣式;自訂字型套用 Taigi callout 既有字級(action 20 / input 32)。
    static func taigi(for fontType: FontType) -> Callouts.CalloutStyle {
        guard let fontName = fontType.customFontName else {
            return .standard
        }
        return Callouts.CalloutStyle(
            actionItemFont: KeyboardFont.custom(fontName, size: 20, weight: .regular),
            inputItemFont: KeyboardFont.custom(fontName, size: 32, weight: .light),
        )
    }
}
