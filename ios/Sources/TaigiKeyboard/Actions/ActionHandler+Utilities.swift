import Foundation

/// 工具函數
///
/// 提供字元判斷的輔助方法。
extension ActionHandler {
    /// 檢查字元是否應進入組字模式（allowlist）
    ///
    /// 只有羅馬字字母、TPS 注音符號、TPS 聲調符號、連字符號和 ˙ 可進入組字。
    /// 其他所有符號（標點、箭頭、emoji 等）直接輸出，不進入組字模式。
    func isComposingCharacter(_ char: String) -> Bool {
        guard let first = char.first else { return false }
        // isLetter covers: a-z, A-Z (Lu/Ll), TPS bopomofo ㄅ-ㆷ (Lo),
        // TPS tone marks ˋ ˊ ˇ ˆ (Lm).
        // Three TPS tone marks are Sk (Symbol, modifier), not caught by isLetter:
        //   ˪ (U+02EA, tone 3), ˫ (U+02EB, tone 7), ˙ (U+02D9, tone 8)
        return first.isLetter || first == "-"
            || first == "\u{02EA}" || first == "\u{02EB}" || first == "\u{02D9}"
    }
}
