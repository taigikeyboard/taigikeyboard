// Executable spec for the part of the key contract the user chooses.

import AppKit
@testable import TaigiInputMethodCore
import XCTest

/// `ComposingKeyIntentTests` pins the contract as it ships. This one pins what
/// a rebinding changes about it, and — just as importantly — what it cannot:
/// a setting that quietly took a typing key away, or left a composition with no
/// way out, would be worse than no setting.
final class ComposingKeyBindingsTests: XCTestCase {
    // MARK: - What a chord may be

    /// The keys a TL or POJ syllable is spelled with, tone marker included. A
    /// recorder that took one would leave the user unable to type it. The
    /// whole alphabet is spelled out so a letter dropped from the set by
    /// mistake fails here; `r` and `c` are the two easy to mistake for free —
    /// `r` spells the dialectal `ir`/`er` finals and `c` the POJ `ch`/`chh`
    /// initials.
    func testTypingKeys_cannotBeRecordedBare() {
        for key in "abceghijklmnoprstu".map(String.init) + ["A", "5", "0", "-"] {
            XCTAssertEqual(
                ComposingKeyChord.make(key: key, modifiers: []),
                .failure(.typesRomanization),
                "'\(key)' spells romanization — binding it costs the user the key",
            )
        }
    }

    /// The eight letters no TL or POJ syllable uses are the keys a user has
    /// free to bind bare — the whole point of narrowing the refusal to the
    /// syllable alphabet.
    func testNonSyllableLetters_canBeRecordedBare() throws {
        for key in ["d", "f", "q", "v", "w", "x", "y", "z"] {
            let chord = try ComposingKeyChord.make(key: key, modifiers: []).get()
            XCTAssertEqual(chord.key, key)
            XCTAssertEqual(chord.modifiers, [])
        }
    }

    /// The factory may receive a capital, and the refusal reads the key
    /// `normalized` has already folded: a capital of a free letter records,
    /// a capital of a syllable letter still does not.
    func testCapitalsFoldToTheirLetter_beforeTheRefusalDecides() throws {
        let chord = try ComposingKeyChord.make(key: "Z", modifiers: .shift).get()
        XCTAssertEqual(chord.key, "z")
        XCTAssertEqual(chord.modifiers, .shift)

        XCTAssertEqual(
            ComposingKeyChord.make(key: "R", modifiers: .shift),
            .failure(.typesRomanization),
        )
    }

    /// Bare `z` and ⇧Z are two different chords on one key, and each fires
    /// only on its own event.
    func testABareLetterChord_andItsShiftedTwin_doNotCrossMatch() throws {
        let bare = try ComposingKeyChord.make(key: "z", modifiers: []).get()
        let shifted = try ComposingKeyChord.make(key: "Z", modifiers: .shift).get()

        let bareEvent = try snapshot("z")
        let shiftedEvent = try snapshot("Z", modifiers: .shift, unmodified: "z")

        XCTAssertTrue(bare.matches(bareEvent))
        XCTAssertFalse(bare.matches(shiftedEvent))
        XCTAssertTrue(shifted.matches(shiftedEvent))
        XCTAssertFalse(shifted.matches(bareEvent))
    }

    /// With a chording modifier they are ordinary chords: ⌥A types no letter.
    func testTypingKeys_canBeRecordedWithAChordingModifier() {
        for modifiers in [NSEvent.ModifierFlags.command, .control, .option] {
            XCTAssertNoThrow(
                try ComposingKeyChord.make(key: "a", modifiers: modifiers).get(),
                "⌥A and friends type nothing, so they are bindable",
            )
        }
    }

    /// Shift is how a capital is typed, not a chord — ⇧A is still the letter A.
    func testShiftAlone_doesNotMakeATypingKeyBindable() {
        XCTAssertEqual(
            ComposingKeyChord.make(key: "a", modifiers: .shift),
            .failure(.typesRomanization),
        )
    }

    /// The fixed tier. Reserved whatever modifiers are held, so no binding can
    /// shadow the way through the candidates or the way out of a composition.
    func testReservedKeys_cannotBeRecordedAtAll() throws {
        let arrows = try [
            NSLeftArrowFunctionKey, NSRightArrowFunctionKey,
            NSUpArrowFunctionKey, NSDownArrowFunctionKey,
            NSPageUpFunctionKey, NSPageDownFunctionKey,
        ].map { try String(XCTUnwrap(UnicodeScalar($0))) }

        for key in arrows + ["\u{1B}", "\u{8}", "\u{7F}"] {
            for modifiers in [NSEvent.ModifierFlags(), .control, [.command, .shift]] {
                XCTAssertEqual(
                    ComposingKeyChord.make(key: key, modifiers: modifiers),
                    .failure(.reservedKey),
                )
            }
        }
    }

