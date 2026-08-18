// What one key event means to a composition. Pure classification, no IMK.

import AppKit

/// A key AppKit names that this input method binds to candidate navigation.
///
/// Its own type rather than `NSEvent.SpecialKey` because that one is not
/// `Sendable`, and its own case list rather than raw scalar constants because
/// recognizing `0xF702` as "left arrow" is data extraction, not key policy — the
/// policy stays in `ComposingKeyIntent`, where the rest of the table lives.
enum NavigationKey: Sendable, Equatable {
    case leftArrow
    case rightArrow
    case upArrow
    case downArrow
    case pageUp
    case pageDown

    init?(_ specialKey: NSEvent.SpecialKey) {
        switch specialKey {
        case .leftArrow: self = .leftArrow
        case .rightArrow: self = .rightArrow
        case .upArrow: self = .upArrow
        case .downArrow: self = .downArrow
        case .pageUp: self = .pageUp
        case .pageDown: self = .pageDown
        default: return nil
        }
    }
}

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
    let modifiers: NSEvent.ModifierFlags
    /// Whether AppKit has a name for this key (`NSEvent.SpecialKey`). Which
    /// name is not recorded: the keys this input method binds are recognized by
    /// their characters, and the rest only need to be told apart from text.
    let isNamedSpecialKey: Bool
    /// The navigation key this event is, if it is one of the six.
    let navigationKey: NavigationKey?

    init(
        characters: String?,
        modifiers: NSEvent.ModifierFlags,
        isNamedSpecialKey: Bool,
        charactersIgnoringModifiers: String? = nil,
        navigationKey: NavigationKey? = nil,
    ) {
        self.characters = characters
        self.charactersIgnoringModifiers = charactersIgnoringModifiers ?? characters
        self.modifiers = modifiers
        self.isNamedSpecialKey = isNamedSpecialKey
        self.navigationKey = navigationKey
    }

    init(_ event: NSEvent) {
        self.init(
            characters: event.characters,
            modifiers: event.modifierFlags,
            isNamedSpecialKey: event.specialKey != nil,
            charactersIgnoringModifiers: event.charactersIgnoringModifiers,
            navigationKey: event.specialKey.flatMap(NavigationKey.init),
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
    /// Move the candidate window's selection. The raw direction rather than a
    /// digested "highlight vs page" pair: what `↓` means depends on whether the
    /// window is laid out as a row, a list or a grid, and the window is where
    /// the layout lives (`CandidatePresenter.navigate`).
    case navigate(CandidateNavigation)
    /// Commit whichever candidate the bar has highlighted.
    case commitHighlightedCandidate
    /// Commit the candidate in this slot of the visible page, counting from
    /// zero — what `⌃1` to `⌃9` address.
    case selectCandidateSlot(Int)

    /// AppKit encodes function and arrow keys as private-use scalars rather
    /// than control characters, so a scalar check alone would let F5 through as
    /// composition input.
    private static let appKitFunctionKeyRange: ClosedRange<UInt32> = 0xF700 ... 0xF8FF

    /// Hoisted because this runs once per scalar per keystroke and
    /// `CharacterSet.controlCharacters` materializes a bridged set each access.
    private static let controlCharacters = CharacterSet.controlCharacters

    /// Classifies `key` for a session whose composition is or is not active,
    /// and whose candidate bar is or is not on screen.
    ///
    /// `isComposing` is a parameter rather than a lookup because it changes the
    /// meaning of most keys: Return, Escape, Space and the digits are the
    /// composition's while it runs and the host's the rest of the time. A bare
    /// digit in particular must never start a composition — digits are the
    /// numeric tone markers of TL and POJ (`tai5`), so they only mean "tone"
    /// once there is romanization to carry it, matching iOS
    /// (`ios/Sources/TaigiKeyboard/Actions/ActionHandler+KeyActions.swift:61-70`).
    ///
    /// `isShowingCandidates` is the second state a key's meaning turns on, and
    /// it is here rather than in the controller so that the whole table stays
    /// readable in one place: the arrows, the paging keys, Space and `⌃1`…`⌃9`
    /// all belong to the bar while it is up and to the host or the document the
    /// rest of the time. It defaults to "no bar" because that is the state every
    /// key outside the candidate slice is decided in.
    static func intent(
        for key: KeyEventSnapshot,
        isComposing: Bool,
        isShowingCandidates: Bool = false,
    ) -> ComposingKeyIntent {
        let modifiers = key.modifiers.intersection(.deviceIndependentFlagsMask)

        // Read before the host-chord guard below, which would otherwise hand
        // every Control chord straight to the host. Classified from the
        // unmodified characters: Control rewrites the digits it is chorded with.
        //
        // Control must be held and the other three chording modifiers must not
        // be; everything else AppKit reports is ignored on purpose. Caps Lock
        // does not change what a digit key means, and the number pad sets
        // `.numericPad` (and `.function` on some keyboards) — testing for an
        // exact flag set would make `⌃3` select on the top row and quietly
        // commit the composition on the keypad.
        if isShowingCandidates,
           modifiers.contains(.control),
           modifiers.isDisjoint(with: [.command, .option, .shift]),
           let slot = directSelectionSlot(key.charactersIgnoringModifiers)
        {
            return .selectCandidateSlot(slot)
        }

        // Command, control and option chords are the host's shortcuts. This
        // holds mid-composition too: swallowing ⌘S to keep a composition tidy
        // would cost the user their save.
        let hostChords: NSEvent.ModifierFlags = [.command, .control, .option]
        guard modifiers.isDisjoint(with: hostChords) else {
            return hostKey(isComposing: isComposing)
        }

        // Shift deliberately excluded: ⇧← extends a selection, and a user who
        // has finished choosing a candidate should get that back rather than
        // walk the bar a second time.
        if isShowingCandidates, !modifiers.contains(.shift),
           let navigation = key.navigationKey
        {
            return intent(for: navigation)
        }

        guard let characters = key.characters, let first = characters.first else {
            return hostKey(isComposing: isComposing)
        }

        switch first {
        case "\r", "\u{3}": // Return, Enter
            // Commits the literal the marked region shows, never the highlighted
            // candidate: with the bar up the two are different strings, and Enter
            // is the only way to keep what was actually typed.
            return isComposing ? .commit : .passThrough
        case "\u{1B}": // Escape
            return isComposing ? .cancel : .passThrough
        case "\u{8}", "\u{7F}": // Backspace — Control-H and Delete both reach us
            return isComposing ? .deleteBackward : .passThrough
        case " ":
            // Space picks the highlighted candidate while the bar is up. With no
            // bar it is ordinary document text that ends the composition it
            // follows, which is what the `commitThenInsert` arm below does for
            // every other printable character.
            if isShowingCandidates {
                return .commitHighlightedCandidate
            }
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

    /// True when `key` is text the host will put into its document, rather than
    /// a key it will act on.
    ///
    /// Lives here because it is the same question the classification above
    /// already answers for itself, and answering it twice in two files is how
    /// the two drift. The pass-through path asks it so that punctuation typed
    /// outside a composition can be reported to the engine as the end of a
    /// context, while Escape, Return and the arrows — which are pass-through
    /// too — are not.
    ///
    /// Takes the whole event rather than its characters: `⌘.` and a typed `.`
    /// carry the same character, and one of them is a host command that inserts
    /// nothing. The chording modifiers are what tells them apart.
    static func isDocumentText(_ key: KeyEventSnapshot) -> Bool {
        guard key.modifiers
            .intersection(.deviceIndependentFlagsMask)
            .isDisjoint(with: [.command, .control, .option])
        else { return false }
        guard !key.isNamedSpecialKey else { return false }
        guard let characters = key.characters, !characters.isEmpty else { return false }
        return characters.unicodeScalars.allSatisfy(isTextScalar)
    }

    /// A key the host owns. It still ends any composition first, so the host
    /// never acts on a document that has an unfinished one in it.
    private static func hostKey(isComposing: Bool) -> ComposingKeyIntent {
        isComposing ? .commitThenPassThrough : .passThrough
    }

    /// The six navigation keys, handed through as directions. The mapping is
    /// one-to-one on purpose: what a direction DOES — walk, page, scroll,
    /// expand — belongs to the candidate window's layout, not to this table,
    /// which only decides that the key is the window's while it is up.
    private static func intent(for navigation: NavigationKey) -> ComposingKeyIntent {
        switch navigation {
        case .leftArrow: .navigate(.left)
        case .rightArrow: .navigate(.right)
        case .upArrow: .navigate(.up)
        case .downArrow: .navigate(.down)
        case .pageUp: .navigate(.pageUp)
        case .pageDown: .navigate(.pageDown)
        }
    }

    /// The candidate slot `⌃1`…`⌃9` addresses, counting from zero. `⌃0` is not
    /// a chord this input method binds: the bar holds nine candidates because
    /// nine is what the digits can name without one of them meaning "the tenth".
    private static func directSelectionSlot(_ charactersIgnoringModifiers: String?) -> Int? {
        guard let character = charactersIgnoringModifiers?.first,
              character.isASCII,
              let digit = character.wholeNumberValue,
              (1 ... 9).contains(digit)
        else { return nil }
        return digit - 1
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
