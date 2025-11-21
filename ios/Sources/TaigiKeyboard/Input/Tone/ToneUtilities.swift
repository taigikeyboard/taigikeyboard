import Foundation

/// 聲調相關工具方法
enum ToneUtilities {

    /// 聲調字母大寫轉換
    /// - Parameters:
    ///   - char: 要轉換的字元
    ///   - mode: 輸入模式（POJ/TL）
    /// - Returns: 大寫後的字元，如果不是聲調字母則使用標準轉換
    static func uppercaseToneLetter(_ char: String, mode: InputMode) -> String {
        let mapping = mode == .poj ?
            ToneMappings.pojLowercaseToUppercase :
            ToneMappings.tlLowercaseToUppercase
        return mapping[char] ?? char.uppercased()
    }

    /// 判斷字串是否包含漢字
    /// - Parameter input: 要判斷的字串
    /// - Returns: 是否包含漢字
    static func isHanzi(_ input: String) -> Bool {
        input.contains { char in
            guard let scalar = char.unicodeScalars.first else { return false }
            return (0x4E00 ... 0x9FFF).contains(scalar.value) // CJK Unified Ideographs
                || (0x3400 ... 0x4DBF).contains(scalar.value) // CJK Extension A
                || (0x20000 ... 0x2A6DF).contains(scalar.value) // CJK Extension B
                || (0x2A700 ... 0x2B73F).contains(scalar.value) // CJK Extension C
                || (0x2B740 ... 0x2B81F).contains(scalar.value) // CJK Extension D
                || (0x2B820 ... 0x2CEAF).contains(scalar.value) // CJK Extension E
        }
    }
}
