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
enum CandidateProcessor {
    static func capitalize(_ text: String, basedOn input: String, inputMode: InputMode, isAutoCap: Bool) -> String {
        RustEngineBridge.capitalizeCandidate(
            text,
            basedOn: input,
            autoCapEnabled: isAutoCap,
            mode: inputMode
        )
    }

    static func startsWithRomanLetter(_ text: String) -> Bool {
        guard let firstChar = text.first else {
            return false
        }
        return firstChar.isLetter
    }
}
