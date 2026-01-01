import Foundation

/// 聲調還原器
///
/// 將聲調標記還原為基本字母，用於 Backspace 刪除操作。
enum ToneRestoration {

    /// 嘗試還原文字中的聲調標記
    /// - Parameters:
    ///   - text: 要還原的文字
    ///   - mode: 輸入模式（POJ/TL）
    /// - Returns: 還原後的文字，若無法還原則回傳 nil
    static func restore(_ text: String, mode: InputMode) -> String? {
        guard !text.isEmpty else { return nil }

        // 取得對應的聲調映射表
        let toneToBaseMapping = mode == .poj ?
            ToneMappings.pojToneToBase :
            ToneMappings.tlToneToBase

        // 從後往前找聲調字符
        for index in text.indices.reversed() {
            let char = text[index]
            let charStr = String(char)

            if let baseChar = toneToBaseMapping[charStr] {
                // 找到聲調字符，進行還原
                let beforeTone = String(text[..<index])
                let afterTone = String(text[text.index(after: index)...])
                let restoredText = beforeTone + baseChar + afterTone

                return restoredText
            }
        }

        // 沒有找到聲調字符
        return nil
    }
}
