/// Keyboard key definition for layout composition
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
    case translate

    // MARK: - 鍵盤切換

    case numeric // 切換到數字鍵盤
    case symbolic // 切換到符號鍵盤
    case alphabetic // 切換回字母鍵盤

    // MARK: - 系統按鍵

    case globe // 切換輸入法
    case emoji // 切換到表情符號
}
