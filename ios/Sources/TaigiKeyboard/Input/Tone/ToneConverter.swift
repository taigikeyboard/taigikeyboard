import Foundation
import OSLog

#if DEBUG
private let toneLogger = Logger(
    subsystem: LexiconConstants.Logging.subsystem,
    category: "ToneConverter"
)
#endif

/// 聲調轉換器
///
/// 協調 POJ 和 TL 模式的聲調轉換，提供統一介面。
enum ToneConverter {

    /// 轉換輸入為聲調標記
    /// - Parameters:
    ///   - input: 輸入字串（可包含多個音節）
    ///   - mode: 輸入模式（POJ/TL）
    /// - Returns: 轉換後的字串
    static func convertToToneMarks(_ input: String, mode: InputMode) -> String {
        let result: String
        switch mode {
        case .poj:
            let preprocessed = POJToneConverter.preprocess(input)
            result = POJToneConverter.convert(preprocessed)
        case .tl:
            result = TLToneConverter.convert(input)
        case .english:
            // 英文模式不需要聲調轉換
            result = input
        }

        #if DEBUG
        if input != result {
            toneLogger.debug("[TONE] input='\(input)' mode=\(String(describing: mode)) -> '\(result)'")
        }
        #endif

        return result
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
