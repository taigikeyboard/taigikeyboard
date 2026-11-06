// A key event as the Swift side reads it, and the key predicates it still shares.

import AppKit

/// The parts of an `NSEvent` a composing decision is made from.
///
/// A value rather than the event itself so the classification can be reasoned
/// about — and tested — without an AppKit event object, and so the decision can
/// be handed between isolation domains: `NSEvent` is a reference type that
/// cannot cross one.
struct KeyEventSnapshot: Sendable {
    let characters: String?
    /// What the same key would have typed with no modifiers held. Carried
    /// because Control rewrites the characters of the digits it is chorded with
    /// — Ctrl+2 through Ctrl+8 arrive as NUL, ESC, FS, GS, RS, US and DEL — so
    /// `characters` would read Ctrl+3 as an Escape and cancel the composition
    /// the chord was meant to pick a candidate from.
    let charactersIgnoringModifiers: String?
    /// The virtual key code (`NSEvent.keyCode`) — the key's position as the
    /// system reports it, before any layout turns it into a character.
    /// Carried for the refusal of a shifted number-row key: `⇧3` types `#`
    /// on a US layout and `charactersIgnoringModifiers` keeps Shift, so the
    /// number row's codes are the one thing that still says which key was
    /// pressed (the core reads them, `key_translation.rs`). Nil for a
    /// snapshot built without an event.
    let keyCode: UInt16?
    let modifiers: NSEvent.ModifierFlags
    /// Whether AppKit has a name for this key (`NSEvent.SpecialKey`). Which
    /// name is not recorded: the keys this input method binds are recognized by
    /// their characters, and the rest only need to be told apart from text.
    let isNamedSpecialKey: Bool
    /// Whether this key-down is the keyboard's auto-repeat of a key still
    /// held (`NSEvent.isARepeat`). Read by the symbol-picker chord, which
    /// toggles: a held chord would otherwise open and close the picker on
    /// every repeat. False for a snapshot built without an event.
    let isRepeat: Bool
    /// `NSEvent.specialKey?.rawValue` — what the desktop core reads a named
    /// key from (`KeyEvent.special_key`). Nil for a key AppKit has no name
    /// for, and for a snapshot built without an event.
    let specialKeyRawValue: UInt32?

    init(
        characters: String?,
        modifiers: NSEvent.ModifierFlags,
        isNamedSpecialKey: Bool,
        charactersIgnoringModifiers: String? = nil,
        keyCode: UInt16? = nil,
        isRepeat: Bool = false,
        specialKeyRawValue: UInt32? = nil,
    ) {
        self.characters = characters
        self.charactersIgnoringModifiers = charactersIgnoringModifiers ?? characters
        self.keyCode = keyCode
        self.modifiers = modifiers
        self.isNamedSpecialKey = isNamedSpecialKey
        self.isRepeat = isRepeat
        self.specialKeyRawValue = specialKeyRawValue
    }

    init(_ event: NSEvent) {
        // Read only off a key event: `NSEvent.keyCode` and `isARepeat` raise
        // on any other type, and the recorder's monitor also sees mouse-ups.
        let isKeyEvent = event.type == .keyDown || event.type == .keyUp
        self.init(
            characters: event.characters,
            modifiers: event.modifierFlags,
            isNamedSpecialKey: event.specialKey != nil,
            charactersIgnoringModifiers: event.charactersIgnoringModifiers,
            keyCode: isKeyEvent ? event.keyCode : nil,
            isRepeat: isKeyEvent && event.isARepeat,
            specialKeyRawValue: event.specialKey.flatMap { UInt32(exactly: $0.rawValue) },
        )
    }
}

/// What the Swift side still reads off a key event: the modifier sets and the
/// text predicates the controller, the core back end and the shortcut recorder
/// share. What a key means to a composition, to the symbol picker and to the
/// Shortcuts pane is desktop-core's (`desktop/crates/taigi-desktop-core/src/keys`).
extension KeyEventSnapshot {
    /// AppKit encodes function and arrow keys as private-use scalars rather
    /// than control characters, so a scalar check alone would let F5 through as
    /// composition input.
    private static let appKitFunctionKeyRange: ClosedRange<UInt32> = 0xF700 ... 0xF8FF

    /// The chords the host owns. Named once because several rules are written
    /// against it — `documentText(inputMode:)` among them — and a list spelled
    /// out at each of them is a list that can drift apart.
    static let hostChords: NSEvent.ModifierFlags = [.command, .control, .option]

    /// The four chording modifiers — what a recorded chord is made of. Caps
    /// Lock, the number pad and the function flag say how a key was reached,
    /// not which key it is.
    static let chordingModifiers: NSEvent.ModifierFlags = hostChords.union(.shift)

    /// The text this key puts into the document, or nil when it is a key the
    /// host acts on. Read off the whole event rather than its characters: `⌘.`
    /// and a typed `.` carry the same character, and one of them is a host
    /// command that inserts nothing — the chording modifiers tell them apart.
    /// The TPS punctuation chord is document text too, and what it types is the
    /// key under the modifier: `⌃,` arrives with `characters` `,` but `⌃[`
    /// arrives as Escape, and the bracket is what the user asked for. The
    /// desktop core's `ComposingKeyIntent::document_text`.
    func documentText(inputMode: InputMode) -> String? {
        if let punctuation = tpsPunctuationChord(inputMode: inputMode) {
            return String(punctuation)
        }
        guard modifiers
            .intersection(.deviceIndependentFlagsMask)
            .isDisjoint(with: Self.hostChords)
        else { return nil }
        guard !isNamedSpecialKey else { return nil }
        guard let characters, !characters.isEmpty else { return nil }
        return characters.unicodeScalars.allSatisfy(Self.isTextScalar) ? characters : nil
    }

    /// The punctuation key under the TPS punctuation chord, or nil when this
    /// key is not one: TPS only (its bare `,` `.` `;` type glyphs, so ⌃ types
    /// their full-width marks), exactly ⌃ among the four chording modifiers, ⇧
    /// allowed since it picks the key (`⌃⇧,` is `⌃<`), and the key one
    /// `FullWidthPunctuation` maps (`isPunctuationChordKey`). Read off
    /// `charactersIgnoringModifiers` because Control rewrites what some keys
    /// type (`⌃[` arrives as Escape). The desktop core's
    /// `ComposingKeyIntent::tps_punctuation_chord`.
    func tpsPunctuationChord(inputMode: InputMode) -> Character? {
        guard inputMode == .tps else { return nil }
        let chording = modifiers.intersection(Self.chordingModifiers)
        guard chording.subtracting(.shift) == .control else { return nil }
        guard !isNamedSpecialKey,
              let unmodified = charactersIgnoringModifiers,
              FullWidthPunctuation.isPunctuationChordKey(unmodified)
        else { return nil }
        return unmodified.first
    }

    /// Not a control character (Cc) and not one of AppKit's function-key
    /// scalars. A format character (Cf) is typed text, so not
    /// `CharacterSet.controlCharacters` (Cc + Cf) — the core's rule (`keys/intent.rs`).
    private static func isTextScalar(_ scalar: Unicode.Scalar) -> Bool {
        scalar.properties.generalCategory != .control && !appKitFunctionKeyRange.contains(scalar.value)
    }
}
