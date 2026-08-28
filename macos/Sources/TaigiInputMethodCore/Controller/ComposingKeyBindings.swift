// Which key does which composing action — the part of the key contract the user chooses.

import AppKit

/// Which keys pick a candidate out of the nine slots — the part of the slot
/// contract the user chooses.
///
/// Every case is a whole key SET, not one key: nine slots need nine names, and
/// what varies between the cases is where those names come from. One set is
/// live at a time — the picker's four ways of selecting (USER 2026-08-28).
/// One thing never varies with the choice and is not here: a bare `1`…`9`
/// picks once no tone can follow the buffer
/// (`ComposingKeyIntent.canTypeToneDigit`). A bare digit is a TL/POJ tone
/// marker first (`tai5`), which is why none of the sets below can put the
/// digits themselves on the slots — and why the system Zhuyin input method's
/// bare-digit selection cannot be matched here.
enum CandidateSlotKeySet: String, CaseIterable, Sendable {
    /// Nine bare keys, one per slot — `q w d f z x v y ;`. The eight letters
    /// are every letter no TL or POJ syllable spells
    /// (`ComposingKeyChord.syllableLetters`), so a syllable can still be typed
    /// with the bar up; `;` is the ninth because neither romanization writes
    /// it and, unlike `,` or `.`, nobody types it straight after a word to end
    /// a composition. Rime's Taigi schema offers a bare row the same way,
    /// `;` included (`references/rime-phah-taibun/schema/phah_taibun.schema.yaml:136`
    /// `alternative_select_keys: "asdfghjkl;"`). The shipped default (USER
    /// 2026-08-28; nine rather than six so every slot of a nine-row page has
    /// a bare key).
    case bareKeys
    /// `⇧1`…`⇧9`. A shifted digit types punctuation, which nobody types while
    /// choosing a candidate — the composition ends first — so the chord costs
    /// nothing while the bar is up and reads as punctuation again the moment
    /// it is down (USER 2026-08-28). Read off the number row's key codes,
    /// since Shift rewrites the characters (`ComposingKeyIntent.shiftedDigitSlot`).
    case shift
    /// `⌃1`…`⌃9`.
    case control
    /// `⌥1`…`⌥9`.
    case option

    /// The slot `key` picks under this set, or nil when it picks none — the
    /// classifier's question, asked of the whole event because the shifted
    /// digits are only knowable from the key code.
    func slot(for key: KeyEventSnapshot) -> Int? {
        guard self != .shift else { return ComposingKeyIntent.shiftedDigitSlot(key) }
        return slot(
            forKey: key.charactersIgnoringModifiers,
            heldWith: key.modifiers.intersection([.command, .control, .option, .shift]),
        )
    }

    /// The keys `bareKeys` puts on slots 0…8, in slot order. Lowercase, as
    /// they are matched and drawn: a bare key types its lowercase form.
    static let bareKeyRow = ["q", "w", "d", "f", "z", "x", "v", "y", ";"]

    /// The slot `key` picks under this set with exactly `modifiers` held, or
    /// nil when it picks none.
    ///
    /// `key` is what the key types with no modifiers held
    /// (`KeyEventSnapshot.charactersIgnoringModifiers`, or a chord's key);
    /// `modifiers` is only the four chording flags, so Caps Lock and the
    /// number pad — which say how a key was reached, not which key it is —
    /// cannot make a slot key miss. The one rule the classifier, the
    /// recorder's refusal and the window's labels all read, so a key drawn
    /// beside a candidate is the key that picks it. For `shift` this answers
    /// for a chord already keyed on the digit; an EVENT goes through
    /// `slot(for:)`, because `⇧3` types `#`.
    func slot(forKey key: String?, heldWith modifiers: NSEvent.ModifierFlags) -> Int? {
        guard let digitModifier else {
            guard modifiers.isEmpty, let key else { return nil }
            return Self.bareKeyRow.firstIndex(of: key.lowercased())
        }
        guard modifiers == digitModifier.flag else { return nil }
        return ComposingKeyIntent.directSelectionSlot(key)
    }

    /// The key this set gives the candidate in `slot`, for the nine slots a
    /// page holds (`CandidateIndexLabel`, which owns which key is drawn).
    func label(forSlot slot: Int) -> String {
        guard let digitModifier else { return Self.bareKeyRow[slot] }
        return digitModifier.symbol + String(slot + 1)
    }

