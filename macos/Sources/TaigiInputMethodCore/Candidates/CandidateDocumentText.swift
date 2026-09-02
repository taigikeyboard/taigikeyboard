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
        // No presentable Hanji: the romanization alone — rendering "guá ()"
        // for it would be a visible defect.
        guard let hanji = candidate.presentableHanji else { return candidate.roman }

        if settings.isOutputBothScripts {
            return settings.isTranslateSwapped
                ? "\(hanji) (\(candidate.roman))"
                : "\(candidate.roman) (\(hanji))"
        }
        return settings.isTranslateSwapped ? hanji : candidate.roman
    }

    /// The script `text(for:settings:)` does NOT lead with — what Space
    /// commits, so 漢羅 (Han characters and romanization mixed in one
    /// sentence) costs one key per word and the mode never moves. Read off
    /// the same settings the cell was built from, so Space writes exactly the
    /// script the user can see offered beside the highlighted one;
    /// `isOutputBothScripts` is not read — it says how to show a candidate
    /// carrying BOTH scripts, and this asks for the other one BY ITSELF.
    ///
    /// Verbatim, hyphens included: 298 dictionary entries write the 輕聲 `--`
    /// into the hanji field (`交--人`, MOE orthography, pinned by §21/S8) and
    /// the 漢羅 mixed entries (`紅kì-kì`) carry the romanized half's own
    /// hyphen — stripping either would be this layer second-guessing the
    /// dictionary.
    ///
    /// nil for a one-script candidate (the §34 literal, an out-of-vocabulary
    /// name) and under a display with no Hanji on screen (羅馬字): the
    /// romanization again would make Space a slower Return, and Hanji the
    /// user never saw would be worse.
    static func alternateText(
        for candidate: ContinuousCandidate,
        settings: EngineSettings,
    ) -> String? {
        guard let hanji = candidate.presentableHanji,
              settings.candidateDisplayMode.showsHanji
        else { return nil }
        return settings.isTranslateSwapped ? candidate.roman : hanji
    }
}
