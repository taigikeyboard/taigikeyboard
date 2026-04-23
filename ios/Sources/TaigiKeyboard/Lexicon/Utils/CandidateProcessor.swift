import Foundation

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.

/// Candidate processing utilities
///
/// Provides capitalization, deduplication, scoring, and text classification.
enum CandidateProcessor {
    private static var logger: LoggerBackend {
        LoggerFactory.make(category: "CandidateProcessor")
    }

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
    ///
    /// Both `inputMode` and `isAutoCap` are supplied by the caller
    /// (typically forwarded from `LexiconService.search`) so this function
    /// stays Foundation-pure — no `SharedSettings.shared`, no
    /// `KeyboardSettings.store`.
    static func capitalize(_ text: String, basedOn input: String, inputMode: InputMode, isAutoCap: Bool) -> String {
        CaseTransformer.capitalizeCandidate(
            text,
            basedOn: input,
            isAutoCapitalizationEnabled: isAutoCap,
            inputMode: inputMode,
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

    /// Remove visual duplicates for TPS mode (dedup by hanzi only).
    /// Words without hanzi are always kept (they display as TPS symbols, unique by roman).
    /// Must be called AFTER sorting so the highest-ranked entry for each hanzi is kept.
    static func removeDisplayDuplicates(_ words: [TaigiWord]) -> [TaigiWord] {
        var seenHanzi = Set<String>()
        return words.filter { word in
            guard let hanzi = word.hanzi, !hanzi.isEmpty else {
                return true
            }
            return seenHanzi.insert(hanzi).inserted
        }
    }

    // MARK: - Source Tier

    /// Maps a dictionary-source bit to a `baseFreqScore` multiplier numerator.
    /// CROSS-PLATFORM INVARIANT — mirrors `dictionary/common/source_bits.py`
    /// (SOURCE_TIERS + TIER_DENOMINATOR) and
    /// `android/.../CandidateProcessor.kt` SOURCE_TIERS. First-match-wins
    /// on overlapping bits. Drift causes silent ranking divergence.
    private struct SourceTier {
        let bit: Int
        let numerator: Int
    }

    private static let SOURCE_TIERS: [SourceTier] = [
        SourceTier(bit: 0, numerator: 15), // kautian
        SourceTier(bit: 1, numerator: 13), // taigitv
        SourceTier(bit: 7, numerator: 12), // stti
        SourceTier(bit: 6, numerator: 11), // kungge
    ]
    private static let DEFAULT_TIER_NUMERATOR = 10
    private static let TIER_DENOMINATOR = 10

    private static func tierNumerator(for bitmask: UInt16?) -> Int {
        guard let bitmask else { return DEFAULT_TIER_NUMERATOR }
        for tier in SOURCE_TIERS where (bitmask & (1 << tier.bit)) != 0 {
            return tier.numerator
        }
        return DEFAULT_TIER_NUMERATOR
    }

    // MARK: - Score Breakdown

    /// Breakdown of candidate score components (single source of truth).
    /// Used by both sorting and debug logging — no recalculation needed.
    struct ScoreBreakdown {
        let userFreqScore: Int
        let recencyBonus: Int
        let exactBonus: Int
        let completionPenalty: Int
        let closenessBonus: Int
        let baseFreqScore: Int

        var total: Int {
            userFreqScore + recencyBonus + exactBonus + completionPenalty + closenessBonus + baseFreqScore
        }
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
    /// - Returns: 分數分解（ScoreBreakdown）
    static func calculateScore(
        word: TaigiWord,
        normalizedInput: String,
        frequencyData: FrequencyData,
        currentTime: Int64,
    ) -> ScoreBreakdown {
        // Normalize both sides to base form (no tones, no hyphens) for comparison
        let candidateBase = romanToBase(word.roman)
        let inputBase = inputToBase(normalizedInput)

        // 使用者頻率（主導因素，上限 100，max 10000）
        let cappedUserFreq = min(frequencyData.count, 100)
        let userFreqScore = cappedUserFreq * 100

        // Recency 加分（微調，最近 1 小時內用過 +200）
        let oneHourMillis: Int64 = 60 * 60 * 1000
        let recencyBonus = if frequencyData.lastUsedMillis > 0 &&
            (currentTime - frequencyData.lastUsedMillis) < oneHourMillis
        {
            200
        } else {
            0
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

        // 詞庫頻率（新詞 fallback，約 0-100）+ tier bonus (kautian/taigitv/stti/kungge)
        let rawBase = (word.lengthScore ?? 0) / 10
        let baseFreqScore = rawBase * Self.tierNumerator(for: word.sourceBitmask) / Self.TIER_DENOMINATOR

        return ScoreBreakdown(
            userFreqScore: userFreqScore,
            recencyBonus: recencyBonus,
            exactBonus: exactBonus,
            completionPenalty: completionPenalty,
            closenessBonus: closenessBonus,
            baseFreqScore: baseFreqScore,
        )
    }

    // MARK: - Base Form Helpers

    /// Strip roman to base form for matching (no tones, no hyphens/spaces, lowercase)
    /// "tāi-tsì" → "taitsi", "m̄ bat" → "mbat"
    private static func romanToBase(_ roman: String) -> String {
        let noHyphens = roman
            .replacingOccurrences(of: "-", with: "")
            .replacingOccurrences(of: " ", with: "")
        let withOo = TaigiUnicode.nfdPreprocessed(noHyphens)
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
    ///   - currentTime: Unix millis used for the recency-bonus window.
    ///     Swift evaluates default arguments at **call time**, so production
    ///     callers get the current clock without supplying a value; tests
    ///     should pass a fixed value to pin the window deterministically.
    /// - Returns: 排序後的詞彙陣列
    static func sortByScore(
        _ words: [TaigiWord],
        normalizedInput: String,
        frequencyDataMap: [String: FrequencyData],
        currentTime: Int64 = Int64(Date().timeIntervalSince1970 * 1000),
    ) -> [TaigiWord] {
        let scored = words.map { word -> (TaigiWord, ScoreBreakdown) in
            let freq = frequencyDataMap[word.displayText] ?? .empty
            let breakdown = calculateScore(
                word: word, normalizedInput: normalizedInput,
                frequencyData: freq, currentTime: currentTime,
            )
            return (word, breakdown)
        }
        let sorted = scored.sorted { $0.1.total > $1.1.total }

        #if DEBUG
            logScoreDetails(sorted: sorted, normalizedInput: normalizedInput)
        #endif

        return sorted.map(\.0)
    }

    /// Log score breakdown for each candidate (visible in Console.app)
    private static func logScoreDetails(
        sorted: [(TaigiWord, ScoreBreakdown)],
        normalizedInput: String,
    ) {
        for (word, b) in sorted {
            logger.debug("[SCORE] input='\(normalizedInput)' | \(word.roman) \(word.hanzi ?? ""): user=\(b.userFreqScore) recency=\(b.recencyBonus) exact=\(b.exactBonus) close=\(b.closenessBonus) base=\(b.baseFreqScore) completion=\(b.completionPenalty) total=\(b.total)")
        }
    }
}
