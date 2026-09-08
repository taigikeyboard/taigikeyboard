// Which key does which composing action — the part of the key contract the user chooses.

import AppKit

/// Which keys pick a candidate out of the nine slots.
///
/// Every case is a whole key SET, not one key: nine slots need nine names, and
/// what varies between the cases is where those names come from. One set is
/// live at a time, and it is the ONLY way to pick (USER 2026-08-28). The set is
/// DERIVED from the tone scheme, not chosen on its own
/// (`ToneInputScheme.slotKeySet`): the letters and the digits are the same
/// keys under both schemes, with the two jobs swapped, so whichever keys type
/// the tones leaves the others free to pick. A bare digit is the TL/POJ tone
/// marker (`tai5`) only under `standard` — which is why the digits can sit on
/// the slots under Telex and not there. The ⇧ / ⌃ / ⌥ digit sets went with
/// the picker that chose them (USER 2026-09-08).
enum CandidateSlotKeySet: CaseIterable, Sendable {
    /// Nine bare keys, one per slot — `q w d f z x v y ;`. The eight letters
    /// are every letter no TL or POJ syllable spells, so a syllable can still
    /// be typed with the bar up; `;` is the ninth because neither
    /// romanization writes it and, unlike `,` or `.`, nobody types it
    /// straight after a word to end a composition. Rime's Taigi schema offers
    /// a bare row the same way, `;` included
    /// (`references/rime-phah-taibun/schema/phah_taibun.schema.yaml:136`
    /// `alternative_select_keys: "asdfghjkl;"`). The set under `standard`
    /// (USER 2026-08-28; nine rather than six so every slot of a nine-row
    /// page has a bare key).
    case bareKeys
    /// Bare `1`…`9`, the set under `telex`, where the letters above type the
    /// tones and a digit no longer can — so the digit is free to pick the way
    /// the system Zhuyin input method's is.
    case digits

    /// The slot `key` picks under this set, or nil when it picks none — the
    /// classifier's question.
    func slot(for key: KeyEventSnapshot) -> Int? {
        slot(
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
    /// cannot make a slot key miss. Both sets are bare keys, so any chording
    /// modifier makes the key miss: ⇧Q is the capital the composition takes
    /// as text, and ⌃3 is the host's. The one rule the classifier and the
    /// window's labels both read, so a key drawn beside a candidate is the
    /// key that picks it.
    func slot(forKey key: String?, heldWith modifiers: NSEvent.ModifierFlags) -> Int? {
        guard modifiers.isEmpty, let key else { return nil }
        switch self {
        case .bareKeys: return Self.bareKeyRow.firstIndex(of: key.lowercased())
        case .digits: return Self.digitSlot(key)
        }
    }

    /// The slot a digit `1`…`9` names, counting from zero. `0` names none: the
    /// bar holds nine candidates because nine is what the digits can name
    /// without one of them meaning "the tenth".
    private static func digitSlot(_ key: String) -> Int? {
        guard let character = key.first,
              character.isASCII,
              let digit = character.wholeNumberValue,
              (1 ... 9).contains(digit)
        else { return nil }
        return digit - 1
    }

    /// The key this set gives the candidate in `slot`, for the nine slots a
    /// page holds (`CandidateIndexLabel`, which owns which key is drawn).
    func label(forSlot slot: Int) -> String {
        switch self {
        case .bareKeys: Self.bareKeyRow[slot]
        case .digits: String(slot + 1)
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
    /// Which keys type a tone. Carried here because it is the other half of
    /// the same contract the chords are: the classifier reads both off one
    /// value, and the slot keys follow from it.
    let toneScheme: ToneInputScheme
    /// Whether a candidate window exists to act on. Carried here for the
    /// same reason as `toneScheme`: with the window off, the keys that
    /// would confirm or page a candidate end the composition as typed
    /// instead, and the classifier decides that off one value.
    let isCandidateWindowEnabled: Bool

    /// The keys that pick a candidate — derived, never stored
    /// (`ToneInputScheme.slotKeySet`).
    var slotKeySet: CandidateSlotKeySet { toneScheme.slotKeySet }

    /// What a fresh install types with.
    static let `default` = ComposingKeyBindings()

    init(
        chords: [ComposingAction: ComposingKeyChord?] = [:],
        toneScheme: ToneInputScheme = .standard,
        isCandidateWindowEnabled: Bool = true,
    ) {
        var resolved: [ComposingAction: ComposingKeyChord] = [:]
        for action in ComposingAction.allCases {
            // A stored nil is "the user cleared this row"; an absent key is
            // "never touched", which is what the default is for. The double
            // optional is what tells the two apart — the outer one answers
            // whether the action was mentioned at all.
            resolved[action] = chords[action] ?? action.defaultChord
        }
        // No pass against the slot tier: every chord came through
        // `ComposingKeyChord.make`, which refuses every bare letter, digit and
        // `;` — the keys either scheme's slots use — so no chord can be a slot
        // key under any scheme.
        Self.removeDuplicates(in: &resolved)
        Self.restoreUnbound(in: &resolved)
        self.chords = resolved
        self.toneScheme = toneScheme
        self.isCandidateWindowEnabled = isCandidateWindowEnabled
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
