import Foundation

/// POJ / TL romanization converter
///
/// Thin wrapper around TaigiPhonetics for POJ↔TL conversion.

// MARK: - Shared-Core Candidate
// Pure logic, Foundation-only. Eligible for cross-platform extraction.
enum RomanizationConverter {

    /// Convert TL diacritic text (with hyphens, from DB) to POJ diacritic text.
    static func tlToPOJ(_ text: String) -> String {
        TaigiPhonetics.tlDisplayToPOJDisplay(text)
    }

    /// Convert POJ diacritic text (with hyphens) to TL diacritic text.
    static func pojToTL(_ text: String) -> String {
        TaigiPhonetics.pojDisplayToTLDisplay(text)
    }
}
