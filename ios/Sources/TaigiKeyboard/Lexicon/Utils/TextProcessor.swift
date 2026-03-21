import Foundation
import KeyboardKit
import OSLog

/// Candidate processing utilities
///
/// Provides capitalization, deduplication, scoring, and text classification.
enum CandidateProcessor {

    private static let logger = Logger(
        subsystem: "com.siansiansu.taigikeyboard",
        category: "CandidateProcessor"
    )

    // MARK: - Text Classification

    /// Check if a string contains Hanzi characters
    static func isHanzi(_ input: String) -> Bool {
        input.contains { char in
            guard let scalar = char.unicodeScalars.first else { return false }
            return (0x4E00 ... 0x9FFF).contains(scalar.value)
                || (0x3400 ... 0x4DBF).contains(scalar.value)
                || (0x20000 ... 0x2A6DF).contains(scalar.value)
                || (0x2A700 ... 0x2B73F).contains(scalar.value)
                || (0x2B740 ... 0x2B81F).contains(scalar.value)
                || (0x2B820 ... 0x2CEAF).contains(scalar.value)
        }
    }

    // MARK: - Capitalization

    /// 根據輸入文字的大小寫狀態，處理目標文字的大小寫
    static func capitalize(_ text: String, basedOn input: String) -> String {
        // 從 KeyboardKit 的持久化設定讀取
        let isAutoCap = KeyboardSettings.store.bool(
            forKey: "com.keyboardkit.settings.keyboard.isAutocapitalizationEnabled"
        )

        return CaseTransformer.capitalizeCandidate(
            text,
            basedOn: input,
            isAutoCapitalizationEnabled: isAutoCap,
            inputMode: SharedSettings.shared.inputMode
        )
    }

    /// 判斷文字是否以羅馬字母開頭
    static func startsWithRomanLetter(_ text: String) -> Bool {
        guard let firstChar = text.first else {
            return false
        }
        return firstChar.isLetter
    }

    // MARK: - Deduplication

    /// 移除重複的詞彙（基於 roman 和 hanzi 組合）
    static func removeDuplicates(_ words: [TaigiWord]) -> [TaigiWord] {
        var seen = Set<String>()
        var result: [TaigiWord] = []

        for word in words {
            let key = "\(word.roman)|\(word.hanzi ?? "")"
            if seen.contains(key) { continue }
            seen.insert(key)
            result.append(word)
        }

        return result
    }

    // MARK: - Sorting

    /// 計算候選詞排序分數
    ///
    /// 公式（v4，與 Android 一致）：
    /// `userFreqScore + recencyBonus + exactBonus + closenessBonus + baseFreqScore`
    ///
    /// 設計理念：
    /// - userFreqScore 主導排序（穩定性優先）
    /// - closenessBonus 讓長度較接近輸入的候選詞排序較前（cold-start 主要因素）
    /// - recencyBonus 和 exactBonus 只做微調（不會讓低頻詞超過高頻詞）
    ///
    /// - Parameters:
    ///   - word: 候選詞
    ///   - normalizedInput: 正規化後的輸入（小寫、去連字符）
    ///   - frequencyData: 使用者頻率資料（包含 count 和 lastUsedMillis）
    /// - Returns: 排序分數（越高越優先）
    static func calculateScore(
        word: TaigiWord,
        normalizedInput: String,
        frequencyData: UserFrequencyService.FrequencyData
    ) -> Int {
        // Normalize both sides to base form (no tones, no hyphens) for comparison
        let candidateBase = romanToBase(word.roman)
        let inputBase = inputToBase(normalizedInput)

        // 使用者頻率（主導因素，上限 100，max 10000）
        let cappedUserFreq = min(frequencyData.count, 100)
        let userFreqScore = cappedUserFreq * 100

        // Recency 加分（微調，最近 1 小時內用過 +200）
        let currentTime = Int64(Date().timeIntervalSince1970 * 1000)
        let oneHourMillis: Int64 = 60 * 60 * 1000
        let recencyBonus: Int
        if frequencyData.lastUsedMillis > 0 &&
            (currentTime - frequencyData.lastUsedMillis) < oneHourMillis {
            recencyBonus = 200
        } else {
            recencyBonus = 0
        }

        // 完全匹配加分（微調，+100）
        let exactBonus = (candidateBase == inputBase) ? 100 : 0

        // Completion penalty: penalize candidates extending beyond input (aligned with RIME/Mozc)
        // Ensures exact matches rank above completions in cold-start;
        // user frequency (~15+ uses) can still overcome this penalty
        let completionPenalty = (candidateBase != inputBase) ? -1000 : 0

        // Match closeness bonus (0-500): reward candidates whose length matches input
        let inputLen = max(inputBase.count, 1)
        let candidateLen = max(candidateBase.count, 1)
        let matchRatio = Double(min(inputLen, candidateLen)) / Double(max(inputLen, candidateLen))
        let closenessBonus = Int(matchRatio * 500)

        // 詞庫頻率（新詞 fallback，約 0-100）
        let baseFreqScore = (word.lengthScore ?? 0) / 10

        return userFreqScore + recencyBonus + exactBonus + closenessBonus + baseFreqScore + completionPenalty
    }

