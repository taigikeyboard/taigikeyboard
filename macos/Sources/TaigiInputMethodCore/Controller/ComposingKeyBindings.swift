// Which key does which composing action — the part of the key contract the user chooses.

import AppKit

/// What Return does while a composition is running.
enum ReturnKeyBehavior: String, CaseIterable, Sendable {
    /// Commits the literal the marked region shows. The shipped behaviour, and
    /// the one iOS and Android's Enter share — the 漢羅 flow depends on being
    /// able to commit exactly what was typed (`behavioral-invariants.md §34`).
    case commitLiteral
    /// Commits the highlighted candidate, the way the system Zhuyin input
    /// method's Return does. ⇧Return still commits the literal, so the typed
    /// romanization is never unreachable.
    case confirmHighlighted
}

/// What Space does while the candidate bar is up.
enum SpaceKeyBehavior: String, CaseIterable, Sendable {
    /// Commits the highlighted candidate. The shipped behaviour.
    case confirmHighlighted
    /// Walks the highlight one candidate forward, the way Space cycles
    /// candidates in the system Zhuyin input method. The slot chords then
    /// commit — as does Return, for a user who also binds it to the
    /// highlighted candidate.
    case nextCandidate
}

/// Whether `[` and `]` page the candidate bar.
enum BracketPagingBehavior: String, CaseIterable, Sendable {
    /// `[` pages back and `]` pages forward while the bar is up, matching the
    /// system Zhuyin input method's candidate window. With no bar both stay
    /// document text, and ⇧ makes them `{` and `}`, which always are.
    case enabled
    /// Both are always document text.
    case disabled
}

/// Whether Tab and ⇧Tab walk the candidate list.
enum TabCycleBehavior: String, CaseIterable, Sendable {
    /// Tab keeps reaching the host, which is what it does today.
    case disabled
    /// Tab moves to the next candidate and ⇧Tab to the previous one while the
    /// bar is up.
    case enabled
}

/// Which modifier the `1`…`9` candidate-slot chords are held with.
///
/// A choice of modifier rather than of characters: the digits themselves are
/// not negotiable, because a bare digit is a TL/POJ tone marker (`tai5`) and
/// binding one would make toned syllables untypable.
enum CandidateSlotModifier: String, CaseIterable, Sendable {
    case control
    case option

    var flag: NSEvent.ModifierFlags {
        switch self {
        case .control: .control
        case .option: .option
        }
    }
}

/// The user's choices for the composing key contract, as one value.
///
/// A value passed into `ComposingKeyIntent.intent(for:...)` rather than read
/// from `UserDefaults` inside it: the classification is the whole key contract
/// of the input method, and it stays a pure function of its inputs so every
/// binding combination can be pinned by a test.
///
/// Each field owns a disjoint set of keys — Return and ⇧Return, Space, the two
/// bracket characters, the Tab pair, and a modifier chorded with the digits.
/// That is why no conflict resolution exists for this family: two settings
/// cannot claim the same key, so there is no state to repair. (The three global
/// shortcuts are recorded freely and do need `ShortcutConflicts`.)
///
/// Keys deliberately left unbindable: letters and `-` build romanization,
/// bare digits are tone markers, Backspace deletes, Escape cancels, and the
/// arrows navigate — binding any of them would take away a key the user needs
/// to type or to get out with.
struct ComposingKeyBindings: Sendable, Equatable {
    var returnKey: ReturnKeyBehavior = .commitLiteral
    var spaceKey: SpaceKeyBehavior = .confirmHighlighted
    var bracketPaging: BracketPagingBehavior = .enabled
    var tabCycle: TabCycleBehavior = .disabled
    var slotModifier: CandidateSlotModifier = .control

    /// What a fresh install types with.
    static let `default` = ComposingKeyBindings()
}
