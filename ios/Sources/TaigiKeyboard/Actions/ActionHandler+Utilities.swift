import Foundation

/// 工具函數
///
/// 提供字元判斷的輔助方法。
extension ActionHandler {

    /// All punctuation characters (excluding hyphen "-" which is used for composing)
    private static let punctuationSet: Set<Character> = {
        let all = ".,!?;:()[]{}\"'`~@#$%^&*+=<>/\\|_" +  // basic
                  "、。，！？；：（）「」『』《》【】〈〉〔〕｛｝…⋯" +  // Chinese
                  "\u{201C}\u{201D}\u{2018}\u{2019}" +  // curly quotes
                  "—«»※" +  // special
                  "€£¥¢$" +  // currency
                  "•·°©®™℃" +  // other
                  "±×÷≠≈∞√"  // math
        return Set(all)
    }()

    /// 檢查是否為標點符號（連字符號 "-" 除外，因為用於組字）
    ///
    /// 包含 Alphabetic、Numeric、Symbolic 鍵盤上的所有符號。
    /// 這些符號會直接輸出，不進入組字模式。
    func isPunctuationExceptHyphen(_ char: String) -> Bool {
        guard let first = char.first else { return false }
        return Self.punctuationSet.contains(first)
    }
}
