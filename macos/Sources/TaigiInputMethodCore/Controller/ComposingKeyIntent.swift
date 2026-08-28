// What one key event means to a composition. Pure classification, no IMK.

import AppKit
import Carbon.HIToolbox

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
    /// Carried for the shifted-digit chord alone: `⇧3` types `#` on a US
    /// layout and `charactersIgnoringModifiers` keeps Shift, so the number
    /// row's codes are the one thing that still says which key was pressed
    /// (`ComposingKeyIntent.shiftedDigitSlot`). Nil for a snapshot built
    /// without an event.
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
    /// zero — what the slot keys address (`CandidateSlotKeySet`, `⇧1`…`⇧9`,
    /// and a bare digit where one can pick).
    case selectCandidateSlot(Int)

    /// AppKit encodes function and arrow keys as private-use scalars rather
    /// than control characters, so a scalar check alone would let F5 through as
    /// composition input.
    private static let appKitFunctionKeyRange: ClosedRange<UInt32> = 0xF700 ... 0xF8FF

    /// Hoisted because this runs once per scalar per keystroke and
    /// `CharacterSet.controlCharacters` materializes a bridged set each access.
    private static let controlCharacters = CharacterSet.controlCharacters

    /// The chords the host owns. Named once because three rules are written
    /// against it — the host-chord guard, the candidate-slot chord's
    /// exclusivity, and `isDocumentText` — and a list spelled out at each of
    /// them is a list that can drift apart.
    private static let hostChords: NSEvent.ModifierFlags = [.command, .control, .option]

    /// Every modifier that turns a key into a chord. The candidate-slot chord
    /// takes one of them and must see none of the rest.
    private static let chordingModifiers: NSEvent.ModifierFlags = hostChords.union(.shift)

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
    ///
    /// `rawInput` is the composition's raw buffer — the letters and digits as
    /// typed, before any tone mark is rendered. Only its last character is
    /// read, and only to answer whether a digit could still be romanization
    /// (`canTypeToneDigit`).
    ///
    /// `isSelectionLatched` is the third state a key's meaning turns on: while
    /// it holds, a bare digit picks a candidate even where the buffer could
    /// still take a tone. The controller owns it — it is session state, not a
    /// setting — and `selectionLatch(after:wasLatched:)` is the rule it keeps
    /// it by.
    static func intent(
        for key: KeyEventSnapshot,
        isComposing: Bool,
        isShowingCandidates: Bool = false,
        bindings: ComposingKeyBindings = .default,
        rawInput: String = "",
        isSelectionLatched: Bool = false,
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

        // The slot-key tier, read before the host-chord guard below — which
        // would otherwise hand every Control chord straight to the host — and
        // before the user's bindings, so that no binding can shadow it. Which
        // keys pick is the user's set (`CandidateSlotKeySet`).
        //
        // Only the four chording modifiers are compared, and exactly. Caps
        // Lock and the number pad (`.numericPad`, plus `.function` on some
        // keyboards) say how a key was reached, not which key it is — testing
        // the full flag set would make `⌃3` select on the top row and quietly
        // commit the composition on the keypad. A modifier the user did NOT
        // choose keeps falling through to the host guard below, so `⌥3` stays
        // the host's while Control holds the slots.
        if isShowingCandidates, let slot = bindings.slotKeySet.slot(for: key) {
            return .selectCandidateSlot(slot)
        }

        // The bare-digit tier. A digit takes its romanization meaning first —
        // the tone of the syllable being typed — and only when it cannot have
        // one does it select a candidate. `canTypeToneDigit` is what decides,
        // and its rule is TL/POJ grammar rather than a mode the user has to
        // hold in their head: nothing can follow a tone digit but a new
        // syllable, which always starts with a letter, so a digit typed after
        // `tai5` was never going to be input.
        //
        // `isSelectionLatched` is the second way in, and it exists because the
        // grammar rule alone never reaches the typist it was written for: a
        // toneless buffer ALWAYS ends in a letter (`tai`, `taigi`), so
        // `canTypeToneDigit` is permanently true and someone who does not know
        // the tones — the majority, since a Taigi speaker need never have
        // learnt them — is left on the chord forever. The latch is the user
        // saying, with `↓`, that they are choosing rather than typing; see
        // `selectionLatch(after:wasLatched:)`.
        //
        // Gated on the bar being up, because selecting needs something to
        // select, and on no chording modifier, so the modifier tier above
        // keeps its chords whatever the buffer looks like.
        if isComposing,
           isShowingCandidates,
           isSelectionLatched || !canTypeToneDigit(after: rawInput),
           modifiers.isDisjoint(with: Self.chordingModifiers),
           let slot = directSelectionSlot(key.charactersIgnoringModifiers)
        {
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

        // A digit still reaches the engine as a tone whenever it could be one.
        // Where it could not, the slot tier above already took `1`…`9` if
        // there were candidates to pick; anything left over — `0`, or a digit
        // typed with no bar up — falls through to document text below, which
        // is what the engine would have made of `tai52` anyway.
        if isRomanizationCharacter(first)
            || (isComposing && canTypeToneDigit(after: rawInput) && isToneDigit(first))
        {
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

    /// Whether a digit typed now could be part of the romanization — the rule
    /// that decides what a bare `1`…`9` means mid-composition.
    ///
    /// TL and POJ spell a syllable as letters plus at most ONE trailing tone
    /// digit (`tai5`), and a syllable can only begin with a letter — a hyphen
    /// separates syllables, so it too is followed by letters. So a digit is
    /// romanization exactly while the buffer ends in a letter:
    ///
    /// - `tai` + `5` → the tone of `tai`
    /// - `taigi` + `2` → the tone of the syllable being typed
    /// - `tai5` + `2` → nothing TL or POJ can spell; the engine would keep it
    ///   verbatim as `tai52` (§10.2), so the key is free to select instead
    /// - `tai5-` + `2` → same: a syllable starts here, and it starts with a
    ///   letter
    ///
    /// An empty buffer answers true, so the classifier's digit handling reads
    /// the same for a composition that has not started as for one whose first
    /// character is still to come.
    ///
    /// A letter tail is an affordance, not a phonotactic proof: the composing
    /// alphabet takes every ASCII letter so a custom-dictionary romanization
    /// can be typed, so `xyz` + `2` still reaches the engine — which keeps an
    /// invalid syllable verbatim rather than toning it (§10.2). What the rule
    /// guarantees is the other direction: where it says "not romanization",
    /// nothing TL or POJ spells could have followed.
    static func canTypeToneDigit(after rawInput: String) -> Bool {
        guard let last = rawInput.last else { return true }
        return last.isLetter
    }

    /// How the selection latch stands after `intent` has been carried out.
    ///
    /// The latch is what lets a bare `1`…`9` pick a candidate out of a buffer
    /// that could still take a tone digit. It turns on for one gesture only —
    /// `↓`, the key that means "into the list" on every layout — and off again
    /// the moment the user does something that is typing rather than choosing.
    ///
    /// `↓` alone, and not the other ways of walking the bar: `⇥` and the arrows
    /// are how a Taigi typist LOOKS at the homophones before deciding which
    /// tone to add, so latching on them would turn a glance into a mode change
    /// and make the very next `5` commit a candidate instead of toning the
    /// syllable. `↓` is the one a user already reaches for to go INTO a
    /// list, and binding it costs no new key and no new `ComposingAction`.
    ///
    /// Keyed on the key pressed, NOT on where the selection lands — those are
    /// different questions, and only the first one says what the user meant.
    /// The horizontal layout reads `.down` and `.pageDown` as the same page
    /// turn (`HorizontalPageLayout.target(for:from:)`), so on that layout the
    /// two keys move the selection identically and only `↓` latches. That is
    /// the intended asymmetry: the paging keys — `⇞`, `⇟`, `[` and `]` — are
    /// for looking further down the list, which is the glance this rule is
    /// written to protect.
    ///
    /// Typing clears it, so the way back is the next thing the user was going
    /// to type anyway; so does anything that ends the composition, which is
    /// where the bar and the list it selects from go too.
    ///
    /// Picking a candidate does NOT clear it, and that is deliberate. It is
    /// choosing, not typing, and what it did is not knowable here: a slot the
    /// page never filled commits nothing at all, and a candidate that consumes
    /// only part of the buffer nails a prefix and leaves the user mid-choice
    /// over the rest — `taigikhipuann` picking 台語 and then still owing a
    /// candidate for `khipuann`. Where a pick really does finish the
    /// composition, the bar comes down with it and
    /// `TaigiInputController.dismissCandidates` clears the latch there, which
    /// is the one place that knows.
    ///
    /// The rest — the other navigations, and a pass-through the host owns —
    /// leave it alone: none of them is a statement either way.
    ///
    /// Pure, and separate from the classification above, because it runs AFTER
    /// an intent rather than producing one: the key that latches is classified
    /// against the latch as it stood when it was pressed.
    static func selectionLatch(after intent: ComposingKeyIntent, wasLatched: Bool) -> Bool {
        switch intent {
        case .navigate(.down):
            true
        case .input, .deleteBackward, .commit, .cancel, .commitThenInsert,
             .commitThenPassThrough:
            false
        case .commitHighlightedCandidate, .commitAlternateScript, .selectCandidateSlot,
             .navigate, .passThrough:
            wasLatched
        }
    }

    /// The slot `key` picks as one of the `⇧1`…`⇧9` chords, counting from
    /// zero, or nil when it is not one — the `shift` set's rule
    /// (`CandidateSlotKeySet.slot(for:)`), asked by the recorder's refusal as
    /// well (`ComposingKeyChord.make(_:)`), which reserves these chords
    /// whichever set is live.
    ///
    /// Shift, and only Shift, among the chording modifiers; then the digit,
    /// read from the key code first, because Shift rewrites the characters:
    /// `⇧3` on a US layout types `#`, and the number row's code is what still
    /// says which key that was. A key code is a position, so this is the same
    /// nine keys on every layout — on AZERTY, where the bare row types
    /// `& é " …` and the digits ARE the shifted characters, `⇧&` is still the
    /// chord that picks the first candidate. (A remapping tool that sends
    /// another code for the key is respected: the key is then whatever it was
    /// remapped to.) Then from `charactersIgnoringModifiers`, for a digit the
    /// number row does not carry: the keypad's, which Shift leaves alone.
    static func shiftedDigitSlot(_ key: KeyEventSnapshot) -> Int? {
        guard key.modifiers.intersection(chordingModifiers) == .shift else { return nil }
        if let keyCode = key.keyCode, let slot = numberRowKeyCodes.firstIndex(of: keyCode) {
            return slot
        }
        return directSelectionSlot(key.charactersIgnoringModifiers)
    }

    /// The `1`…`9` keys of the number row, in slot order. Carbon's ANSI
    /// codes are positions on the keyboard and hold on ISO and JIS boards
    /// too; note that `6` and `9` sit out of numeric order.
    static let numberRowKeyCodes: [UInt16] = [
        kVK_ANSI_1, kVK_ANSI_2, kVK_ANSI_3, kVK_ANSI_4, kVK_ANSI_5,
        kVK_ANSI_6, kVK_ANSI_7, kVK_ANSI_8, kVK_ANSI_9,
    ].map(UInt16.init)

    /// The slot a digit `1`…`9` names, counting from zero. `0` names none: the
    /// bar holds nine candidates because nine is what the digits can name
    /// without one of them meaning "the tenth".
    ///
    /// Shared by every rule that reads a digit as a slot — the shifted digit
    /// above, the modifier sets in `CandidateSlotKeySet`, and the bare-digit
    /// tier — so the nine digits address the nine slots the same way on all of
    /// them.
    static func directSelectionSlot(_ charactersIgnoringModifiers: String?) -> Int? {
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
    ///
    /// Visible to `ComposingKeyChord`, which refuses to bind a bare digit:
    /// the digits carry tone, so a chord may not take one away.
    static func isToneDigit(_ character: Character) -> Bool {
        character.isASCII && character.isNumber
    }

    /// The characters a TL or POJ syllable is built from. ASCII-only on
    /// purpose: both romanizations are typed as plain letters plus the hyphen
    /// that separates syllables, so a letter from another script is document
    /// text, not input the engine could parse. Tone digits are handled by the
    /// caller, which knows whether a composition is running.
    ///
    /// Wider than `ComposingKeyChord.syllableLetters` on purpose: a
    /// custom-dictionary romanization is free text, so every ASCII letter
    /// must reach the composition.
    static func isRomanizationCharacter(_ character: Character) -> Bool {
        (character.isLetter && character.isASCII) || character == "-"
    }

    private static func isTextScalar(_ scalar: Unicode.Scalar) -> Bool {
        !controlCharacters.contains(scalar) && !appKitFunctionKeyRange.contains(scalar.value)
    }
}
