// When a committed word earns the auto-space trailing space, as pure decisions.

import Foundation

/// The auto-space decisions, separated from the controller so they can be
/// tested without a client.
///
/// The rules mirror iOS (`ActionHandler+Suggestions.swift` /
/// `ActionHandler+KeyActions.swift`, behavioral-invariants.md §23): a committed
/// word gets a trailing space; a trailing hyphen — a syllable the user is
/// about to continue — suppresses it; the 漢字-swap mode turns the feature off
/// unless 括號標註 keeps the romanization in the output. macOS retired the
/// 括號標註 toggle (`RetiredSettingsCleanup` pins it false), so that branch of
/// the gate never fires here today — it is kept because it IS the iOS formula,
/// and a second spelling of the gate is how the platforms drift.
enum AutoSpacePolicy {
    /// True when the current mode auto-inserts a trailing space at all — the
    /// same gate every insertion site and the punctuation swap read.
    static func isGateActive(
        isAutoSpaceEnabled: Bool,
        isTranslateSwapped: Bool,
        isOutputBothScripts: Bool,
    ) -> Bool {
        guard isAutoSpaceEnabled else { return false }
        return !isTranslateSwapped || isOutputBothScripts
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
