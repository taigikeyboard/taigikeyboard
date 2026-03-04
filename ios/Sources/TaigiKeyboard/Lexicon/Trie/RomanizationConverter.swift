import Foundation

/// POJ / TL romanization converter
///
/// Thin wrapper around TaigiPhonetics for POJ↔TL conversion.
enum RomanizationConverter {

    /// Convert TL diacritic text (with hyphens, from DB) to POJ diacritic text.
    static func tlToPOJ(_ text: String) -> String {
        TaigiPhonetics.tlDisplayToPOJDisplay(text)
    }
}
