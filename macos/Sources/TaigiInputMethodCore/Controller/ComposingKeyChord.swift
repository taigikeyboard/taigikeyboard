// One recordable key combination, and the keys a binding may never claim.

import AppKit

/// A key plus its modifiers, as a composing action can be bound to it.
///
/// The key is stored as the character it types with no modifiers held, not as a
/// key code: a layout that puts `[` somewhere else should bind the key that
/// actually types `[`. AppKit names the arrows and the function keys with
/// private-use scalars, which serve the same purpose here.
///
/// Constructing one is where the typing keys are defended. `init?` refuses any
/// chord that would take away a key the user composes with, so a corrupt
/// defaults value or a mistaken migration cannot produce a binding that
/// swallows the letters of a syllable — the classifier never has to re-check.
struct ComposingKeyChord: Hashable, Sendable {
    /// What the key types with no modifiers held, lowercased for ASCII so
    /// `⇧[` and `[` cannot be recorded as two different chords on the same key.
    let key: String
    /// Only the four chording modifiers. Caps Lock, the number pad and the
    /// function flag are dropped: they say how a key was reached, not which
    /// key it is (`ComposingKeyIntent` makes the same choice for the slot
    /// chords).
    let modifiers: NSEvent.ModifierFlags

    /// Keys that build a composition, and the two that get out of one. None may
    /// be bound with no modifier held; several may not be bound at all.
    ///
    /// The arrows and the paging keys are absent from the bindable set for a
    /// different reason: they are the fixed navigation contract
    /// (`ComposingKeyIntent`), so a binding must not be able to shadow them
    /// even WITH a modifier.
    private static let neverBindable: Set<String> = {
        let named = [
            NSLeftArrowFunctionKey, NSRightArrowFunctionKey,
            NSUpArrowFunctionKey, NSDownArrowFunctionKey,
            NSPageUpFunctionKey, NSPageDownFunctionKey,
        ]
        // `compactMap` rather than a `?? " "` fallback: a scalar that failed to
        // build would otherwise land in this set as a SPACE, reserving the key
        // the next-candidate default is on and taking the whole app down on the
        // `preconditionFailure` in `ComposingAction.defaultChord`. These
        // constants cannot fail, and this is what happens if that changes.
        let arrows = named.compactMap(UnicodeScalar.init).map(String.init)
        return Set(arrows).union([
            "\u{8}", // Backspace
            "\u{7F}", // Delete
            "\u{1B}", // Escape
        ])
    }()

    /// Why a key could not be recorded, so the recorder can say so rather than
    /// silently doing nothing.
    enum Rejection: Error, Equatable, Sendable {
        /// A letter, a digit or the hyphen with no modifier held — the
        /// characters a TL or POJ syllable is spelled with, tone marker
        /// included.
        case typesRomanization
        /// Backspace, Escape, the arrows or the paging keys, which the input
        /// method reserves whatever modifiers are held.
        case reservedKey
        /// An event carrying no character to bind.
        case noKey
        /// One of the nine candidate-slot chords. Refused because that tier is
        /// classified before bound actions, so the binding would be recorded
        /// and then never fire (`ComposingKeyIntent.intent`).
        case candidateSlotChord
    }

    /// Whether this chord is one of the nine `1`…`9` slot chords under
    /// `slotModifier`.
    ///
    /// Asked by the recorder rather than by `make`, because the answer changes
    /// with a setting: ⌥3 is bindable while Control holds the slots, and stops
    /// being so the moment the user switches the slot modifier to Option.
    func isCandidateSlotChord(under slotModifier: CandidateSlotModifier) -> Bool {
        // The classifier's own slot rule rather than a second copy of it: the
        // refusal exists because that tier is read first, so the two have to
        // name the same chords.
        modifiers == slotModifier.flag && ComposingKeyIntent.directSelectionSlot(key) != nil
    }

    /// The chord `key` and `modifiers` name, or why it cannot be one.
    static func make(key rawKey: String?, modifiers rawModifiers: NSEvent.ModifierFlags)
        -> Result<ComposingKeyChord, Rejection>
    {
        guard let rawKey, let first = rawKey.first else { return .failure(.noKey) }
        let key = normalized(rawKey)
        guard !neverBindable.contains(key) else { return .failure(.reservedKey) }

        let modifiers = chordingModifiers(of: rawModifiers)
        // Shift alone does not make a chord out of a typing key: ⇧A is still
        // the letter A, and binding it would cost the user their capitals.
        let hasChordingModifier = !modifiers.isDisjoint(with: [.command, .control, .option])
        if !hasChordingModifier, isRomanizationKey(first) {
            return .failure(.typesRomanization)
        }
        return .success(ComposingKeyChord(key: key, modifiers: modifiers))
    }

