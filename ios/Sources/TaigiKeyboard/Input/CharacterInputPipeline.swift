import Foundation

/// Pure-function pipeline for TPS key-level character preprocessing.
///
/// Separates "what character should be emitted" and "does the raw buffer need a
/// retroactive last-char replacement" from the composing state machine. The caller
/// (`ActionHandler+KeyActions`) reads the result and issues `replaceLastCharacter`
/// when `replaceLast` is non-nil — no hidden mutations happen inside the pipeline.
///
/// Non-TPS input modes pass through unchanged.

// MARK: - Shared-Core Candidate
// Pure logic, Foundation-only. Eligible for cross-platform extraction.
enum CharacterInputPipeline {
    /// Result of TPS key-level adjustment.
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
    static func adjust(_ char: String, inputMode: InputMode, rawInput: String) -> Adjustment {
        guard inputMode == .tps else {
            return Adjustment(char: char, replaceLast: nil)
        }

        var adjusted = TPSInputAdjuster.adjustInitialKey(char, afterRawInput: rawInput)
        adjusted = TPSInputAdjuster.adjustNasalizedVowelKey(adjusted, afterRawInput: rawInput)

        let lastChar = rawInput.last
        let replaceLast = TPSInputAdjuster.syllabicNasalReplacement(forIncoming: adjusted, lastRawChar: lastChar)
            ?? TPSInputAdjuster.palatalizationReplacement(forIncoming: adjusted, lastRawChar: lastChar)

        return Adjustment(char: adjusted, replaceLast: replaceLast)
    }
}
