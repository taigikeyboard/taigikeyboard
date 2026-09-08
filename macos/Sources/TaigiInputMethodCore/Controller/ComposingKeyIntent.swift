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
    /// The virtual key code (`NSEvent.keyCode`) — the key's position as the
    /// system reports it, before any layout turns it into a character.
    /// Carried for the recorder's refusal of a shifted number-row key alone:
    /// `⇧3` types `#` on a US layout and `charactersIgnoringModifiers` keeps
    /// Shift, so the number row's codes are the one thing that still says
    /// which key was pressed (`ComposingKeyChord.make(_:)`). Nil for a
    /// snapshot built without an event.
    let keyCode: UInt16?
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
        keyCode: UInt16? = nil,
        navigationKey: NavigationKey? = nil,
    ) {
        self.characters = characters
        self.charactersIgnoringModifiers = charactersIgnoringModifiers ?? characters
        self.keyCode = keyCode
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
            // Read only off a key event: `NSEvent.keyCode` raises on any
            // other type, and the recorder's monitor also sees mouse-ups.
            keyCode: event.type == .keyDown || event.type == .keyUp ? event.keyCode : nil,
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
    /// One of the Telex keys (`ToneInputScheme.telexKeys`), handed to the
    /// engine's `TelexKey` intent rather than appended: the engine decides
    /// which tone it writes, or which initial `z` spells in this input mode.
    case telexKey(String)
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
    /// Commit whichever candidate the bar has highlighted, written as the
    /// output settings render it.
    case commitHighlightedCandidate
    /// Commit the highlighted candidate in the script the output settings do
    /// NOT lead with, leaving the settings alone.
    ///
    /// The 漢羅 key. Taiwanese is written with Han characters and romanization
    /// mixed inside one sentence, and which words a person romanizes is
    /// personal — so the choice belongs to the word being typed, not to a mode
    /// the user has to flip in and out of for it (`CandidateDocumentText
    /// .alternateText(for:settings:)`).
    case commitAlternateScript
    /// Commit the candidate in this slot of the visible page, counting from
    /// zero — what the slot keys address (`CandidateSlotKeySet`: the bare
    /// letters under Standard, the bare digits under Telex).
    case selectCandidateSlot(Int)

    /// AppKit encodes function and arrow keys as private-use scalars rather
    /// than control characters, so a scalar check alone would let F5 through as
    /// composition input.
    private static let appKitFunctionKeyRange: ClosedRange<UInt32> = 0xF700 ... 0xF8FF

    /// Hoisted because this runs once per scalar per keystroke and
    /// `CharacterSet.controlCharacters` materializes a bridged set each access.
    private static let controlCharacters = CharacterSet.controlCharacters

    /// The chords the host owns. Named once because three rules are written
    /// against it — the host-chord guard, the fixed tier's modifier check, and
    /// `isDocumentText` — and a list spelled out at each of them is a list
    /// that can drift apart.
    private static let hostChords: NSEvent.ModifierFlags = [.command, .control, .option]

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
    /// readable in one place: the arrows, the paging keys, Space and the slot
    /// keys all belong to the bar while it is up and to the host or the
    /// document the rest of the time. It defaults to "no bar" because that is the state every
    /// key outside the candidate slice is decided in.
    ///
    /// The two are separate parameters rather than one state because a
    /// composition can run with no bar up, but not the other way round: the
    /// controller only ever shows the bar for candidates a composition fetched,
    /// and takes it down when the composition ends.
    ///
    /// `bindings` carries the parts of the contract the user chooses; why they
    /// arrive as an argument is in `ComposingKeyBindings`. It defaults so that
    /// every call site with no opinion still reads as the shipped contract.
    static func intent(
        for key: KeyEventSnapshot,
        isComposing: Bool,
        isShowingCandidates: Bool = false,
        bindings: ComposingKeyBindings = .default,
    ) -> ComposingKeyIntent {
        let modifiers = key.modifiers.intersection(.deviceIndependentFlagsMask)

        // The fixed tier, read before anything the user can rebind so that no
        // binding can shadow it. These are the keys a user who has mis-bound
        // everything else still has: the way through the candidates, the way
        // out of the composition, and the way to take a character back.
        if isShowingCandidates, !modifiers.contains(.shift),
           modifiers.isDisjoint(with: Self.hostChords),
           let navigation = key.navigationKey
        {
            // Shift deliberately excluded: ⇧← extends a selection, and a user
            // who has finished choosing a candidate should get that back rather
            // than walk the bar a second time.
            return intent(for: navigation)
        }
        if modifiers.isDisjoint(with: Self.hostChords), let first = key.characters?.first {
            switch first {
            case "\u{1B}": // Escape
                return isComposing ? .cancel : .passThrough
            case "\u{8}", "\u{7F}": // Backspace — Control-H and Delete both reach us
                return isComposing ? .deleteBackward : .passThrough
            default:
                break
            }
        }

        // The slot-key tier, read before the user's bindings so that no
        // binding can shadow it. Which keys pick follows from the tone scheme
        // (`ToneInputScheme.slotKeySet`).
        //
        // Only the four chording modifiers are compared, and exactly. Caps
        // Lock and the number pad (`.numericPad`, plus `.function` on some
        // keyboards) say how a key was reached, not which key it is — testing
        // the full flag set would make a keypad `3` miss the slot under Telex
        // and quietly commit the composition instead. Any chording modifier
        // makes the key miss, so `⌃3` keeps falling through to the host guard
        // below.
        if isShowingCandidates, let slot = bindings.slotKeySet.slot(for: key) {
            return .selectCandidateSlot(slot)
        }

        // What the user put on this key, read before the host-chord guard so a
        // chord they deliberately recorded — ⌥Return on a paging row, say —
        // reaches its action. Only an EXACT match does: an unrecorded ⌥ chord
        // still falls to the host below, because the exception is the binding,
        // not the modifier.
        //
        // Above the named-special-key guard for the same reason, since Return,
        // Space and Tab are all keys AppKit names and all keys a user may bind.
        if isComposing, let action = bindings.action(for: key),
           isShowingCandidates || !action.requiresCandidates
        {
            return action.intent
        }

        // Command, control and option chords are the host's shortcuts. This
        // holds mid-composition too: swallowing ⌘S to keep a composition tidy
        // would cost the user their save.
        guard modifiers.isDisjoint(with: Self.hostChords) else {
            return hostKey(isComposing: isComposing)
        }

        guard let characters = key.characters, let first = characters.first else {
            return hostKey(isComposing: isComposing)
        }

        // Keys AppKit names that nothing above claimed. Return and Tab reach
        // here when the user has moved every action off them, and ending the
        // composition first is what keeps a paragraph break landing after the
        // text rather than through it.
        //
        // The check is not subsumed by the scalar rules below: `.lineSeparator`
        // is `U+2028` and `.paragraphSeparator` is `U+2029`, ordinary separator
        // scalars that would otherwise read as document text.
        guard !key.isNamedSpecialKey else { return hostKey(isComposing: isComposing) }

        guard characters.unicodeScalars.allSatisfy(isTextScalar) else {
            return hostKey(isComposing: isComposing)
        }

        // Under Telex the tone letters and `f` are the engine's, not the
        // composition's text. A tone letter or `f` typed outside a
        // composition is document text (like an idle digit): there is no
        // syllable for it to mark. `z` types an initial, so it starts one.
        if bindings.toneScheme == .telex, ToneInputScheme.isTelexKey(first) {
            guard isComposing || ToneInputScheme.startsComposition(first) else { return .passThrough }
            return .telexKey(characters)
        }
        // Under Standard a digit mid-composition is always the tone marker,
        // whatever the buffer looks like — even after `tai5`, where the engine
        // keeps `tai52` verbatim (§10.2). Picking a candidate is the slot
        // keys' alone (`CandidateSlotKeySet`): one set of keys that picks, and
        // a digit that always means the same thing (USER 2026-08-28, retiring
        // the 2026-08-24 rule that let a bare digit pick once no tone could
        // follow — two keys for one slot read as "why does this pick and that
        // not"). Under Telex the digits ARE the slot keys, taken above while
        // the bar is up; with no bar a digit falls through to the punctuation
        // rule and commits the composition ahead of itself.
        if isRomanizationCharacter(first) || (isComposing && bindings.toneScheme == .standard && isToneDigit(first)) {
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
            .isDisjoint(with: hostChords)
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

    /// The numeric tone markers of TL and POJ, which the engine reads as ASCII
    /// digits. A full-width `５` or another script's numeral is a character the
    /// engine cannot parse, so it is document text rather than a tone.
    ///
    /// Visible to `ComposingKeyChord`, which refuses to bind a bare digit:
    /// the digits carry tone under Standard and pick candidates under Telex,
    /// so a chord may not take one away under either.
    static func isToneDigit(_ character: Character) -> Bool {
        character.isASCII && character.isNumber
    }

    /// The characters a TL or POJ syllable is built from. ASCII-only on
    /// purpose: both romanizations are typed as plain letters plus the hyphen
    /// that separates syllables, so a letter from another script is document
    /// text, not input the engine could parse. Tone digits are handled by the
    /// caller, which knows whether a composition is running.
    ///
    /// Every ASCII letter, not only the eighteen a syllable is spelled with:
    /// a custom-dictionary romanization is free text, so all of them must
    /// reach the composition. Under Telex the eight the scheme claims are
    /// taken before this is asked.
    static func isRomanizationCharacter(_ character: Character) -> Bool {
        (character.isLetter && character.isASCII) || character == "-"
    }

    private static func isTextScalar(_ scalar: Unicode.Scalar) -> Bool {
        !controlCharacters.contains(scalar) && !appKitFunctionKeyRange.contains(scalar.value)
    }
}