    // MARK: - Base Form Helpers

    /// Strip roman to base form for matching (no tones, no hyphens/spaces, lowercase)
    /// "tāi-tsì" → "taitsi", "m̄ bat" → "mbat"
    private static func romanToBase(_ roman: String) -> String {
        let noHyphens = roman.replacingOccurrences(of: "-", with: "")
            .replacingOccurrences(of: " ", with: "")
        let withNasal = noHyphens
            .replacingOccurrences(of: "\u{207F}", with: "nn")
            .replacingOccurrences(of: "\u{1D3A}", with: "nn")
        let nfd = withNasal.decomposedStringWithCanonicalMapping
        let withOo = nfd.replacingOccurrences(of: "\u{0358}", with: "o")
        // Strip combining marks (tone diacritics) and tone digits
        let stripped = withOo.unicodeScalars.filter {
            $0.properties.generalCategory != .nonspacingMark
        }
        return String(String.UnicodeScalarView(stripped))
            .filter { !$0.isNumber }
            .lowercased()
    }

    /// Strip tone digits from normalized input
    /// "tai5tsi3" → "taitsi", "taitsi" → "taitsi"
    private static func inputToBase(_ normalizedInput: String) -> String {
        normalizedInput.filter { !$0.isNumber }.lowercased()
    }

    /// 根據分數排序詞彙（與 Android 一致）
    /// - Parameters:
    ///   - words: 要排序的詞彙陣列
    ///   - normalizedInput: 正規化後的輸入（用於完全匹配判斷）
    ///   - frequencyDataMap: 詞彙頻率資料字典
    /// - Returns: 排序後的詞彙陣列
    static func sortByScore(
        _ words: [TaigiWord],
        normalizedInput: String,
        frequencyDataMap: [String: UserFrequencyService.FrequencyData]
    ) -> [TaigiWord] {
        let scored = words.map { word -> (TaigiWord, Int) in
            let freq = frequencyDataMap[word.displayText] ?? .empty
            let score = calculateScore(word: word, normalizedInput: normalizedInput, frequencyData: freq)
            return (word, score)
        }
        let sorted = scored.sorted { $0.1 > $1.1 }

        #if DEBUG
        logScoreDetails(sorted: sorted, normalizedInput: normalizedInput, frequencyDataMap: frequencyDataMap)
        #endif

        return sorted.map { $0.0 }
    }

    #if DEBUG
    /// Log score breakdown for each candidate (visible in Console.app)
    private static func logScoreDetails(
        sorted: [(TaigiWord, Int)],
        normalizedInput: String,
        frequencyDataMap: [String: UserFrequencyService.FrequencyData]
    ) {
        let inputBase = inputToBase(normalizedInput)
        let currentTime = Int64(Date().timeIntervalSince1970 * 1000)
        let oneHourMillis: Int64 = 60 * 60 * 1000

        for (word, total) in sorted {
            let freq = frequencyDataMap[word.displayText] ?? .empty
            let candidateBase = romanToBase(word.roman)
            let userFreqScore = min(freq.count, 100) * 100
            let recency = (freq.lastUsedMillis > 0 && (currentTime - freq.lastUsedMillis) < oneHourMillis) ? 200 : 0
            let exact = (candidateBase == inputBase) ? 100 : 0
            let completion = (candidateBase != inputBase) ? -1000 : 0
            let inputLen = max(inputBase.count, 1)
            let candidateLen = max(candidateBase.count, 1)
            let closeness = Int(Double(min(inputLen, candidateLen)) / Double(max(inputLen, candidateLen)) * 500)
            let base = (word.lengthScore ?? 0) / 10

            logger.debug("[SCORE] input='\(normalizedInput, privacy: .public)' | \(word.roman, privacy: .public) \(word.hanzi ?? "", privacy: .public): user=\(userFreqScore) recency=\(recency) exact=\(exact) close=\(closeness) base=\(base) completion=\(completion) total=\(total)")
        }
    }
    #endif


}
