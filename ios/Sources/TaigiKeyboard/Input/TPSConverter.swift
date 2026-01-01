import Foundation

/// 台灣方音符號（TPS）轉換器
///
/// 支援雙向轉換：
/// - TPS → TL：用於 Trie 搜尋
/// - TL → TPS：用於候選詞顯示
///
/// 參考：https://github.com/leechunhoe/Tailo-TPS-Converter
enum TPSConverter {

    // MARK: - 聲母對照表（TPS → TL）

    /// 聲母對照表
    /// 注意：複合聲母（如 ㄐㄧ → tsi）需優先匹配
    private static let consonants: [(tps: String, tl: String)] = [
        // 複合聲母（優先匹配）
        ("ㄑㄧ", "tshi"),
        ("ㄐㄧ", "tsi"),
        ("ㄒㄧ", "si"),
        ("ㆢㄧ", "ji"),
        // 單聲母
        ("ㄅ", "p"),
        ("ㄆ", "ph"),
        ("ㄇ", "m"),
        ("ㆠ", "b"),
        ("ㄉ", "t"),
        ("ㄊ", "th"),
        ("ㄋ", "n"),
        ("ㄌ", "l"),
        ("ㄍ", "k"),
        ("ㄎ", "kh"),
        ("ㄫ", "ng"),
        ("ㆣ", "g"),
        ("ㄏ", "h"),
        ("ㄗ", "ts"),
        ("ㄘ", "tsh"),
        ("ㄙ", "s"),
        ("ㆡ", "j"),
    ]

    // MARK: - 韻母對照表（TPS → TL）

    /// 韻母對照表
    /// 注意：複合韻母需優先匹配
    private static let vowels: [(tps: String, tl: String)] = [
        // 鼻化韻母（優先匹配）
        ("ㆮ", "ainn"),
        ("ㆯ", "aunn"),
        ("ㆩ", "ann"),
        ("ㆥ", "enn"),
        ("ㆪ", "inn"),
        ("ㆧ", "onn"),
        ("ㆫ", "unn"),
        // 複合韻母
        ("ㄤ", "ang"),
        ("ㆲ", "ong"),
        ("ㆦ", "oo"),
        ("ㄝ", "ee"),
        ("ㄜ", "er"),
        ("ㆨ", "ir"),
        ("ㄞ", "ai"),
        ("ㄠ", "au"),
        ("ㆰ", "am"),
        ("ㆱ", "om"),
        ("ㄢ", "an"),
        ("ㆭ", "ng"),
        // 單韻母
        ("ㄚ", "a"),
        ("ㆤ", "e"),
        ("ㄧ", "i"),
        ("ㄛ", "o"),
        ("ㄨ", "u"),
        ("ㄣ", "n"),
        ("ㄥ", "ng"),
        ("ㆬ", "m"),
    ]

    // MARK: - 聲調對照表（TPS → TL）

    /// 聲調符號對照表
    /// 入聲韻尾（ㆴㆵㆶㆷ）會同時影響韻尾和聲調
    private static let tones: [(tps: String, tl: String, tone: String)] = [
        // 入聲韻尾（帶點為第8聲，無點為第4聲）
        ("ㆴ̇", "p", "8"),  // p + 第8聲
        ("ㆵ̇", "t", "8"),  // t + 第8聲
        ("ㆶ̇", "k", "8"),  // k + 第8聲
        ("ㆷ̇", "h", "8"),  // h + 第8聲
        ("ㆴ", "p", "4"),   // p + 第4聲
        ("ㆵ", "t", "4"),   // t + 第4聲
        ("ㆶ", "k", "4"),   // k + 第4聲
        ("ㆷ", "h", "4"),   // h + 第4聲
    ]

    /// 聲調符號（非入聲）
    private static let toneMarks: [(tps: String, tone: String)] = [
        ("ˋ", "2"),   // 第2聲
        ("˪", "3"),   // 第3聲
        ("ˊ", "5"),   // 第5聲
        ("˫", "7"),   // 第7聲
        // 第1聲無符號
    ]

    // MARK: - TPS 字符集合（用於檢測）

    /// 所有 TPS 字符（用於快速檢測）
    private static let tpsCharacters: Set<Character> = {
        var chars = Set<Character>()

        // 聲母
        for (tps, _) in consonants {
            chars.formUnion(tps)
        }
        // 韻母
        for (tps, _) in vowels {
            chars.formUnion(tps)
        }
        // 入聲韻尾
        for (tps, _, _) in tones {
            chars.formUnion(tps)
        }
        // 聲調符號
        for (tps, _) in toneMarks {
            chars.formUnion(tps)
        }

        return chars
    }()

    // MARK: - Public API

    /// 檢測字串是否包含 TPS 字符
    ///
    /// - Parameter input: 輸入字串
    /// - Returns: 是否包含 TPS 字符
    static func containsTPS(_ input: String) -> Bool {
        input.contains { tpsCharacters.contains($0) }
    }

