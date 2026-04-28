import Foundation

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only.

/// Candidate-processing utilities retained on the iOS platform side.
///
/// CROSS-PLATFORM INVARIANT — the score/sort/tier math originally living
/// in this file moved to the Rust shared core in v3.5.2 (slice
/// `phase4b/ranking-slice`). Production ranking now flows through
/// `RustEngineBridge.processCandidates(...)` → `engine/ranking/`. The
/// helpers that remain here are either out-of-slice on iOS
/// (`isHanzi` / `capitalize` / `startsWithRomanLetter` — used outside
/// `LexiconService`) or used by the cold-start fallback in
/// `LexiconService.processCandidates` when the user-frequency DB has not
/// yet warmed up. Drift between this file and `engine/ranking/` causes
/// silent ranking divergence; mirror behavior changes both places.
enum CandidateProcessor {
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

    // MARK: - Deduplication (cold-start fallback)

    /// 移除重複的詞彙（基於 roman 和 hanzi 組合）
    ///
    /// Only the cold-start branch of `LexiconService.processCandidates`
    /// (user-frequency DB not yet connected) reaches this helper. The
    /// connected branch routes through `RustEngineBridge.processCandidates`
    /// where `engine/ranking/dedup.rs` runs the same logic.
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
    ///
    /// Cold-start fallback only — see `removeDuplicates` doc.
    static func removeDisplayDuplicates(_ words: [TaigiWord]) -> [TaigiWord] {
        var seenHanzi = Set<String>()
        return words.filter { word in
            guard let hanzi = word.hanzi, !hanzi.isEmpty else {
                return true
            }
            return seenHanzi.insert(hanzi).inserted
        }
    }
}
