import KeyboardKit

/// 全形標點符號映射
enum PunctuationMapping {
    /// 根據鍵盤類型取得對應的全形符號
    static func fullWidthCharacter(
        for char: String,
        keyboardType: Keyboard.KeyboardType
    ) -> String? {
        switch keyboardType {
        case .numeric:
            return numericMap[char]
        case .symbolic:
            return symbolicMap[char]
        case .alphabetic, .webSearch:
            return alphabeticMap[char]
        default:
            return nil
        }
    }

    // MARK: - Alphabetic Keyboard Mapping

    /// Alphabetic 鍵盤全形標點符號映射表（PhahTaigi Layout 使用）
    private static let alphabeticMap: [String: String] = [
        ",": "，",
        ".": "。",
        "?": "？",
        "!": "！",
    ]

    // MARK: - Numeric Keyboard Mapping

    /// Numeric 鍵盤全形標點符號映射表（包含基本標點符號）
    private static let numericMap: [String: String] = [
        ",": "，",
        ".": "。",
        "?": "？",
        "!": "！",
        "’": "、",

        "”": "」",
        "@": "「",
        "&": "＠",
        "$": "＄",
        "(": "（",
        ")": "）",
        ";": "；",
        ":": "：",
    ]

    // MARK: - Symbolic Keyboard Mapping

    /// Symbolic 鍵盤全形標點符號映射表
    private static let symbolicMap: [String: String] = [
        // Symbolic 鍵盤專用符號
        "[": "［",
        "]": "］",
        "{": "｛",
        "}": "｝",
        "#": "＃",
        "%": "％",
        "^": "＾",
        "*": "＊",
        "+": "＋",
        "=": "＝",

        "\\": "—",
        "_": "＿",
        "|": "＼",
        "~": "｜",
        "<": "～",
        ">": "《",
        "€": "》",
        "£": "＆",
        "•": "·",

        ".": "⋯",
        ",": "，",
        "?": "？",
        "!": "！",
        "'": "'",
    ]
}
