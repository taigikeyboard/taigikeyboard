// 中文: 候選詞處理的薄包裝層 — 主要邏輯都遷到 Rust,iOS 端只剩 capitalize 入口
// 中文: 與羅馬字母首字判斷。

import Foundation

/// Candidate-processing utilities retained on the iOS platform side.
///
/// The score / sort / tier / dedup math moved to the Rust shared core in
/// v3.5.2 (`engine/ranking/`); the cold-start dedup helpers were removed
/// in the v3.5.3 follow-up (PR #192) once `LexiconService` cold-start
/// started routing through
/// `RustEngineBridge.processCandidates(..., mergeOrderOnly: true)`. The
/// hanzi-range check moved to `engine/lexicon::classification` in v3.5.7
/// (`RustEngineBridge.isHanzi`). Candidate capitalization moved to the
/// Rust shared core in the case-transform slice
/// (`RustEngineBridge.capitalizeCandidate`); this file now only retains
/// `startsWithRomanLetter` which is a 3-line predicate.
// 中文: 平台端只剩兩個薄包裝 — capitalize 轉發到 Rust,
// 中文: startsWithRomanLetter 為 3 行平台端判斷,沒搬到 Rust 的價值。
enum CandidateProcessor {
    static func capitalize(_ text: String, basedOn input: String, inputMode: InputMode, isAutoCap: Bool) -> String {
        RustEngineBridge.capitalizeCandidate(
            text,
            basedOn: input,
            autoCapEnabled: isAutoCap,
            mode: inputMode,
        )
    }

    static func startsWithRomanLetter(_ text: String) -> Bool {
        guard let firstChar = text.first else {
            return false
        }
        return firstChar.isLetter
    }
}
