import Foundation

/// 方音符號（TPS）轉換器
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
        // Standalone palatalized consonants (for nasalized vowel combinations like ㄐㆪ)
        ("ㄐ", "ts"),
        ("ㄑ", "tsh"),
        ("ㄒ", "s"),
        ("ㆢ", "j"),
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
    /// 入聲韻尾（ㆴㆵㆻㆷ）會同時影響韻尾和聲調
    private static let tones: [(tps: String, tl: String, tone: String)] = [
        // 入聲韻尾（帶點為第8聲，無點為第4聲）
        ("ㆴ˙", "p", "8"),  // p + 第8聲
        ("ㆵ˙", "t", "8"),  // t + 第8聲
        ("ㆻ˙", "k", "8"),  // k + 第8聲
        ("ㆷ˙", "h", "8"),  // h + 第8聲
        ("ㆴ", "p", "4"),   // p + 第4聲
        ("ㆵ", "t", "4"),   // t + 第4聲
        ("ㆻ", "k", "4"),   // k + 第4聲
        ("ㆷ", "h", "4"),   // h + 第4聲
    ]

    /// 聲調符號（非入聲）
    private static let toneMarks: [(tps: String, tone: String)] = [
        ("ˋ", "2"),   // 第2聲
        ("˪", "3"),   // 第3聲
        ("ˊ", "5"),   // 第5聲
        ("ˇ", "6"),   // 第6聲
        ("˫", "7"),   // 第7聲
        ("ˆ", "9"),   // 第9聲
        ("˙", "8"),   // 第8聲（非入聲），U+02D9 DOT ABOVE
        // 第1聲無符號
    ]

    /// Stop consonants excluded from vowel matching (already consumed as checked tone finals)
    private static let stopConsonants: Set<String> = ["p", "t", "k", "h"]

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

    // MARK: - TPS Initial Key Auto-Selection

    /// Characters that indicate the start of a new syllable (tone marks and checked tone finals)
    private static let syllableBoundaryChars: Set<Character> = {
        var chars = Set<Character>()
        // Tone marks: ˋ ˪ ˊ ˇ ˫ ˙ ˆ
        for (tps, _) in toneMarks {
            chars.formUnion(tps)
        }
        // Checked tone finals: ㆴ ㆵ ㆻ ㆷ
        chars.insert("ㆴ")
        chars.insert("ㆵ")
        chars.insert("ㆻ")
        chars.insert("ㆷ")
        return chars
    }()

    /// Returns context-adjusted TPS character for ㄇ/ㄫ keys.
    /// Only called when inputMode == .tps.
    ///
    /// At syllable start (empty, after tone mark, after checked final, after space) → initial form.
    /// After ㄧ: ㄇ→ㆬ, ㄫ→ㄥ (ing special case).
    /// Other positions: ㄇ→ㆬ, ㄫ→ㆭ.
    static func adjustTPSInitialKey(_ char: String, afterRawInput raw: String) -> String {
        guard char == "ㄇ" || char == "ㄫ" else { return char }

        // At syllable start → keep initial form
        if raw.isEmpty { return char }
        let lastChar = raw.last!
        if lastChar == " " || syllableBoundaryChars.contains(lastChar) {
            return char
        }

        // Not at syllable start → final/syllabic form
        if char == "ㄇ" {
            return "ㆬ"
        }
        // char == "ㄫ": after ㄧ → ㄥ (ing), otherwise → ㆭ
        return lastChar == "ㄧ" ? "ㄥ" : "ㆭ"
    }

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

        // Post-process: oo before stop tone → o (e.g., "ook4" → "ok4", "oot8" → "ot8")
        // Reference: taigi-converter fromZhuyin rule
        return result.replacingOccurrences(
            of: #"oo([ptk][48])"#,
            with: "o$1",
            options: .regularExpression
        )
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
    private static let tlToTPSConsonants: [(tl: String, tps: String)] = [
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
    private static let tlToTPSVowels: [(tl: String, tps: String)] = [
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
        ("or", "ㄛ"),
        ("ai", "ㄞ"),
        ("au", "ㄠ"),
        ("am", "ㆰ"),
        ("om", "ㆱ"),
        ("an", "ㄢ"),
        ("ng", "ㆭ"),
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
    private static let tlToTPSTones: [(tl: String, tps: String)] = [
        ("2", "ˋ"),
        ("3", "˪"),
        ("5", "ˊ"),
        ("6", "ˇ"),
        ("7", "˫"),
        ("8", "\u{02D9}"),  // Non-stop tone 8 (U+02D9 DOT ABOVE)
        ("9", "ˆ"),
        // 1, 4 無符號
    ]

    /// TL → TPS 入聲韻尾對照表
    private static let tlToTPSCheckedTones: [(tl: String, tps: String)] = [
        ("p8", "ㆴ˙"),
        ("t8", "ㆵ˙"),
        ("k8", "ㆻ˙"),
        ("h8", "ㆷ˙"),
        ("p4", "ㆴ"),
        ("t4", "ㆵ"),
        ("k4", "ㆻ"),
        ("h4", "ㆷ"),
    ]

    /// 將 TL 轉換為 TPS
    ///
    /// 用於候選詞顯示，將羅馬字轉為方音符號
    /// 連字符 "-" 會轉換為空白分隔
    ///
    /// - Parameters:
    ///   - tl: TL 字串（如 "gua2-gua2"）
    ///   - orMapsToER: true 時 or → ㄜ（方言變體），false 時 or → ㄛ（預設）
    /// - Returns: TPS 字串（如 "ㄍㄨㄚˋ ㄍㄨㄚˋ"）
    static func toTPS(_ tl: String, orMapsToER: Bool = false) -> String {
        guard !tl.isEmpty else { return "" }

        // 先按連字符分割成音節，連字符轉為空白
        let syllables = tl.split(separator: "-", omittingEmptySubsequences: false)
        let result = syllables.map { convertSyllableToTPS(String($0), orMapsToER: orMapsToER) }
        return result.joined(separator: " ")
    }

    /// Convert a single TL syllable to TPS.
    ///
    /// Separates consonant, vowel, and tone parts for correct post-processing:
    /// - Standalone m/ng → syllabic form (ㄇ→ㆬ, ㄫ→ㆭ)
    /// - Palatalized initial + ㄣㄣ → nasalized ㆪ (e.g. tsinn→ㄐㆪ)
    private static func convertSyllableToTPS(_ syllable: String, orMapsToER: Bool = false) -> String {
        guard !syllable.isEmpty else { return "" }

        var remaining = syllable.lowercased()
        var consonant = ""
        var vowel = ""
        var tone = ""

        // 1. Match consonant (longest match first, table is pre-sorted)
        for (tl, tps) in tlToTPSConsonants {
            if remaining.hasPrefix(tl) {
                consonant = tps
                remaining.removeFirst(tl.count)
                break
            }
        }

        // 2. Extract checked tone suffix (p4/t4/k4/h4/p8/t8/k8/h8)
        for (tl, tps) in tlToTPSCheckedTones {
            if remaining.hasSuffix(tl) {
                remaining.removeLast(tl.count)
                tone = tps
                break
            }
        }

        // 3. Extract general tone digit if no checked tone was found
        if tone.isEmpty, let lastChar = remaining.last, lastChar.isNumber {
            let toneDigit = String(lastChar)
            remaining.removeLast()
            for (tlTone, tpsTone) in tlToTPSTones {
                if toneDigit == tlTone {
                    tone = tpsTone
                    break
                }
            }
        }

        // 4. Match vowels from remaining (greedy, longest match first)
        while !remaining.isEmpty {
            var matched = false
            for (tl, tps) in tlToTPSVowels {
                if remaining.hasPrefix(tl) && !stopConsonants.contains(tl) {
                    let effectiveTPS = (orMapsToER && tl == "or") ? "ㄜ" : tps
                    vowel += effectiveTPS
                    remaining.removeFirst(tl.count)
                    matched = true
                    break
                }
            }
            if !matched {
                vowel.append(remaining.removeFirst())
            }
        }

        // 5. Post-processing: standalone m/ng → syllabic form
        if vowel.isEmpty {
            if consonant == "ㄇ" { consonant = ""; vowel = "ㆬ" }
            else if consonant == "ㄫ" { consonant = ""; vowel = "ㆭ" }
        }

        // 5b. Post-processing: ing special case — ㄧㆭ → ㄧㄥ
        if vowel == "ㄧㆭ" { vowel = "ㄧㄥ" }

        // 6. Post-processing: palatalized initial + ㄣㄣ → nasalized ㆪ
        if consonant.hasSuffix("ㄧ") && vowel == "ㄣㄣ" {
            consonant = String(consonant.dropLast())
            vowel = "ㆪ"
        }

        // 7. Post-processing: o → oo before stop tone (e.g., "ok4" → ㆦㆻ not ㄛㆻ)
        // Reference: taigi-converter toZhuyin — if vowel contains ㄛ and tone is p/t/k checked,
        // replace ㄛ with ㆦ (the "oo" form is used before stop consonants)
        if vowel.contains("ㄛ") && !tone.isEmpty {
            let first = tone.unicodeScalars.first!.value
            // ㆴ = U+31B4 (p), ㆵ = U+31B5 (t), ㆻ = U+31BB (k)
            if first == 0x31B4 || first == 0x31B5 || first == 0x31BB {
                vowel = vowel.replacingOccurrences(of: "ㄛ", with: "ㆦ")
            }
        }

        return consonant + vowel + tone
    }
}