    func testAnEventWithNoCharacters_recordsNothing() {
        XCTAssertEqual(ComposingKeyChord.make(key: nil, modifiers: []), .failure(.noKey))
        XCTAssertEqual(ComposingKeyChord.make(key: "", modifiers: []), .failure(.noKey))
    }

    /// Modifiers that say how a key was reached rather than which key it is.
    /// Keeping them would record ⌃3-on-the-keypad as a different chord from
    /// ⌃3 on the top row.
    func testRecording_dropsTheNonChordingModifiers() throws {
        let chord = try ComposingKeyChord
            .make(key: "]", modifiers: [.control, .capsLock, .numericPad, .function])
            .get()

        XCTAssertEqual(chord.modifiers, .control)
    }

    // MARK: - Storage

    func testChords_roundTripThroughTheirRawValue() throws {
        for (key, modifiers) in [
            (" ", NSEvent.ModifierFlags()),
            ("\r", .shift),
            ("z", []),
            ("Z", .shift),
            ("]", [.command, .control, .option, .shift]),
        ] as [(String, NSEvent.ModifierFlags)] {
            let chord = try ComposingKeyChord.make(key: key, modifiers: modifiers).get()
            XCTAssertEqual(ComposingKeyChord(rawValue: chord.rawValue), chord)
        }
    }

    func testRawValues_stayStable() throws {
        XCTAssertEqual(try ComposingKeyChord.make(key: " ", modifiers: []).get().rawValue, "|0020")
        XCTAssertEqual(try ComposingKeyChord.make(key: "\r", modifiers: .shift).get().rawValue, "s|000D")
        XCTAssertEqual(
            try ComposingKeyChord.make(key: "]", modifiers: [.control, .option]).get().rawValue,
            "co|005D",
        )
    }

    /// A hand-edited defaults value goes through the same gate the recorder
    /// does, so a domain edited behind the app's back cannot install a binding
    /// that swallows the letters of a syllable.
    func testRawValues_thatWouldTakeATypingKey_doNotParse() {
        XCTAssertNil(ComposingKeyChord(rawValue: "|0061"), "bare 'a'")
        XCTAssertNil(ComposingKeyChord(rawValue: "s|0035"), "⇧5")
        XCTAssertNil(ComposingKeyChord(rawValue: "c|F702"), "⌃← is still an arrow")
        XCTAssertNil(ComposingKeyChord(rawValue: "garbage"))
        XCTAssertNil(ComposingKeyChord(rawValue: "x|0020"), "unknown modifier letter")
    }

    // MARK: - Defaults

    /// The keys a user arriving from the system Zhuyin input method already
    /// knows. Two places that keyboard cannot be matched: it picks candidates
    /// with bare digits, which are tone markers here, and it walks the list
    /// with Space, which here writes the other script — the 漢羅 key, which a
    /// Mandarin keyboard has no equivalent of. Walking moved to ⇥, pairing with
    /// the ⇧⇥ that already walked back.
    func testDefaults_followTheSystemZhuyinKeyboard() throws {
        let bindings = ComposingKeyBindings.default

        XCTAssertEqual(bindings.chord(for: .nextCandidate), try chord("\t"))
        XCTAssertEqual(bindings.chord(for: .commitAlternateScript), try chord(" "))
        XCTAssertEqual(bindings.chord(for: .confirmHighlighted), try chord("\r"))
        XCTAssertEqual(bindings.chord(for: .commitLiteral), try chord("\r", .shift))
        XCTAssertEqual(bindings.chord(for: .pageBackward), try chord("["))
        XCTAssertEqual(bindings.chord(for: .pageForward), try chord("]"))
        XCTAssertEqual(bindings.slotModifier, .control)
    }

    /// Walking BACK through the candidates is the one action the system Zhuyin
    /// keyboard has no key for, so its default is borrowed from McBopomofo's ⇧⇥
    /// rather than inherited (`KeyHandler.mm:817-870`).
    func testTheReverseWalk_takesMcBopomofosShiftTab() throws {
        XCTAssertEqual(
            ComposingKeyBindings.default.chord(for: .previousCandidate),
            try chord("\t", .shift),
        )
    }

