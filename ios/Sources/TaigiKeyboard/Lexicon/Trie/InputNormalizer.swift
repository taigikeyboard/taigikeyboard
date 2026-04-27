import Foundation

// MARK: - Shared-Core Candidate
// Pure logic, Foundation-only. Eligible for cross-platform extraction.

/// Input normalizer — thin wrapper over `RustEngineBridge` after D9.4.
///
/// Converts user input to numeric tone format (mode-native spelling).
/// The full pipeline (TPS preprocess + per-syllable diacritic→digit +
/// checked-ending heuristic) lives in Rust as `Method::NormalizeInput`. The
/// `mode` parameter is no longer used by the engine (Codex v3 §2 — both
/// platforms ignored it pre-D9.4) but is kept on the public signature
/// for call-site source compatibility; remove in a follow-up sweep.
enum InputNormalizer {
    /// Normalize input to Trie query format (TL numeric tones).
    static func normalize(_ input: String, mode _: InputMode) -> String {
        guard !input.isEmpty else { return "" }
        return RustEngineBridge.normalizeInput(input)
    }

    /// Check if input contains tone mark diacritics.
    static func hasToneMarks(_ input: String) -> Bool {
        RustEngineBridge.hasToneMarks(input)
    }
}
