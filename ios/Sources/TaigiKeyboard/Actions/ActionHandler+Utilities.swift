import Foundation

/// 工具函數
///
/// 提供字元判斷的輔助方法。
extension ActionHandler {

    /// 檢查是否為標點符號（連字符號 "-" 除外，因為用於組字）
    ///
    /// 包含 Alphabetic、Numeric、Symbolic 鍵盤上的所有符號。
    /// 這些符號會直接輸出，不進入組字模式。
    func isPunctuationExceptHyphen(_ char: String) -> Bool {
        // 基本標點符號
        let basicPunctuation = ".,!?;:()[]{}\"'`~@#$%^&*+=<>/\\|_"

        // 中文標點符號
        let chinesePunctuation = "、。，！？；：（）「」『』《》【】〈〉〔〕｛｝…⋯"

        // Curly quotes（Numeric 鍵盤使用）
        let curlyQuotes = "\u{201C}\u{201D}\u{2018}\u{2019}"  // " " ' '

        // 特殊符號（Symbolic 鍵盤）
        let specialSymbols = "—«»※"

        // 貨幣符號
        let currencySymbols = "€£¥¢$"

        // 其他符號
        let otherSymbols = "•·°©®™℃"

        // 數學符號
        let mathSymbols = "±×÷≠≈∞√"

        let allPunctuation = basicPunctuation + chinesePunctuation + curlyQuotes +
                             specialSymbols + currencySymbols + otherSymbols + mathSymbols

        return allPunctuation.contains(char)
    }
}