    /// 將 TPS 轉換為 TL
    ///
    /// 轉換流程：
    /// 1. 解析入聲韻尾（同時處理韻尾和聲調）
    /// 2. 解析聲調符號
    /// 3. 解析聲母（複合聲母優先）
    /// 4. 解析韻母（複合韻母優先）
    /// 5. 組合為 TL 格式
    ///
    /// - Parameter tps: TPS 字串（如 "ㄉㄧㄠˊ"）
    /// - Returns: TL 字串（如 "tiau5"）
    static func toTL(_ tps: String) -> String {
        guard !tps.isEmpty else { return "" }

        var remaining = tps
        var result = ""

        // 逐字符處理
        while !remaining.isEmpty {
            var matched = false

            // 1. 嘗試匹配入聲韻尾（優先，因為會影響韻尾和聲調）
            for (tpsPattern, tlEnding, tone) in tones {
                if remaining.hasPrefix(tpsPattern) {
                    result += tlEnding + tone
                    remaining.removeFirst(tpsPattern.count)
                    matched = true
                    break
                }
            }
            if matched { continue }

            // 2. 嘗試匹配聲調符號
            for (tpsPattern, tone) in toneMarks {
                if remaining.hasPrefix(tpsPattern) {
                    result += tone
                    remaining.removeFirst(tpsPattern.count)
                    matched = true
                    break
                }
            }
            if matched { continue }

            // 3. 嘗試匹配聲母（複合聲母優先，因為陣列已排序）
            for (tpsPattern, tl) in consonants {
                if remaining.hasPrefix(tpsPattern) {
                    result += tl
                    remaining.removeFirst(tpsPattern.count)
                    matched = true
                    break
                }
            }
            if matched { continue }

            // 4. 嘗試匹配韻母（複合韻母優先，因為陣列已排序）
            for (tpsPattern, tl) in vowels {
                if remaining.hasPrefix(tpsPattern) {
                    result += tl
                    remaining.removeFirst(tpsPattern.count)
                    matched = true
                    break
                }
            }
            if matched { continue }

            // 5. 無法匹配，保留原字符（可能是空格、標點等）
            result.append(remaining.removeFirst())
        }

        return result
    }

    /// 將多音節 TPS 轉換為 TL
    ///
    /// 支援以空格分隔的多音節輸入
    ///
    /// - Parameter tps: TPS 字串（如 "ㄉㄧㄠˊ ㄙㄨˊ"）
    /// - Returns: TL 字串（如 "tiau5 su5"）
    static func toTLMultiSyllable(_ tps: String) -> String {
        tps.split(separator: " ")
            .map { toTL(String($0)) }
            .joined(separator: " ")
    }

    // MARK: - TL → TPS 轉換

    /// TL → TPS 聲母對照表（長的優先匹配）
    private static let tlToTpsConsonants: [(tl: String, tps: String)] = [
        // 複合聲母（優先匹配）
        ("tshi", "ㄑㄧ"),
        ("tsi", "ㄐㄧ"),
        ("tsh", "ㄘ"),
        ("ts", "ㄗ"),
        ("ph", "ㄆ"),
        ("th", "ㄊ"),
        ("kh", "ㄎ"),
        ("ng", "ㄫ"),
        ("si", "ㄒㄧ"),
        ("ji", "ㆢㄧ"),
        // 單聲母
        ("p", "ㄅ"),
        ("m", "ㄇ"),
        ("b", "ㆠ"),
        ("t", "ㄉ"),
        ("n", "ㄋ"),
        ("l", "ㄌ"),
        ("k", "ㄍ"),
        ("g", "ㆣ"),
        ("h", "ㄏ"),
        ("s", "ㄙ"),
        ("j", "ㆡ"),
    ]

    /// TL → TPS 韻母對照表（長的優先匹配）
    private static let tlToTpsVowels: [(tl: String, tps: String)] = [
        // 鼻化韻母（優先匹配）
        ("ainn", "ㆮ"),
        ("aunn", "ㆯ"),
        ("ann", "ㆩ"),
        ("enn", "ㆥ"),
        ("inn", "ㆪ"),
        ("onn", "ㆧ"),
        ("unn", "ㆫ"),
        // 複合韻母
        ("ang", "ㄤ"),
        ("ong", "ㆲ"),
        ("oo", "ㆦ"),
        ("ee", "ㄝ"),
        ("er", "ㄜ"),
        ("ir", "ㆨ"),
        ("ai", "ㄞ"),
        ("au", "ㄠ"),
        ("am", "ㆰ"),
        ("om", "ㆱ"),
        ("an", "ㄢ"),
        // 單韻母
        ("a", "ㄚ"),
        ("e", "ㆤ"),
        ("i", "ㄧ"),
        ("o", "ㄛ"),
        ("u", "ㄨ"),
        ("m", "ㆬ"),
        ("n", "ㄣ"),
    ]

