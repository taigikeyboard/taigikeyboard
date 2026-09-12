// One recordable key combination, and the keys a binding may never claim.

import AppKit
import Carbon.HIToolbox

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
        /// A letter, a digit, the hyphen or `;` with no ⌘/⌃/⌥ held — every
        /// key one of the two tone schemes types with or picks a candidate
        /// with (`isTypingKey`). Refused whichever scheme is live, so a chord
        /// recorded under one cannot go inert when the user switches to the
        /// other.
        case typesRomanization
        /// Backspace, Escape, the arrows or the paging keys, which the input
        /// method reserves whatever modifiers are held.
        case reservedKey
        /// An event carrying no character to bind.
        case noKey
        /// A chord the system already answers to. Global-tier only: a Carbon
        /// hotkey never gets a chord the window server has taken first, so
        /// recording one would leave a row that reads as bound and does
        /// nothing (`KeyboardShortcuts.Shortcut.isTakenBySystem`).
        case takenBySystem
        /// A key press the Carbon registry cannot name. Global-tier only: a
        /// global row stores a key CODE, and a press that yields none has
        /// nothing to store.
        case notAGlobalKey
        /// A bare ⌘ combination. Global-tier only: ⌘ plus a key is the shape a
        /// Mac application puts its own menu commands on, and a global hotkey
        /// takes that key from whatever the user is typing into for as long as
        /// this input source is selected — ⌘, would cost them their app's own
        /// settings command. Composing rows never see this: they are read
        /// inside a composition, not registered process-wide.
        case belongsToHost
    }

    /// The chord `key` and `modifiers` name, or why it cannot be one.
    static func make(key rawKey: String?, modifiers rawModifiers: NSEvent.ModifierFlags)
        -> Result<ComposingKeyChord, Rejection>
    {
        guard let rawKey, !rawKey.isEmpty else { return .failure(.noKey) }
        let key = normalized(rawKey)
        guard !neverBindable.contains(key) else { return .failure(.reservedKey) }

        let modifiers = chordingModifiers(of: rawModifiers)
        // Shift alone does not make a chord out of a typing key: ⇧A is still
        // the letter A, and binding it would cost the user their capitals.
        let hasChordingModifier = !modifiers.isDisjoint(with: [.command, .control, .option])
        if !hasChordingModifier, let first = key.first, isTypingKey(first) {
            return .failure(.typesRomanization)
        }
        return .success(ComposingKeyChord(key: key, modifiers: modifiers))
    }

    /// The chord this event would record, or why it cannot be recorded.
    ///
    /// A shifted number-row key is refused as the digit it is: Shift alone
    /// does not make a chord out of a typing key (`make(key:modifiers:)`),
    /// and `⇧3` is the `3` key even though a US layout types `#` for it. Read
    /// off the key code, because that is the one thing Shift does not
    /// rewrite — and it is what keeps this path and the Carbon bridge
    /// (`ShortcutConflicts.composingChord(occupiedBy:)`, which is handed the
    /// unmodified `3`) refusing the same press. Every other shifted key
    /// records as the character it types, which is what its stored chords
    /// already hold.
    static func make(_ key: KeyEventSnapshot) -> Result<ComposingKeyChord, Rejection> {
        if chordingModifiers(of: key.modifiers) == .shift, let keyCode = key.keyCode,
           numberRowKeyCodes.contains(keyCode) || keyCode == semicolonKeyCode
        {
            return .failure(.typesRomanization)
        }
        return make(key: key.charactersIgnoringModifiers ?? key.characters, modifiers: key.modifiers)
    }

    /// The `;` key, refused under Shift for the same reason as the number
    /// row: it is the ninth slot key, and `⇧;` aims the 漢羅 commit at it
    /// (`CandidateSlotKeySet.shiftedSlot(for:)`) even though a US layout
    /// types `:` for it.
    static let semicolonKeyCode = UInt16(kVK_ANSI_Semicolon)

    /// The `1`…`9` keys of the number row, in digit order. Carbon's ANSI
    /// codes are positions on the keyboard and hold on ISO and JIS boards
    /// too; note that `6` and `9` sit out of numeric order.
    static let numberRowKeyCodes: [UInt16] = [
        kVK_ANSI_1, kVK_ANSI_2, kVK_ANSI_3, kVK_ANSI_4, kVK_ANSI_5,
        kVK_ANSI_6, kVK_ANSI_7, kVK_ANSI_8, kVK_ANSI_9,
    ].map(UInt16.init)

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
    /// one key. Two keys are folded onto the character they mean, because a
    /// user binding one means both: the keypad's Enter onto Return, since a
    /// keyboard has two of them and leaving them apart would make the keypad
    /// quietly stop committing, and the back tab AppKit reports for ⇧⇥ onto
    /// Tab, since ⇧⇥ is Tab with Shift held to everyone but AppKit. Folding the
    /// back tab here rather than at each site that has to know about it is what
    /// keeps the display table, the defaults and the recorder free of it.
    private static func normalized(_ key: String) -> String {
        switch key {
        case "\u{3}": "\r" // Keypad Enter
        case "\u{19}": "\t" // Back tab, which is what ⇧⇥ reports
        default: key.lowercased()
        }
    }

    /// The keys a composition is typed or picked with, under either tone
    /// scheme: all 26 ASCII letters, the digits, the hyphen and `;`.
    ///
    /// All 26 rather than the eighteen a TL or POJ syllable is spelled with,
    /// because the other eight are not free either: under Telex `v y d w x q
    /// z f` type the tones, and under Standard those eight and `;` are the
    /// candidate slots (`CandidateSlotKeySet.bareKeyRow`). One rule for both
    /// schemes, so a chord recorded under one cannot go inert when the user
    /// switches — which is also what lets `ComposingKeyBindings` skip any
    /// pass against the slot tier. Asked of the normalized key, so the case
    /// fold is `normalized`'s — the same way the reserved-key check reads it.
    private static func isTypingKey(_ character: Character) -> Bool {
        guard character.isASCII else { return false }
        return character.isLetter
            || ComposingKeyIntent.isToneDigit(character)
            || character == "-"
            || character == ";"
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
        if modifiers.contains(.command) {
            letters += "d"
        }
        if modifiers.contains(.control) {
            letters += "c"
        }
        if modifiers.contains(.option) {
            letters += "o"
        }
        if modifiers.contains(.shift) {
            letters += "s"
        }
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
