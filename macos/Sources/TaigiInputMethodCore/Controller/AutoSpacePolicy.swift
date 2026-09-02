// When a committed word earns the auto-space trailing space, as pure decisions.

import Foundation

/// The auto-space decisions, separated from the controller so they can be
/// tested without a client.
///
/// The rules mirror iOS (`ActionHandler+Suggestions.swift` /
/// `ActionHandler+KeyActions.swift`, behavioral-invariants.md §23): a committed
/// word gets a trailing space; a trailing hyphen — a syllable the user is
/// about to continue — suppresses it; a commit that put HANJI in the document
/// turns the feature off unless 括號標註 keeps the romanization in the output.
/// macOS retired the 括號標註 toggle (`RetiredSettingsCleanup` pins it false),
/// so that branch of the gate never fires here today — it is kept because it IS
/// the iOS formula, and a second spelling of the gate is how the platforms
/// drift.
enum AutoSpacePolicy {
    /// True when this commit earns a trailing space at all — the same gate
    /// every insertion site and the punctuation swap read.
    ///
    /// `wroteRomanization` rather than the output mode. Spacing is a property
    /// of romanization — `guá beh khì` needs the gaps, 我欲去 does not — and
    /// until the 漢羅 key existed the mode was an exact proxy for it, because
    /// the mode was the only thing deciding what got written. Space commits the
    /// script the mode does NOT lead with, and a candidate with no Hanji (the
    /// §34 字面羅馬字, an out-of-vocabulary name) writes its romanization under
    /// every mode, so the proxy disagrees with the document in both
    /// directions. The verdict comes from whatever resolved the string:
    /// `CandidateDocumentText.resolved` for a candidate,
    /// `rawPreeditWritesRomanization(inputMode:)` for the preedit itself.
    static func isGateActive(isAutoSpaceEnabled: Bool, wroteRomanization: Bool) -> Bool {
        isAutoSpaceEnabled && wroteRomanization
    }

    /// Whether committing the preedit AS TYPED writes romanization — the ⇧Enter
    /// literal commit and the mid-composition punctuation commit, neither of
    /// which goes through a candidate.
    ///
    /// A `switch` over a two-case enum rather than `true`, so that adding a
    /// non-romanized layout (TPS composes Bopomofo, which takes no spacing —
    /// see the iOS `keyboardLayoutType` gate) fails to compile here instead of
    /// silently spacing 注音.
    static func rawPreeditWritesRomanization(inputMode: InputMode) -> Bool {
        switch inputMode {
        case .tl, .poj: true
        }
    }

    /// Whether the text a commit just wrote should be followed by a space.
    ///
    /// Judged on the actual document string, never on a display mirror: the
    /// output mode can make the two differ, and the hyphen rule is about what
    /// is in front of the caret.
    static func shouldAppendSpace(afterCommitting documentText: String) -> Bool {
        guard !documentText.isEmpty else { return false }
        return !documentText.hasSuffix("-")
    }

    /// What `commitThenInsert` should hand the engine, so one keystroke stays
    /// one document mutation.
    struct AugmentedInsert: Equatable {
        let text: String
        /// True when the insertion leaves the auto space immediately before the
        /// caret, so the next attaching-punctuation key may swap with it.
        let leavesTrailingAutoSpace: Bool
    }

    /// Rewrites the external text a mid-composition punctuation key inserts.
    ///
    /// On iOS the same keystroke is two steps — the commit appends the auto
    /// space, then `insertNonComposingCharacter` places the character around it
    /// (`ActionHandler+KeyActions.swift:132-147`). macOS commits preedit and
    /// character in ONE engine mutation, so the space is folded into that
    /// mutation instead, to the same document result: attaching punctuation
    /// lands before the space (`guá? `), everything else after it (`guá (`).
    ///
    /// `composingDisplayText` is the marked region about to be committed — for
    /// this raw-commit path the engine writes exactly what the region shows, so
    /// its trailing hyphen is the committed text's trailing hyphen.
    ///
    /// A typed space is left alone: the engine already appends it, and adding
    /// another would double it. It still arms the swap — iOS swaps with any
    /// space in front of the caret while the gate is active, user-typed
    /// included.
    static func augmentInsert(
        _ text: String,
        afterComposition composingDisplayText: String,
        isGateActive: Bool,
    ) -> AugmentedInsert {
        guard isGateActive, !text.isEmpty else {
            return AugmentedInsert(text: text, leavesTrailingAutoSpace: false)
        }
        guard !text.contains(where: \.isWhitespace) else {
            return AugmentedInsert(text: text, leavesTrailingAutoSpace: text == " ")
        }
        // The same rule as the post-commit sites, through the one spelling:
        // on this raw-commit path the display text IS the committed text.
        guard shouldAppendSpace(afterCommitting: composingDisplayText) else {
            return AugmentedInsert(text: text, leavesTrailingAutoSpace: false)
        }
        if AutoSpacePunctuation.isAttaching(text) {
            return AugmentedInsert(text: text + " ", leavesTrailingAutoSpace: true)
        }
        return AugmentedInsert(text: " " + text, leavesTrailingAutoSpace: false)
    }
}