    /// Every action, not just the ones a case names: a new one added with a
    /// blank default would be a blank row.
    func testDefaults_leaveNoActionUnbound() {
        let bindings = ComposingKeyBindings.default

        for action in ComposingAction.allCases {
            XCTAssertNotNil(bindings.chord(for: action), "\(action.rawValue) starts blank")
        }
    }

    /// An upgrade that hands an untouched row a default the user had already
    /// put on another action must not empty the row they set: what they
    /// recorded wins, and the row holding only a default gives way.
    func testAStoredChord_outranksADefaultThatArrivesOnTopOfIt() {
        let bracket = ComposingAction.pageForward.defaultChord

        let bindings = ComposingKeyBindings(chords: [.nextCandidate: bracket])

        XCTAssertEqual(bindings.chord(for: .nextCandidate), bracket, "the recorded row lost its chord")
        XCTAssertNil(bindings.chord(for: .pageForward), "two rows answer to ]")
    }

    /// The same the other way round, where `allCases` order would have let the
    /// recording win on its own — the rule is provenance, not position.
    func testADefault_givesWayWhateverTheRosterOrder() {
        let bracket = ComposingAction.pageForward.defaultChord

        let bindings = ComposingKeyBindings(chords: [.pageBackward: bracket])

        XCTAssertEqual(bindings.chord(for: .pageBackward), bracket)
        XCTAssertNil(bindings.chord(for: .pageForward))
    }

    /// The upgrade case for the 2026-08-25 move: an install where the user had
    /// deliberately RECORDED Space on `nextCandidate` keeps it there, and the
    /// new 漢羅 action arrives empty rather than taking a key out from under
    /// them. The provenance rule already says this — a recorded chord outranks
    /// a default landing on top of it — and this pins that it covers the move.
    func testAUserWhoRecordedSpaceOnWalking_keepsIt() throws {
        let space = ComposingAction.commitAlternateScript.defaultChord

        let bindings = ComposingKeyBindings(chords: [.nextCandidate: space])

        XCTAssertEqual(bindings.chord(for: .nextCandidate), space)
        XCTAssertNil(
            bindings.chord(for: .commitAlternateScript),
            "the new action does not take a key the user chose for another row",
        )
    }

    /// A case added to the roster but not to a group would be missing from
    /// both the pane and the menu, which draw from the groups.
    func testTheGroups_holdEveryActionExactlyOnce() {
        let grouped = ComposingAction.groups.flatMap(\.self)

        XCTAssertEqual(Set(grouped), Set(ComposingAction.allCases))
        XCTAssertEqual(grouped.count, ComposingAction.allCases.count, "an action is in two groups")
    }

    func testEveryAction_hasItsOwnSettingsKey() {
        let names = ComposingAction.allCases.map(\.settingsKeyName)

        XCTAssertEqual(Set(names).count, names.count, "two actions share a settings key: \(names)")
    }

    // MARK: - Resolution

    func testAChordRecordedTwice_staysOnTheLastActionOnly() throws {
        // Neither row is on its own default, so provenance cannot separate them
        // and `allCases` order is what decides — the tiebreak this case is for.
        let recorded = try TestFixtures.chordNoDefaultHolds()
        let bindings = ComposingKeyBindings(chords: [
            .pageForward: recorded,
            .pageBackward: recorded,
        ])

        XCTAssertEqual(bindings.chord(for: .pageBackward), recorded)
        XCTAssertNil(bindings.chord(for: .pageForward), "the earlier row gives the chord up")
    }

    /// Between them these two are the only way to end a composition into the
    /// document. A roster where both went missing would leave a user with a
    /// composition they can only cancel.
    func testAnAlwaysBoundAction_getsItsDefaultBackWhenCleared() throws {
        for action in ComposingAction.alwaysBound {
            let bindings = ComposingKeyBindings(chords: [action: nil])

            XCTAssertEqual(
                bindings.chord(for: action),
                action.defaultChord,
                "\(action) may not be left with no key",
            )
        }
    }

    /// Restoring one must not leave its default on two rows.
    func testRestoringAnAlwaysBoundAction_takesItsChordBack() throws {
        let bindings = ComposingKeyBindings(chords: [
            .confirmHighlighted: nil,
            .pageForward: try chord("\r"),
        ])

        XCTAssertEqual(bindings.chord(for: .confirmHighlighted), try chord("\r"))
        XCTAssertNil(bindings.chord(for: .pageForward))
    }

