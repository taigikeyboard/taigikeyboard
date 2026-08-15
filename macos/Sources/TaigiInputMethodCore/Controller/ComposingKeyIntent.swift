// What one key event means to a composition. Pure classification, no IMK.

import AppKit

/// The parts of an `NSEvent` a composing decision is made from.
///
/// A value rather than the event itself so the classification can be reasoned
/// about — and tested — without an AppKit event object, and so the decision can
/// be handed between isolation domains: `NSEvent` is a reference type that
/// cannot cross one.
struct KeyEventSnapshot: Sendable {
    let characters: String?
    let modifiers: NSEvent.ModifierFlags
    /// Whether AppKit has a name for this key (`NSEvent.SpecialKey`). Which
    /// name is not recorded: the keys this input method binds are recognized by
    /// their characters, and the rest only need to be told apart from text.
    let isNamedSpecialKey: Bool

    init(characters: String?, modifiers: NSEvent.ModifierFlags, isNamedSpecialKey: Bool) {
        self.characters = characters
        self.modifiers = modifiers
        self.isNamedSpecialKey = isNamedSpecialKey
    }

    init(_ event: NSEvent) {
        self.init(
            characters: event.characters,
            modifiers: event.modifierFlags,
            isNamedSpecialKey: event.specialKey != nil,
        )
    }
}

/// The composing meaning of a key event, decided before any engine call.
///
/// Split out of the controller because this is the whole key contract of the
/// input method: every key the host never sees is a key the user can no longer
/// use in their app, so the table deserves to be readable and testable on its
/// own.
enum ComposingKeyIntent: Equatable {
    /// A romanization character to append to the composition.
    case input(String)
    case deleteBackward
    /// Finish the composition as rendered.
    case commit
    /// Abandon the composition without writing to the document.
    case cancel
    /// Finish the composition and write `text` after it, as one step. Space and
    /// punctuation typed mid-composition: the character belongs to the
    /// document, and delivering it separately would let the host see it before
    /// the composition it follows.
    case commitThenInsert(String)
    /// The host must receive this event, and the composition has to be finished
    /// into the document first. The host is about to do something to that
    /// document — move the caret, run a shortcut, change the selection — and a
    /// composition left running would then be re-rendered somewhere it does not
    /// belong. Matches khiin's ignored-event path
    /// (`references/khiin-rs/swift/osx/src/controller/InputController+handler.swift:56-58`),
    /// which commits before returning `false`.
    case commitThenPassThrough
    /// Not ours — the host must receive this event, and there is no composition
    /// to finish first.
    case passThrough

    /// AppKit encodes function and arrow keys as private-use scalars rather
    /// than control characters, so a scalar check alone would let F5 through as
    /// composition input.
    private static let appKitFunctionKeyRange: ClosedRange<UInt32> = 0xF700 ... 0xF8FF

    /// Hoisted because this runs once per scalar per keystroke and
    /// `CharacterSet.controlCharacters` materializes a bridged set each access.
    private static let controlCharacters = CharacterSet.controlCharacters

    /// Classifies `key` for a session whose composition is or is not active.
    ///
    /// `isComposing` is a parameter rather than a lookup because it changes the
    /// meaning of most keys: Return, Escape, Space and the digits are the
    /// composition's while it runs and the host's the rest of the time. A bare
    /// digit in particular must never start a composition — digits are the
    /// numeric tone markers of TL and POJ (`tai5`), so they only mean "tone"
    /// once there is romanization to carry it, matching iOS
    /// (`ios/Sources/TaigiKeyboard/Actions/ActionHandler+KeyActions.swift:61-70`).
    static func intent(for key: KeyEventSnapshot, isComposing: Bool) -> ComposingKeyIntent {
        // Command, control and option chords are the host's shortcuts. This
        // holds mid-composition too: swallowing ⌘S to keep a composition tidy
        // would cost the user their save.
        let hostChords: NSEvent.ModifierFlags = [.command, .control, .option]
        guard key.modifiers.intersection(.deviceIndependentFlagsMask)
            .isDisjoint(with: hostChords)
        else { return hostKey(isComposing: isComposing) }

        guard let characters = key.characters, let first = characters.first else {
            return hostKey(isComposing: isComposing)
        }

        switch first {
        case "\r", "\u{3}": // Return, Enter
            return isComposing ? .commit : .passThrough
        case "\u{1B}": // Escape
            return isComposing ? .cancel : .passThrough
        case "\u{8}", "\u{7F}": // Backspace — Control-H and Delete both reach us
            return isComposing ? .deleteBackward : .passThrough
        default:
            break
        }

        // Checked after the keys above, which are `SpecialKey`s we bind on
        // purpose. What remains are keys AppKit names but we do not act on, and
        // the check is not subsumed by the scalar rules below: `.lineSeparator`
        // is `U+2028` and `.paragraphSeparator` is `U+2029`, ordinary separator
        // scalars that would otherwise read as document text.
        guard !key.isNamedSpecialKey else { return hostKey(isComposing: isComposing) }

        guard characters.unicodeScalars.allSatisfy(isTextScalar) else {
            return hostKey(isComposing: isComposing)
        }

        if isRomanizationCharacter(first) || (isComposing && isToneDigit(first)) {
            return .input(characters)
        }
        // Everything else printable — space, punctuation, a character from
        // another script — is document text. It ends the composition it was
        // typed after, and is the host's business when there is none.
        return isComposing ? .commitThenInsert(characters) : .passThrough
    }

    /// A key the host owns. It still ends any composition first, so the host
    /// never acts on a document that has an unfinished one in it.
    private static func hostKey(isComposing: Bool) -> ComposingKeyIntent {
        isComposing ? .commitThenPassThrough : .passThrough
    }

    /// The numeric tone markers of TL and POJ, which the engine reads as ASCII
    /// digits. A full-width `５` or another script's numeral is a character the
    /// engine cannot parse, so it is document text rather than a tone.
    private static func isToneDigit(_ character: Character) -> Bool {
        character.isASCII && character.isNumber
    }

    /// The characters a TL or POJ syllable is built from. ASCII-only on
    /// purpose: both romanizations are typed as plain letters plus the hyphen
    /// that separates syllables, so a letter from another script is document
    /// text, not input the engine could parse. Tone digits are handled by the
    /// caller, which knows whether a composition is running.
    private static func isRomanizationCharacter(_ character: Character) -> Bool {
        (character.isLetter && character.isASCII) || character == "-"
    }

    private static func isTextScalar(_ scalar: Unicode.Scalar) -> Bool {
        !controlCharacters.contains(scalar) && !appKitFunctionKeyRange.contains(scalar.value)
    }
}