    /// What the shortcut pane's picker offers to choose between — the keys
    /// themselves, in the glyphs the keyboard prints them with, so the row
    /// reads the same in every display language.
    var menuLabel: String {
        guard let digitModifier else { return Self.bareKeyRow.joined(separator: " ") }
        return "\(digitModifier.symbol)1 – \(digitModifier.symbol)9"
    }

    /// The modifier the digit chords are held with, and how it is written on
    /// a key cap. Nil for `bareKeys` — so every rule above asks this once and
    /// reads the answer as "the bare keys" or "the digits".
    private var digitModifier: (flag: NSEvent.ModifierFlags, symbol: String)? {
        switch self {
        case .bareKeys: nil
        case .shift: (.shift, "⇧")
        case .control: (.control, "⌃")
        case .option: (.option, "⌥")
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
    var slotKeySet: CandidateSlotKeySet

    /// What a fresh install types with.
    static let `default` = ComposingKeyBindings()

    init(
        chords: [ComposingAction: ComposingKeyChord?] = [:],
        slotKeySet: CandidateSlotKeySet = .bareKeys,
    ) {
        var resolved: [ComposingAction: ComposingKeyChord] = [:]
        for action in ComposingAction.allCases {
            // A stored nil is "the user cleared this row"; an absent key is
            // "never touched", which is what the default is for. The double
            // optional is what tells the two apart — the outer one answers
            // whether the action was mentioned at all.
            resolved[action] = chords[action] ?? action.defaultChord
        }
        Self.removeShadowedByCandidateSlots(in: &resolved, under: slotKeySet)
        Self.removeDuplicates(in: &resolved)
        Self.restoreUnbound(in: &resolved)
        self.chords = resolved
        self.slotKeySet = slotKeySet
    }

    /// The chord on `action`, or nil when the row is empty.
    func chord(for action: ComposingAction) -> ComposingKeyChord? { chords[action] }

    /// The action `event` is bound to, if any.
    ///
    /// Linear over the action roster rather than a dictionary keyed by chord:
    /// the roster is small enough that the lookup cost is noise next to the
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
    /// `excluding` is the action being recorded, which is not competing with
    /// itself; the cross-registry scan asks without one, because the global
    /// tier is nobody's row here (`ShortcutConflicts`).
    func actionsHolding(_ chord: ComposingKeyChord, excluding changed: ComposingAction? = nil)
        -> [ComposingAction]
    {
        ComposingAction.allCases.filter { $0 != changed && chords[$0] == chord }
    }

    /// Drops a chord the candidate-slot tier would swallow before any binding
    /// is looked at.
    ///
    /// Needed here as well as in the recorder because the slot key set can be
    /// changed afterwards: a ⌥Return recorded while Control held the slots
    /// keeps working, but a ⌥3 does not — nor does a bare `z` once the bare
    /// keys hold them — and a row that silently does nothing is worse than an
    /// empty one. Dropped from the resolved value, not from storage: the row
    /// comes back if the picker moves off the set that shadowed it.
    private static func removeShadowedByCandidateSlots(
        in resolved: inout [ComposingAction: ComposingKeyChord],
        under slotKeySet: CandidateSlotKeySet,
    ) {
        for (action, chord) in resolved where chord.isCandidateSlotChord(under: slotKeySet) {
            resolved[action] = nil
        }
    }

    /// Drops a chord from every action but the last one holding it, with the
    /// rows still on their own default going first.
    ///
    /// Last wins, so the order IS the policy: a row holding a chord the user
    /// chose outranks a row holding only what it shipped with. That matters on
    /// an upgrade, where an action that gains a default can gain one the user
    /// had already put somewhere else — resolving on `allCases` order alone
    /// would let the new default silently empty the row they set.
    ///
    /// Within each half `allCases` order is the tiebreak, which makes it
    /// deterministic rather than fair — the pane resolves conflicts as they are
    /// made, off `actionsHolding(_:excluding:)` above, so that half only has to
    /// handle a defaults domain edited behind the app's back.
    private static func removeDuplicates(in resolved: inout [ComposingAction: ComposingKeyChord]) {
        let onItsDefault = resolved.filter { $0.value == $0.key.defaultChord }.keys
        let order = ComposingAction.allCases.filter(onItsDefault.contains)
            + ComposingAction.allCases.filter { !onItsDefault.contains($0) }

        var seen: [ComposingKeyChord: ComposingAction] = [:]
        for action in order {
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
        let pool = alwaysBound.map(\.defaultChord)

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