    /// The regression this resolver was rewritten for: filling one always-bound
    /// row by taking a chord back must not empty the other one. Every
    /// arrangement of the two rows over their two chords, plus every way a
    /// third row can be holding one of them.
    func testTheAlwaysBoundActions_areNeverBothLeftUnbound() throws {
        let candidates: [ComposingKeyChord?] = try [nil, chord("\r"), chord("\r", .shift), chord("]")]
        let others = ComposingAction.allCases.filter { !ComposingAction.alwaysBound.contains($0) }

        for confirm in candidates {
            for literal in candidates {
                for other in others {
                    for otherChord in candidates {
                        let bindings = ComposingKeyBindings(chords: [
                            .confirmHighlighted: confirm,
                            .commitLiteral: literal,
                            other: otherChord,
                        ])

                        for action in ComposingAction.alwaysBound {
                            XCTAssertNotNil(
                                bindings.chord(for: action),
                                """
                                \(action) left unbound by confirm=\(String(describing: confirm)) \
                                literal=\(String(describing: literal)) \
                                \(other)=\(String(describing: otherChord))
                                """,
                            )
                        }
                        XCTAssertNotEqual(
                            bindings.chord(for: .confirmHighlighted),
                            bindings.chord(for: .commitLiteral),
                            "the two commits cannot share one key",
                        )
                    }
                }
            }
        }
    }

    /// Swapping the pair is a state a user can ask for, and both rows stay full
    /// so nothing is restored over it.
    func testTheAlwaysBoundActions_maySwapTheirChords() throws {
        let bindings = ComposingKeyBindings(chords: [
            .commitLiteral: try chord("\r"),
            .confirmHighlighted: try chord("\r", .shift),
        ])

        XCTAssertEqual(bindings.chord(for: .commitLiteral), try chord("\r"))
        XCTAssertEqual(bindings.chord(for: .confirmHighlighted), try chord("\r", .shift))
    }

    /// Their two chords belong to them: an ordinary action holding one gives it
    /// up, so the keys that end a composition stay where a user can find them.
    func testAnOrdinaryAction_cannotHoldAnAlwaysBoundChord() throws {
        let bindings = ComposingKeyBindings(chords: [.nextCandidate: try chord("\r", .shift)])

        XCTAssertNil(bindings.chord(for: .nextCandidate))
        XCTAssertEqual(bindings.chord(for: .commitLiteral), try chord("\r", .shift))
    }

    // MARK: - The candidate-slot tier

    /// The slot chords are classified before any binding, so a row holding one
    /// would be recorded and then never fire. Dropped here as well as refused
    /// by the recorder, because the modifier can be changed afterwards.
    func testAChordTheSlotTierWouldSwallow_isDropped() throws {
        let control = ComposingKeyBindings(
            chords: [.pageForward: try chord("3", .control)],
            slotModifier: .control,
        )
        XCTAssertNil(control.chord(for: .pageForward))

        let option = ComposingKeyBindings(
            chords: [.pageForward: try chord("3", .control)],
            slotModifier: .option,
        )
        XCTAssertEqual(
            option.chord(for: .pageForward),
            try chord("3", .control),
            "⌃3 is an ordinary chord once Option holds the slots",
        )
    }

    func testIsCandidateSlotChord_namesOnlyTheNineDigitsUnderTheBoundModifier() throws {
        XCTAssertTrue(try chord("3", .control).isCandidateSlotChord(under: .control))
        XCTAssertFalse(try chord("3", .control).isCandidateSlotChord(under: .option))
        XCTAssertFalse(
            try chord("0", .control).isCandidateSlotChord(under: .control),
            "⌃0 addresses no slot",
        )
        XCTAssertFalse(
            try chord("3", [.control, .shift]).isCandidateSlotChord(under: .control),
            "an extra modifier makes it a different chord",
        )
        XCTAssertFalse(try chord("]", .control).isCandidateSlotChord(under: .control))
    }

    // MARK: - Keypad Enter

    /// A keyboard has two Enter keys and a user binding one means both, so the
    /// keypad's folds into Return rather than being a chord of its own.
    func testKeypadEnter_isTheSameChordAsReturn() throws {
        let keypad = try chord("\u{3}")

        XCTAssertEqual(keypad, try chord("\r"))
        XCTAssertTrue(keypad.matches(try snapshot("\r")))
        XCTAssertTrue(try chord("\r").matches(try snapshot("\u{3}")))
        XCTAssertEqual(
            ComposingKeyBindings.default.action(for: try snapshot("\u{3}")),
            .confirmHighlighted,
            "the keypad commits out of the box, as it always has",
        )
    }

    // MARK: - Back tab

