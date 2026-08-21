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
    /// Which script a commit writes.
    ///
    /// A policy the caller names, not a string it supplies: the text is still
    /// produced here, from the same candidate and the same settings snapshot
    /// the engine call uses, so there remains one place that knows how a
    /// candidate becomes document text.
    enum Rendering: Equatable, Sendable {
        /// However the user has chosen to see Taigi written — what every
        /// ordinary commit uses.
        case settings
        /// The Hanji alone, whatever the settings say. What the 直接輸出漢字
        /// action asks for: the user is overriding their own preference for
        /// this one word, not changing it.
        case hanji
        /// The romanization alone, same terms.
        case romanization

        /// Whether `candidate` has the script this rendering asks for.
        ///
        /// Only 直接輸出漢字 can come up empty — a romanization-only candidate
        /// has no Hanji — and the answer is no rather than "fall back to the
        /// romanization": the key names a script, and quietly writing the other
        /// one is worse than doing nothing.
        func canRender(_ candidate: ContinuousCandidate) -> Bool {
            switch self {
            case .settings, .romanization: true
            case .hanji: !(candidate.hanji ?? "").isEmpty
            }
        }
    }

    /// CROSS-PLATFORM INVARIANT — mirrors
    /// ios/Sources/TaigiKeyboard/Actions/ActionHandler+Suggestions.swift:218.
    /// Drift changes what a candidate writes into the document.
    ///
    /// The TPS branch of the iOS original is deliberately absent: macOS ships TL
    /// and POJ only (`docs/architecture/macos-roadmap.md` § Goal), so there is no
    /// layout that could ask for bracketed Bopomofo.
    ///
    /// A forced rendering reads NEITHER output setting: not `isTranslateSwapped`,
    /// and not `isOutputBothScripts` — which no longer has a UI but whose stored
    /// value the engine still reads, so an install carrying a stale `true` must
    /// not get bracketed pairs out of a key that says "Hanji".
    static func text(
        for candidate: ContinuousCandidate,
        settings: EngineSettings,
        rendering: Rendering = .settings,
    ) -> String {
        // A romanization-only candidate is marked by an absent hanji, but a
        // producer that emitted an empty string instead means the same thing —
        // and rendering "guá ()" for it would be a visible defect.
        guard let hanji = candidate.hanji, !hanji.isEmpty else { return candidate.roman }

        switch rendering {
        case .hanji:
            return hanji
        case .romanization:
            return candidate.roman
        case .settings:
            break
        }

        if settings.isOutputBothScripts {
            return settings.isTranslateSwapped
                ? "\(hanji) (\(candidate.roman))"
                : "\(candidate.roman) (\(hanji))"
        }
        return settings.isTranslateSwapped ? hanji : candidate.roman
    }
}
