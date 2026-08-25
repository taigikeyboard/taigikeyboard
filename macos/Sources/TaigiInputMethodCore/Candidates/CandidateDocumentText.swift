// Renders one candidate as the string that goes into the user's document.

import Foundation

/// Turns a candidate into document text according to the output settings.
///
/// Separate from the candidate's own `displayText`, which is the engine's
/// canonical form (`hanji ?? roman`) and pairs with `canonicalTl` to identify
/// the word the user's frequency and association data are learnt under — the
/// identity is that PAIR, never either field alone, because one Hanji has
/// several readings and they are different morphemes (Core Principle #7).
/// Document text and identity are different strings on purpose: what the
/// document gets depends on how the user has chosen to see Taigi written, while
/// which word it is does not.
enum CandidateDocumentText {
    /// CROSS-PLATFORM INVARIANT — mirrors
    /// ios/Sources/TaigiKeyboard/Actions/ActionHandler+Suggestions.swift:218.
    /// Drift changes what a candidate writes into the document.
    ///
    /// The TPS branch of the iOS original is deliberately absent: macOS ships TL
    /// and POJ only (`docs/architecture/macos-roadmap.md` § Goal), so there is no
    /// layout that could ask for bracketed Bopomofo.
    static func text(
        for candidate: ContinuousCandidate,
        settings: EngineSettings,
    ) -> String {
        // A romanization-only candidate is marked by an absent hanji, but a
        // producer that emitted an empty string instead means the same thing —
        // and rendering "guá ()" for it would be a visible defect.
        guard let hanji = candidate.hanji, !hanji.isEmpty else { return candidate.roman }

        if settings.isOutputBothScripts {
            return settings.isTranslateSwapped
                ? "\(hanji) (\(candidate.roman))"
                : "\(candidate.roman) (\(hanji))"
        }
        return settings.isTranslateSwapped ? hanji : candidate.roman
    }
}