    /// AppKit reports ⇧⇥ as `NSBackTabCharacter` rather than as Tab with Shift
    /// held. It folds onto Tab for the same reason the keypad's Enter folds
    /// onto Return: ⇧⇥ is Tab-with-Shift to the user, and every site that
    /// prints or stores a chord would otherwise have to know the scalar.
    func testBackTab_isTabWithShift() throws {
        let backTab = try chord("\u{19}", .shift)

        XCTAssertEqual(backTab, try chord("\t", .shift))
        XCTAssertTrue(backTab.matches(try snapshot("\u{19}", modifiers: .shift)))
        XCTAssertEqual(
            ComposingKeyBindings.default.action(for: try snapshot("\u{19}", modifiers: .shift)),
            .previousCandidate,
            "⇧⇥ walks back through the candidates out of the box",
        )
        XCTAssertEqual(ShortcutKeyDisplay.text(for: backTab), "⇧⇥")
    }

    /// A chord a build before the fold stored keeps working: `init?(rawValue:)`
    /// goes back through `make`, which normalizes.
    func testAStoredBackTab_readsBackAsTabWithShift() throws {
        XCTAssertEqual(ComposingKeyChord(rawValue: "s|0019"), try chord("\t", .shift))
    }

    func testActionsHolding_namesTheRowsARecordingWouldEmpty() throws {
        let bindings = ComposingKeyBindings(chords: [.pageForward: try chord("]")])

        XCTAssertEqual(
            bindings.actionsHolding(try chord("]"), excluding: .nextCandidate),
            [.pageForward],
        )
        XCTAssertEqual(
            bindings.actionsHolding(try chord("]"), excluding: .pageForward),
            [],
            "the row being recorded is not its own conflict",
        )
    }

    // MARK: - Matching

    func testAChordMatches_onlyItsOwnKeyAndModifiers() throws {
        let optionReturn = try chord("\r", .option)

        XCTAssertTrue(optionReturn.matches(try snapshot("\r", modifiers: .option)))
        XCTAssertFalse(optionReturn.matches(try snapshot("\r")), "no modifier held")
        XCTAssertFalse(
            optionReturn.matches(try snapshot("\r", modifiers: [.option, .shift])),
            "an extra chording modifier is a different chord",
        )
    }

    /// Option rewrites most of the keyboard and Control rewrites the digits, so
    /// matching reads the unmodified characters — the same field the candidate
    /// slot chords are read from.
    func testAChordMatches_throughTheCharactersAModifierRewrote() throws {
        let optionJ = try chord("j", .option)

        XCTAssertTrue(
            optionJ.matches(try snapshot("∆", modifiers: .option, unmodified: "j")),
            "⌥J arrives as ∆",
        )
    }

    func testMatching_ignoresTheNonChordingModifiers() throws {
        let space = try chord(" ")

        XCTAssertTrue(space.matches(try snapshot(" ", modifiers: [.capsLock, .numericPad])))
    }

    // MARK: - A bare letter through the intent tiers

    /// The design promise behind freeing the eight letters: mid-composition
    /// the bindings tier is read before input, so the bound letter fires its
    /// action; everywhere the binding does not apply, the letter is still the
    /// letter.
    func testABareBoundLetter_firesItsAction_onlyWhereTheActionApplies() throws {
        let bindings = ComposingKeyBindings(chords: [.pageForward: try chord("z")])
        let z = try snapshot("z")

        XCTAssertEqual(
            ComposingKeyIntent.intent(for: z, isComposing: true, isShowingCandidates: true, bindings: bindings),
            .navigate(.pageDown),
            "the binding wins over literal input while candidates are up",
        )
        XCTAssertEqual(
            ComposingKeyIntent.intent(for: z, isComposing: true, bindings: bindings),
            .input("z"),
            "an action that needs candidates gives the key back when none are up",
        )
        XCTAssertEqual(
            ComposingKeyIntent.intent(for: z, isComposing: false, bindings: bindings),
            .input("z"),
            "with no composition there is no page to turn — the letter still starts one",
        )
    }

    private func chord(_ key: String, _ modifiers: NSEvent.ModifierFlags = []) throws -> ComposingKeyChord {
        try ComposingKeyChord.make(key: key, modifiers: modifiers).get()
    }

    private func snapshot(
        _ characters: String,
        modifiers: NSEvent.ModifierFlags = [],
        unmodified: String? = nil,
    ) throws -> KeyEventSnapshot {
        try KeyEventSnapshot(TestFixtures.keyDownEvent(
            characters: characters,
            modifiers: modifiers,
            charactersIgnoringModifiers: unmodified,
        ))
    }
}
