import Foundation

/// 聲調轉換器 - 統一介面
/// 負責協調 POJ 和 TL 兩種模式的聲調轉換
enum ToneConverter {

    /// 轉換輸入為聲調標記
    /// - Parameters:
    ///   - input: 輸入字串（可包含多個音節）
    ///   - mode: 輸入模式（POJ/TL）
    /// - Returns: 轉換後的字串
    static func convertToToneMarks(_ input: String, mode: InputMode) -> String {
        switch mode {
        case .poj:
            let preprocessed = POJToneConverter.preprocess(input)
            return POJToneConverter.convert(preprocessed)
        case .tl:
            return TLToneConverter.convert(input)
        }
    }

    /// 還原聲調標記為基本字母
    /// - Parameters:
    ///   - text: 要還原的文字
    ///   - mode: 輸入模式（POJ/TL）
    /// - Returns: 還原後的文字，若無法還原則回傳 nil
    static func restoreTone(_ text: String, mode: InputMode) -> String? {
        ToneRestoration.restore(text, mode: mode)
    }
}