    /// The chord this event would record, or why it cannot be recorded.
    static func make(_ key: KeyEventSnapshot) -> Result<ComposingKeyChord, Rejection> {
        make(key: key.charactersIgnoringModifiers ?? key.characters, modifiers: key.modifiers)
    }

    /// Whether `event` is this chord. Compared on the unmodified characters for
    /// the same reason they are stored: Control rewrites the digits it is held
    /// with, and Option rewrites most of the keyboard.
    func matches(_ event: KeyEventSnapshot) -> Bool {
        guard let characters = event.charactersIgnoringModifiers ?? event.characters else {
            return false
        }
        guard Self.normalized(characters) == key else { return false }
        return Self.chordingModifiers(of: event.modifiers) == modifiers
    }

    /// The four chording modifiers a chord is made of, and nothing else.
    ///
    /// One function because recording and matching have to agree: a chord
    /// stored with a flag that matching then dropped would be a row that never
    /// fires.
    private static func chordingModifiers(of flags: NSEvent.ModifierFlags) -> NSEvent.ModifierFlags {
        flags.intersection([.command, .control, .option, .shift])
    }

    /// The form a key is stored and compared in.
    ///
    /// ASCII is lowercased so `⇧[` and `[` cannot be recorded as two chords on
    /// one key. The keypad's Enter is folded into Return because a keyboard has
    /// two of them and a user binding one means both — leaving them apart would
    /// make the keypad quietly stop committing.
    private static func normalized(_ key: String) -> String {
        key == "\u{3}" ? "\r" : key.lowercased()
    }

    /// The letters and the hyphen a syllable is spelled with, plus the digits
    /// that carry its tone (`tai5`).
    ///
    /// Asked of the classifier rather than spelled out again: a chord may not
    /// take a key that would otherwise reach the composition as input, so the
    /// refusal has to be the same rule the input path uses.
    private static func isRomanizationKey(_ character: Character) -> Bool {
        ComposingKeyIntent.isRomanizationCharacter(character)
            || ComposingKeyIntent.isToneDigit(character)
    }
}

extension ComposingKeyChord: RawRepresentable {
    /// `"<modifiers>|<scalars>"` — modifier letters in a fixed order, then the
    /// key's Unicode scalars in hex.
    ///
    /// Hex scalars rather than the character itself because most bound keys are
    /// control characters or private-use scalars: a defaults file holding a raw
    /// `\r` is one careless editor away from being unreadable, and `defaults
    /// read` would print it as a line break.
    var rawValue: String {
        var letters = ""
        if modifiers.contains(.command) { letters += "d" }
        if modifiers.contains(.control) { letters += "c" }
        if modifiers.contains(.option) { letters += "o" }
        if modifiers.contains(.shift) { letters += "s" }
        let scalars = key.unicodeScalars
            .map { String(format: "%04X", $0.value) }
            .joined(separator: ",")
        return "\(letters)|\(scalars)"
    }

    init?(rawValue: String) {
        let halves = rawValue.split(separator: "|", maxSplits: 1, omittingEmptySubsequences: false)
        guard halves.count == 2 else { return nil }

        var modifiers: NSEvent.ModifierFlags = []
        for letter in halves[0] {
            switch letter {
            case "d": modifiers.insert(.command)
            case "c": modifiers.insert(.control)
            case "o": modifiers.insert(.option)
            case "s": modifiers.insert(.shift)
            default: return nil
            }
        }

        var key = ""
        for field in halves[1].split(separator: ",") {
            guard let value = UInt32(field, radix: 16),
                  let scalar = UnicodeScalar(value) else { return nil }
            key.unicodeScalars.append(scalar)
        }

        // Back through the same gate the recorder goes through, so a
        // hand-edited or downgraded defaults value cannot install a binding the
        // recorder would have refused.
        guard case let .success(chord) = Self.make(key: key, modifiers: modifiers) else {
            return nil
        }
        self = chord
    }
}
