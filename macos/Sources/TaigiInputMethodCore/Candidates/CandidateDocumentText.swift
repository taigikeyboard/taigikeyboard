// Renders one candidate as the string that goes into the user's document.

import Foundation

/// Turns a candidate into document text according to the output settings.
///
/// Separate from the candidate's own `displayText`, which is the engine's
/// canonical form (`hanji ?? roman`; the §34 literal may carry the identity of
/// a dictionary row it absorbed — engine `adopt_collapsed_dict_identity`) and
/// pairs with `canonicalTl` to identify
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
        resolved(for: candidate, settings: settings).text
    }

    /// The document string AND whether writing it puts romanization in the
    /// document — resolved together, by the one branch that picks the string.
    ///
    /// Auto-space is a property of ROMANIZATION (`guá beh khì` needs the gaps,
    /// 我欲去 does not), so its gate has to answer for the string this commit
    /// actually writes. Deriving that verdict from the output mode instead is
    /// only ever an approximation, and it is wrong for a candidate with no
    /// Hanji: the 字面羅馬字 candidate (§34), an out-of-vocabulary name, a
    /// romanization-only custom entry all write their romanization whatever
    /// the mode leads with. Asking the branch that built the string is how the
    /// two cannot disagree.
    static func resolved(
        for candidate: ContinuousCandidate,
        settings: EngineSettings,
    ) -> ResolvedCommit {
        // No presentable Hanji: the romanization alone — rendering "guá ()"
        // for it would be a visible defect — and romanization is what the
        // document gets whichever script the mode leads with.
        guard let hanji = candidate.presentableHanji else {
            return ResolvedCommit(text: candidate.roman, wroteRomanization: true)
        }

        if settings.isOutputBothScripts {
            // 括號標註 writes the pair, so the romanization IS in the document
            // whichever half leads.
            let text = settings.isTranslateSwapped
                ? "\(hanji) (\(candidate.roman))"
                : "\(candidate.roman) (\(hanji))"
            return ResolvedCommit(text: text, wroteRomanization: true)
        }
        return settings.isTranslateSwapped
            ? ResolvedCommit(text: hanji, wroteRomanization: false)
            : ResolvedCommit(text: candidate.roman, wroteRomanization: true)
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
    /// The alternate is one script by itself, never the bracketed pair, so
    /// the verdict it carries is simply which script that is.
    static func resolvedAlternate(
        for candidate: ContinuousCandidate,
        settings: EngineSettings,
    ) -> ResolvedCommit? {
        guard let hanji = candidate.presentableHanji,
              settings.candidateDisplayMode.showsHanji
        else { return nil }
        return settings.isTranslateSwapped
            ? ResolvedCommit(text: candidate.roman, wroteRomanization: true)
            : ResolvedCommit(text: hanji, wroteRomanization: false)
    }
}

/// What one commit writes into the document, and whether that string carries
/// romanization — the single input the auto-space gate reads
/// (`AutoSpacePolicy.isGateActive`).
struct ResolvedCommit: Equatable, Sendable {
    let text: String
    let wroteRomanization: Bool
}
