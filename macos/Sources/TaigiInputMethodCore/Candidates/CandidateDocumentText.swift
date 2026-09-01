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

    /// The script `text(for:settings:)` does NOT lead with, or nil when this
    /// candidate has only one.
    ///
    /// What Space commits, so that 漢羅 — Taiwanese written with Han characters
    /// and romanization mixed inside one sentence — costs one key per word
    /// instead of a round trip through the output setting. Typing `我ê名` in
    /// 漢字 mode was ↩ / `` ` `` ↩ `` ` `` / ↩; it is now ↩ / Space / ↩, and the
    /// mode never moves.
    ///
    /// Verbatim: whatever the field holds is what the document gets. The hanji
    /// field legitimately carries hyphens — 298 dictionary entries write the
    /// 輕聲 marker into it (`交--人`, `彼--的`), which is MOE orthography and is
    /// pinned by §21/S8, and the 漢羅 mixed entries (`紅kì-kì`) carry the
    /// romanized half's own hyphen. Stripping either would be this layer
    /// second-guessing the dictionary.
    ///
    /// Read off the same settings the cell was built from — the effective swap
    /// — so Space writes exactly the script the user can see offered beside
    /// (漢羅並排's annotation) or next to (漢羅合用's adjacent cell) the one the
    /// highlight is on. Under 合用 a romanization cell is itself `.alternate`,
    /// and Space on it flips back to `.primary` (`PresentedCandidate`), which
    /// is how the same rule gives Space the Hanji there.
    ///
    /// `isOutputBothScripts` is not read: it describes how to write a candidate
    /// carrying BOTH scripts, and this is the request for the other one BY
    /// ITSELF.
    ///
    /// nil when the candidate has one script — the §34 literal candidate, an
    /// out-of-vocabulary name — or when the display shows no Hanji at all
    /// (羅馬字): answering with the romanization again would make Space a
    /// slower Return, and writing Hanji the user never saw would be worse.
    static func alternateText(
        for candidate: ContinuousCandidate,
        settings: EngineSettings,
    ) -> String? {
        guard let hanji = candidate.hanji, !hanji.isEmpty,
              settings.candidateDisplayMode.showsHanji
        else { return nil }
        return settings.isTranslateSwapped ? candidate.roman : hanji
    }
}