    /// TL → TPS 聲調對照表
    private static let tlToTpsTones: [(tl: String, tps: String)] = [
        ("2", "ˋ"),
        ("3", "˪"),
        ("5", "ˊ"),
        ("7", "˫"),
        // 1, 4 無符號
    ]

    /// TL → TPS 入聲韻尾對照表
    private static let tlToTpsCheckedTones: [(tl: String, tps: String)] = [
        ("p8", "ㆴ̇"),
        ("t8", "ㆵ̇"),
        ("k8", "ㆶ̇"),
        ("h8", "ㆷ̇"),
        ("p4", "ㆴ"),
        ("t4", "ㆵ"),
        ("k4", "ㆶ"),
        ("h4", "ㆷ"),
    ]

    /// 將 TL 轉換為 TPS
    ///
    /// 用於候選詞顯示，將羅馬字轉為方音符號
    /// 連字符 "-" 會轉換為空白分隔
    ///
    /// - Parameter tl: TL 字串（如 "gua2-gua2"）
    /// - Returns: TPS 字串（如 "ㄍㄨㄚˋ ㄍㄨㄚˋ"）
    static func toTPS(_ tl: String) -> String {
        guard !tl.isEmpty else { return "" }

        // 先按連字符分割成音節，連字符轉為空白
        let syllables = tl.split(separator: "-", omittingEmptySubsequences: false)
        let result = syllables.map { convertSyllableToTPS(String($0)) }
        return result.joined(separator: " ")
    }

    /// 將單一 TL 音節轉換為 TPS
    private static func convertSyllableToTPS(_ syllable: String) -> String {
        guard !syllable.isEmpty else { return "" }

        var remaining = syllable.lowercased()
        var result = ""

        // 1. 先處理聲母
        for (tl, tps) in tlToTpsConsonants {
            if remaining.hasPrefix(tl) {
                // 特殊處理：tsi/tshi/si/ji 後面不能再接 i
                if ["tsi", "tshi", "si", "ji"].contains(tl) {
                    result += tps
                    remaining.removeFirst(tl.count)
                    break
                }
                result += tps
                remaining.removeFirst(tl.count)
                break
            }
        }

        // 2. 處理韻母（可能有多個韻母組合，如 iau）
        var vowelMatched = true
        while vowelMatched && !remaining.isEmpty {
            vowelMatched = false

            // 先檢查是否為入聲韻尾 + 聲調（如 p4, t8）
            for (tl, _) in tlToTpsCheckedTones {
                if remaining.hasSuffix(tl) {
                    // 入聲韻尾在最後處理
                    break
                }
            }

            // 匹配韻母
            for (tl, tps) in tlToTpsVowels {
                // 避免匹配到入聲韻尾的輔音
                let remainingWithoutTone = remaining.filter { !$0.isNumber }
                if remainingWithoutTone.hasPrefix(tl) && !["p", "t", "k", "h"].contains(tl) {
                    result += tps
                    remaining.removeFirst(tl.count)
                    vowelMatched = true
                    break
                } else if remaining.hasPrefix(tl) && !["p", "t", "k", "h", "m", "n"].contains(tl) {
                    result += tps
                    remaining.removeFirst(tl.count)
                    vowelMatched = true
                    break
                }
            }
        }

        // 3. 處理入聲韻尾 + 聲調（如 p4, t8, k4, h8）
        for (tl, tps) in tlToTpsCheckedTones {
            if remaining.hasSuffix(tl) {
                remaining.removeLast(tl.count)
                // 先處理剩餘的韻母
                for (tlV, tpsV) in tlToTpsVowels {
                    if remaining.hasPrefix(tlV) {
                        result += tpsV
                        remaining.removeFirst(tlV.count)
                        break
                    }
                }
                result += tps
                return result
            }
        }

        // 4. 處理一般聲調（1-9）
        if let lastChar = remaining.last, lastChar.isNumber {
            let tone = String(lastChar)
            remaining.removeLast()

            // 處理剩餘的韻母
            while !remaining.isEmpty {
                var matched = false
                for (tl, tps) in tlToTpsVowels {
                    if remaining.hasPrefix(tl) {
                        result += tps
                        remaining.removeFirst(tl.count)
                        matched = true
                        break
                    }
                }
                if !matched {
                    // 無法匹配，保留原字符
                    result.append(remaining.removeFirst())
                }
            }

            // 加上聲調符號
            for (tlTone, tpsTone) in tlToTpsTones {
                if tone == tlTone {
                    result += tpsTone
                    break
                }
            }
            // 1, 4 聲不加符號
        } else {
            // 無聲調數字，處理剩餘字符
            while !remaining.isEmpty {
                var matched = false
                for (tl, tps) in tlToTpsVowels {
                    if remaining.hasPrefix(tl) {
                        result += tps
                        remaining.removeFirst(tl.count)
                        matched = true
                        break
                    }
                }
                if !matched {
                    result.append(remaining.removeFirst())
                }
            }
        }

        return result
    }
}
