import Foundation

/// 輸入正規化工具
///
/// 將使用者輸入統一轉換為 TL 數字聲調格式：
/// - POJ 調符（hó）→ hoo2
/// - TL 調符（hóo）→ hoo2
/// - POJ 數字（ho2）→ hoo2
/// - TL 數字（hoo2）→ hoo2
///
/// 處理步驟：分割音節 → 逐音節處理（調符轉數字）→ 合併
enum InputNormalizer {

    // MARK: - Tone Mappings

    /// 調符 → 聲調數字（參考 KeSi）
    /// 使用 Unicode.Scalar 作為 key，因為需要按 unicode scalar 迭代才能正確識別 combining marks
    private static let toneMarkToNumber: [Unicode.Scalar: String] = [
        "\u{0301}": "2",  // ́ COMBINING ACUTE ACCENT
        "\u{0300}": "3",  // ̀ COMBINING GRAVE ACCENT
        "\u{0302}": "5",  // ̂ COMBINING CIRCUMFLEX ACCENT
        "\u{030C}": "6",  // ̌ COMBINING CARON
        "\u{0304}": "7",  // ̄ COMBINING MACRON
        "\u{030D}": "8",  // ̍ COMBINING VERTICAL LINE ABOVE
        "\u{0306}": "9",  // ̆ COMBINING BREVE (POJ)
        "\u{030B}": "9",  // ̋ COMBINING DOUBLE ACUTE ACCENT (TL)
    ]

    // MARK: - Public API

    /// 正規化輸入為 Trie 查詢格式（TL 數字聲調）
    ///
    /// 支援任何輸入格式：
    /// - POJ 調符（hó-bô）→ hoo2boo5
    /// - TL 調符（hóo-bôo）→ hoo2boo5
    /// - POJ 數字（ho2-bo5）→ hoo2boo5
    /// - TL 數字（hoo2-boo5）→ hoo2boo5
    ///
    /// - Parameters:
    ///   - input: 使用者輸入
    ///   - mode: POJ 或 TL 模式（目前未使用，POJ/TL 調符相同）
    /// - Returns: 正規化後的字串（小寫、無連字符、數字聲調）
    static func normalize(_ input: String, mode: InputMode) -> String {
        guard !input.isEmpty else { return "" }

        // 轉小寫
        let lowercased = input.lowercased()

        // 以連字符分割音節，逐音節處理
        let syllables = lowercased.split(separator: "-", omittingEmptySubsequences: false)
        let result = syllables.map { syllable in
            normalizeSyllable(String(syllable))
        }

        // 合併（不含連字符）
        return result.joined()
    }

    /// 移除輸入中的所有聲調（調符和數字）
    ///
    /// 用於生成無聲調查詢 key
    ///
    /// - Parameters:
    ///   - input: 使用者輸入
    ///   - mode: POJ 或 TL 模式
    /// - Returns: 移除聲調後的字串
    static func removeAllTones(_ input: String, mode: InputMode) -> String {
        // 先正規化（調符→數字）
        let normalized = normalize(input, mode: mode)
        // 再移除數字
        return normalized.replacingOccurrences(
            of: "[0-9]",
            with: "",
            options: .regularExpression
        )
    }

    /// 檢查輸入是否包含聲調標記
    static func hasToneMarks(_ input: String) -> Bool {
        // NFD 分解
        let nfd = input.decomposedStringWithCanonicalMapping
        return nfd.unicodeScalars.contains { toneMarkToNumber[$0] != nil }
    }

    /// 檢查輸入是否包含數字聲調
    static func hasNumericTones(_ input: String) -> Bool {
        input.contains { $0.isNumber }
    }

    // MARK: - Private Methods

    /// 正規化單一音節
    ///
    /// 步驟：
    /// 1. 轉換 POJ 鼻音符號 ⁿ → nn
    /// 2. NFD 分解（將預組合字符分解為基礎字符 + 組合標記）
    /// 3. 提取聲調標記，轉為數字
    /// 4. 組合：音節 + 聲調數字
    ///
    /// 注意：不做 POJ→TL 拼法轉換（如 ch→ts），因為 Trie 使用前綴區分（tl:/poj:）
    private static func normalizeSyllable(_ syllable: String) -> String {
        guard !syllable.isEmpty else { return "" }

        // 轉換 POJ 鼻音符號 ⁿ (U+207F) → nn
        let withNasalConverted = syllable.replacingOccurrences(of: "\u{207F}", with: "nn")

        // 檢查是否已有數字聲調（如 ho2）
        if let lastChar = withNasalConverted.last, lastChar.isNumber {
            // 已有數字聲調，直接返回
            return withNasalConverted
        }

        // NFD 分解
        let nfd = withNasalConverted.decomposedStringWithCanonicalMapping

        // 轉換 POJ o͘：把 U+0358 (COMBINING DOT ABOVE RIGHT) 替換成 o
        // 需在 NFD 分解後處理，因為 ó͘ 分解後是 o + ́ + ͘
        let withOoConverted = nfd.replacingOccurrences(of: "\u{0358}", with: "o")

        // 提取聲調標記（使用 unicodeScalars 迭代，才能正確識別 combining marks）
        var toneNumber = ""
        var withoutTone = ""

        for scalar in withOoConverted.unicodeScalars {
            if let tone = toneMarkToNumber[scalar] {
                toneNumber = tone  // 取最後一個聲調標記
            } else {
                withoutTone.append(String(scalar))
            }
        }

        // 組合：音節 + 聲調數字
        return withoutTone + toneNumber
    }
}
