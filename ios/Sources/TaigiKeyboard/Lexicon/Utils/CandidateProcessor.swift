import Foundation

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only.

/// Candidate-processing utilities retained on the iOS platform side.
///
/// The score / sort / tier / dedup math moved to the Rust shared core in
/// v3.5.2 (`engine/ranking/`); the cold-start dedup helpers were removed
/// in the v3.5.3 follow-up (PR #192) once `LexiconService` cold-start
/// started routing through
/// `RustEngineBridge.processCandidates(..., mergeOrderOnly: true)`. The
/// remaining helpers (`isHanzi`, `capitalize`, `startsWithRomanLetter`)
/// are platform-specific text classification / orchestration used outside
/// the lexicon ranking pipeline.
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

}
