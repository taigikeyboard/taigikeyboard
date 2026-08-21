// Which key does which composing action — the part of the key contract the user chooses.

import AppKit

/// Which modifier the `1`…`9` candidate-slot chords are held with.
///
/// A choice of modifier rather than of characters: the digits themselves are
/// not negotiable, because a bare digit is a TL/POJ tone marker (`tai5`) and
/// binding one would make toned syllables untypable. This is also why the
/// system Zhuyin input method's bare-digit selection cannot be matched here.
enum CandidateSlotModifier: String, CaseIterable, Sendable {
    case control
    case option

    var flag: NSEvent.ModifierFlags {
        switch self {
        case .control: .control
        case .option: .option
        }
    }

    /// The nine chords this modifier stands for, as one string.
    ///
    /// Written into a menu row's title rather than set as its `keyEquivalent`:
    /// a menu item prints one key, and this setting is nine of them.
    var menuRange: String {
        switch self {
        case .control: "⌃1 – ⌃9"
        case .option: "⌥1 – ⌥9"
        }
    }
}

/// The user's composing key contract, resolved and ready to classify against.
///
/// A value passed into `ComposingKeyIntent.intent(for:...)` rather than read
/// from `UserDefaults` inside it: the classification is the whole key contract
/// of the input method, and it stays a pure function of its inputs so every
/// binding combination can be pinned by a test.
///
/// "Resolved" means three things have already happened, so the classifier can
/// trust the value it is handed:
///
/// - Every chord came through `ComposingKeyChord.make`, so none of them is a
///   key the user composes with.
/// - No chord is on two actions. A later action recording a chord takes it
///   from the earlier one, the way the System Settings keyboard pane behaves.
/// - `ComposingAction.alwaysBound` is honoured: an action in that set that
///   would otherwise be unbound is restored to its default chord, because
///   between them those two are the only way to end a composition into the
///   document.
struct ComposingKeyBindings: Sendable, Equatable {
    private(set) var chords: [ComposingAction: ComposingKeyChord]
    var slotModifier: CandidateSlotModifier

    /// What a fresh install types with.
    static let `default` = ComposingKeyBindings()

    init(
        chords: [ComposingAction: ComposingKeyChord?] = [:],
        slotModifier: CandidateSlotModifier = .control,
    ) {
        var resolved: [ComposingAction: ComposingKeyChord] = [:]
        for action in ComposingAction.allCases {
            // A stored nil is "the user cleared this row"; an absent key is
            // "never touched", which is what the default is for. The double
            // optional is what tells the two apart — the outer one answers
            // whether the action was mentioned at all.
            resolved[action] = chords[action] ?? action.defaultChord
        }
        Self.removeShadowedByCandidateSlots(in: &resolved, under: slotModifier)
        Self.removeDuplicates(in: &resolved)
        Self.restoreUnbound(in: &resolved)
        self.chords = resolved
        self.slotModifier = slotModifier
    }

    /// The chord on `action`, or nil when the row is empty.
    func chord(for action: ComposingAction) -> ComposingKeyChord? { chords[action] }

    /// The action `event` is bound to, if any.
    ///
    /// Linear over eight actions rather than a dictionary keyed by chord: the
    /// roster is small enough that the lookup cost is noise next to the
    /// keystroke around it, and a chord-keyed dictionary would need the same
    /// duplicate handling a second time to build.
    func action(for event: KeyEventSnapshot) -> ComposingAction? {
        ComposingAction.allCases.first { chords[$0]?.matches(event) == true }
    }

    /// Which other actions currently hold `chord`.
    ///
    /// The pure half of recording: the recorder asks before it writes, so the
    /// row that loses a chord empties in front of the user rather than being
    /// discovered later. Mirrors `ShortcutConflicts` for the global chords.
    func actionsHolding(_ chord: ComposingKeyChord, excluding changed: ComposingAction)
        -> [ComposingAction]
    {
        ComposingAction.allCases.filter { $0 != changed && chords[$0] == chord }
    }

    /// Drops a chord the candidate-slot tier would swallow before any binding
    /// is looked at.
    ///
    /// Needed here as well as in the recorder because the slot modifier can be
    /// changed afterwards: a ⌥Return recorded while Control held the slots
    /// keeps working, but a ⌥3 does not, and a row that silently does nothing
    /// is worse than an empty one.
    private static func removeShadowedByCandidateSlots(
        in resolved: inout [ComposingAction: ComposingKeyChord],
        under slotModifier: CandidateSlotModifier,
    ) {
        for (action, chord) in resolved where chord.isCandidateSlotChord(under: slotModifier) {
            resolved[action] = nil
        }
    }

    /// Drops a chord from every action but the last one holding it.
    ///
    /// `allCases` order is the tiebreak, which makes it deterministic rather
    /// than fair — the pane resolves conflicts as they are made, off
    /// `actionsHolding(_:excluding:)` above, so this only has to handle a
    /// defaults domain edited behind the app's back.
    private static func removeDuplicates(in resolved: inout [ComposingAction: ComposingKeyChord]) {
        var seen: [ComposingKeyChord: ComposingAction] = [:]
        for action in ComposingAction.allCases {
            guard let chord = resolved[action] else { continue }
            if let earlier = seen[chord] {
                resolved[earlier] = nil
            }
            seen[chord] = action
        }
    }

    /// Keeps every always-bound action reachable.
    ///
    /// Their default chords are a pool the always-bound actions share, and no
    /// other action may hold one. An empty always-bound row then takes whichever
    /// chord in that pool nobody else in the pool has — which is what makes this
    /// terminate: filling one row can never empty another, so there is no cycle
    /// to iterate out of.
    ///
    /// Sharing the pool rather than pinning each action to its own default is
    /// what lets the two SWAP: a user who wants Return on the literal and
    /// ⇧Return on the candidate keeps that, because both rows are full and
    /// nothing needs restoring.
    ///
    /// The cost is that Return and ⇧Return cannot be moved onto anything else.
    /// That is the trade: the two keys that end a composition into the document
    /// stay where a user can find them.
    private static func restoreUnbound(in resolved: inout [ComposingAction: ComposingKeyChord]) {
        let alwaysBound = ComposingAction.allCases.filter(ComposingAction.alwaysBound.contains)
        let pool = alwaysBound.compactMap(\.defaultChord)

        for (action, chord) in resolved
            where pool.contains(chord) && !ComposingAction.alwaysBound.contains(action)
        {
            resolved[action] = nil
        }

        var taken = Set(alwaysBound.compactMap { resolved[$0] })
        for action in alwaysBound where resolved[action] == nil {
            guard let free = pool.first(where: { !taken.contains($0) }) else { continue }
            resolved[action] = free
            taken.insert(free)
        }
    }
}
