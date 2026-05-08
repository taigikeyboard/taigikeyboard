// 中文: TPS 鍵層字元前處理的純函式管線。
// 中文: 把「要 emit 哪個字元」與「raw buffer 是否要回溯改寫最後一個字元」與組字 state machine 分離。

import Foundation

// Pure-function pipeline for TPS key-level character preprocessing.
//
// Separates "what character should be emitted" and "does the raw buffer need a
// retroactive last-char replacement" from the composing state machine. The caller
// (`ActionHandler+KeyActions`) reads the result and issues `replaceLastCharacter`
// when `replaceLast` is non-nil — no hidden mutations happen inside the pipeline.
//
// Non-TPS input modes pass through unchanged.

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.
// 中文: 純邏輯模組,只用 Foundation;符合跨平台抽取候選條件。
enum CharacterInputPipeline {
    /// Result of TPS key-level adjustment.
    // 中文: TPS 鍵層調整的回傳值。replaceLast 非 nil 時,呼叫端要先把 raw 最後一個字元換掉再 append。
    struct Adjustment {
        /// Character to process (context-adjusted for TPS, original otherwise).
        let char: String

        /// If non-nil, the caller should replace the last char of raw input with this value
        /// before appending `char`. Used for palatalization / syllabic-nasal auto-correct.
        let replaceLast: String?
    }

    /// Apply TPS key-level adjustments to an incoming character.
    ///
    /// For `inputMode == .tps`, runs in order:
    /// 1. `adjustInitialKey` — dual-form nasal/stop positional adjust
    /// 2. `adjustNasalizedVowelKey` — ㆮ → ㆯ after ㄧ
    /// 3. `syllabicNasalReplacement` — ㄇ/ㄫ + tone mark (at most one replacement)
    /// 4. `palatalizationReplacement` — ㄗ/ㄘ/ㄙ/ㆡ + ㄧ/ㆪ (at most one replacement)
    ///
    /// Steps 3 and 4 have disjoint trigger conditions (different "last char" sets),
    /// so at most one `replaceLast` is ever returned.
    ///
    /// For other input modes, returns the character unchanged.
    // 中文: 對輸入字元套用 TPS 鍵層調整。非 TPS 模式直接原樣回傳。
    // 中文: 實際邏輯走 RustEngineBridge.tpsInputAdjust 在 Rust 端處理。
    static func adjust(_ char: String, inputMode: InputMode, rawInput: String) -> Adjustment {
        guard inputMode == .tps else {
            return Adjustment(char: char, replaceLast: nil)
        }
        let result = RustEngineBridge.tpsInputAdjust(incoming: char, rawInput: rawInput)
        return Adjustment(char: result.adjusted, replaceLast: result.replaceLast)
    }
}
