// 鍵盤按鍵的抽象定義 — TaigiLayouts 用 [[KeyDef]] 描述每張版面,
// 再由 LayoutConverter 轉成 KeyboardKit 的 KeyboardLayout。

/// Keyboard key definition for layout composition
// 鍵盤按鍵的抽象 enum — 用於 TaigiLayouts 描述各種佈局。
enum KeyDef {
    // MARK: - 字符按鍵

    /// 字符按鍵
    /// - Parameters:
    ///   - char: 半形字符
    ///   - fullWidth: 全形字符（可選，用於 isTranslateSwapped 模式）
    case char(String, fullWidth: String? = nil)

    // MARK: - 功能鍵

    case shift
    case backspace
    case space
    case `return`
    // 漢字 ↔ 羅馬字切換鍵(translate),候選詞顯示語系切換用。
    case translate

    // MARK: - 鍵盤切換

    case numeric // 切換到數字鍵盤
    case symbolic // 切換到符號鍵盤
    case alphabetic // 切換回字母鍵盤

    // MARK: - 系統按鍵

    case globe // 切換輸入法
    case emoji // 切換到表情符號
}
