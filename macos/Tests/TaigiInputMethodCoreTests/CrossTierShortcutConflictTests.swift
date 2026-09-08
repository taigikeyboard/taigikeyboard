// The seam between the two shortcut registries: a chord may belong to the
// global tier or to the composing tier, never to both.

import KeyboardShortcuts
@testable import TaigiInputMethodCore
import XCTest

/// Global shortcuts reach the app through Carbon, composing keys through our
/// own classifier — so a chord on both means the composing one never fires.
/// These cases pin the bridge between the two representations, the queries
/// either side asks, and the launch pass that reconciles data no recorder saw.
@MainActor
final class CrossTierShortcutConflictTests: XCTestCase {
    /// Records a global shortcut and takes it back off at teardown: the
    /// library's registry is process-wide, so a case that left one behind
    /// would reach every other suite in the target.
    private func recordGlobal(_ shortcut: KeyboardShortcuts.Shortcut, for name: KeyboardShortcuts.Name) {
        KeyboardShortcuts.setShortcut(shortcut, for: name)
        addTeardownBlock { KeyboardShortcuts.reset(name) }
    }

    private func chord(_ key: String, _ modifiers: NSEvent.ModifierFlags = []) throws -> ComposingKeyChord {
        try ComposingKeyChord.make(key: key, modifiers: modifiers).get()
    }

    /// The keys `ComposingKeyChord.neverBindable` refuses whatever modifiers
    /// are held, so the bridge must answer nil for them.
    private static let reservedKeys: Set<KeyboardShortcuts.Key> = [
        .escape, .delete, .deleteForward, .pageUp, .pageDown,
        .leftArrow, .rightArrow, .upArrow, .downArrow,
    ]

    /// The keys the library gives a name and a glyph to, plus the number pad —
    /// the two families `namedKeyCharacters` exists for.
    private static let allNamedKeys: [KeyboardShortcuts.Key] = [
        .return, .tab, .space, .escape, .delete, .deleteForward,
        .home, .end, .help, .pageUp, .pageDown,
        .leftArrow, .rightArrow, .upArrow, .downArrow,
        .keypad0, .keypad1, .keypad2, .keypad3, .keypad4, .keypad5,
        .keypad6, .keypad7, .keypad8, .keypad9,
        .keypadClear, .keypadDecimal, .keypadDivide, .keypadEnter,
        .keypadEquals, .keypadMinus, .keypadMultiply, .keypadPlus,
    ]

    // MARK: - The bridge

    func testAChordedLetterShortcut_readsAsTheComposingChordOnTheSameKey() throws {
        let chord = try XCTUnwrap(ShortcutConflicts.composingChord(
            occupiedBy: KeyboardShortcuts.Shortcut(.r, modifiers: [.control, .command]),
        ))

        XCTAssertEqual(chord.key, "r", "the key is the character it types unmodified")
        XCTAssertEqual(chord.modifiers, [.control, .command])
    }

    func testTheNamedKeys_bridgeToTheCharactersTheComposingTierStores() throws {
        // Return and Tab are where the two registries are most likely to meet:
        // the composing tier keeps its commit keys in the Return family.
        let returnChord = try XCTUnwrap(ShortcutConflicts.composingChord(
            occupiedBy: KeyboardShortcuts.Shortcut(.return, modifiers: [.shift]),
        ))
        XCTAssertEqual(
            returnChord,
            ComposingAction.commitLiteral.defaultChord,
            "⇧↩ commits what was typed",
        )

        let tabChord = try XCTUnwrap(ShortcutConflicts.composingChord(
            occupiedBy: KeyboardShortcuts.Shortcut(.tab, modifiers: [.shift]),
        ))
        XCTAssertEqual(tabChord, ComposingAction.previousCandidate.defaultChord, "⇧⇥ walks back")
    }

    func testTheBacktick_bridges() throws {
        // The shipped 漢羅對調 default, and a chord the composing recorder
        // accepts — the collision that needs no exotic user.
        let chord = try XCTUnwrap(ShortcutConflicts.composingChord(
            occupiedBy: KeyboardShortcuts.Shortcut(.backtick),
        ))

        XCTAssertEqual(chord.key, "`")
        XCTAssertEqual(chord.modifiers, [])
    }

    func testAKeypadDigit_bridgesToTheSameChordAsTheTopRow() throws {
        // Neither tier keeps the `.numericPad` flag, so the keypad's 3 and the
        // top row's 3 are one chord.
        let chord = try XCTUnwrap(ShortcutConflicts.composingChord(
            occupiedBy: KeyboardShortcuts.Shortcut(.keypad3, modifiers: [.control]),
        ))

        XCTAssertEqual(chord.key, "3")
        XCTAssertEqual(chord.modifiers, .control)
    }

    func testTheKeypadOperators_bridgeToWhatTheyType() throws {
        // The library reports the whole number pad as nil, because no SwiftUI
        // key equivalent addresses it — but these keys still type a character.
        let cases: [(KeyboardShortcuts.Key, String)] = [
            (.keypadDecimal, "."), (.keypadDivide, "/"), (.keypadMinus, "-"),
            (.keypadMultiply, "*"), (.keypadPlus, "+"), (.keypadEquals, "="),
            (.keypadEnter, "\r"),
        ]

        for (key, character) in cases {
            let chord = ShortcutConflicts.composingChord(
                occupiedBy: KeyboardShortcuts.Shortcut(key, modifiers: [.control]),
            )
            XCTAssertEqual(chord?.key, character, "keypad \(character)")
        }
    }

    func testHomeEndAndHelp_bridgeToTheScalarsAppKitNamesThemWith() throws {
        let cases: [(KeyboardShortcuts.Key, Int)] = [
            (.home, NSHomeFunctionKey), (.end, NSEndFunctionKey), (.help, NSHelpFunctionKey),
        ]

        for (key, functionKey) in cases {
            let chord = ShortcutConflicts.composingChord(
                occupiedBy: KeyboardShortcuts.Shortcut(key, modifiers: [.control]),
            )
            XCTAssertEqual(
                chord?.key, String(try XCTUnwrap(UnicodeScalar(functionKey))),
                "the library reports a display glyph for this key",
            )
        }
    }

    func testEverySpecialKeyTheLibraryNames_bridgesToWhatItTypes_orToNothing() throws {
        // The guard against the next key the library glyphs. Exactly two
        // answers are right, and which one depends on whether a binding could
        // hold the key: the reserved ones must bridge to nothing (the gate
        // recognised the scalar and refused it), everything else to the
        // character it types. A DISPLAY GLYPH is always wrong — it would
        // compare as a chord of its own and so never collide with anything.
        let glyphs = Set("↩⌫⌦↘⎋↖↗⇥⇞⇟↑→↓←")

        for key in Self.allNamedKeys {
            let chord = ShortcutConflicts.composingChord(
                occupiedBy: .init(key, modifiers: [.control]),
            )
            guard !Self.reservedKeys.contains(key) else {
                XCTAssertNil(chord, "a key no binding can hold must bridge to no conflict")
                continue
            }
            let scalar = try XCTUnwrap(chord?.key.unicodeScalars.first, "no bridge for a bindable key")
            XCTAssertFalse(
                glyphs.contains(Character(scalar)),
                "bridged to a display glyph rather than what the key types",
            )
        }
    }

    func testTheBridgeDropsTheModifiersAChordIsNotMadeOf() throws {
        let chord = try XCTUnwrap(ShortcutConflicts.composingChord(
            occupiedBy: KeyboardShortcuts.Shortcut(.r, modifiers: [.control, .capsLock, .function]),
        ))

        XCTAssertEqual(chord.modifiers, [.control], "Caps Lock and the function flag are not chords")
    }

    func testAKeyNoBindingCanHold_bridgesToNoConflict() throws {
        // The invariant the bridge rests on: every chord a composing action
        // can hold went through `make`, so a chord `make` refuses is one no
        // binding holds — and answering nil is answering "nothing to clear".
        XCTAssertNil(
            ShortcutConflicts.composingChord(occupiedBy: .init(.leftArrow, modifiers: [.command])),
            "an arrow is reserved whatever modifiers are held",
        )
        XCTAssertEqual(
            ComposingKeyChord.make(key: "a", modifiers: []),
            .failure(.typesRomanization),
            "and the gate that decides this is the recorder's own",
        )
    }

    // MARK: - The queries, both directions

    func testAGlobalShortcut_isSeenByTheComposingRowHoldingItsChord() {
        let holders = ShortcutConflicts.composingActionsHolding(
            KeyboardShortcuts.Shortcut(.rightBracket),
            in: .default,
        )

        XCTAssertEqual(holders, [.pageForward])
    }

    func testAComposingChordNoGlobalShortcutHolds_findsNothing() {
        let holders = ShortcutConflicts.composingActionsHolding(
            KeyboardShortcuts.Shortcut(.f, modifiers: [.control, .command]),
            in: .default,
        )

        XCTAssertEqual(holders, [])
    }

    func testAComposingChord_isSeenByTheGlobalRowHoldingIt() throws {
        recordGlobal(.init(.r, modifiers: [.control, .command]), for: .toggleRomanization)

        let holders = ShortcutConflicts.globalActionsHolding(try chord("r", [.control, .command]))

        XCTAssertEqual(holders, [.toggleRomanization])
    }

    /// A slot key — a bare letter under Standard, a bare digit under Telex —
    /// is a typing key to the gate, so the bridge answers nil for a global
    /// row holding one: no composing chord can exist for it to collide with,
    /// and the recorder refuses to put one on a global row in the first
    /// place. A shifted digit bridges to the digit with Shift — the library
    /// names the key by what it types unmodified — which the gate refuses
    /// the same way.
    func testAGlobalShortcutOnASlotKey_bridgesToNoChord() {
        XCTAssertNil(ShortcutConflicts.composingChord(occupiedBy: KeyboardShortcuts.Shortcut(.q)))
        XCTAssertNil(ShortcutConflicts.composingChord(occupiedBy: KeyboardShortcuts.Shortcut(.three)))
        XCTAssertNil(ShortcutConflicts.composingChord(
            occupiedBy: KeyboardShortcuts.Shortcut(.three, modifiers: [.shift]),
        ))
    }

    // MARK: - Shipped defaults never collide

    func testEveryShippedDefault_holdsAChordNoOtherTierShipsWith() throws {
        let composingDefaults = Set(ComposingAction.allCases.map(\.defaultChord))

        for action in ShortcutAction.allCases {
            let shortcut = try XCTUnwrap(action.defaultShortcut, "\(action) ships unbound")
            let chord = try XCTUnwrap(
                ShortcutConflicts.composingChord(occupiedBy: shortcut),
                "\(action)'s default cannot be compared across the seam",
            )
            // The slot half of this question is already pinned by
            // `ShortcutActionsTests.testNoDefault_isASlotKey`.
            XCTAssertFalse(
                composingDefaults.contains(chord),
                "\(action)'s default collides with a composing default",
            )
        }
    }

    // MARK: - The launch pass

    func testALaunchPass_clearsTheGlobalDefaultShadowedByAComposingRecording() throws {
        let store = try makeScratchSettingsStore()
        // The upgrade case: a version gives a global action a default chord the
        // user had already recorded on a composing row. The recording wins.
        let chord = try chord("c", [.control, .command])
        store.setComposingChord(chord, for: .pageForward)
        recordGlobal(.init(.c, modifiers: [.control, .command]), for: .toggleRomanization)

        ShortcutConflicts.resolveAcrossRegistries(in: store)

        XCTAssertNil(KeyboardShortcuts.getShortcut(for: .toggleRomanization), "the default gives way")
        XCTAssertEqual(store.composingKeyBindings.chord(for: .pageForward), chord, "the recording stays")
    }

    func testALaunchPass_clearsTheComposingDefaultShadowedByAGlobalRecording() throws {
        let store = try makeScratchSettingsStore()
        // The mirror image: the composing row is on the chord it shipped with,
        // and the user put that chord on a global action by hand.
        recordGlobal(.init(.tab, modifiers: [.shift]), for: .toggleRomanization)

        ShortcutConflicts.resolveAcrossRegistries(in: store)

        XCTAssertNil(store.composingKeyBindings.chord(for: .previousCandidate), "the default gives way")
        XCTAssertEqual(
            KeyboardShortcuts.getShortcut(for: .toggleRomanization),
            .init(.tab, modifiers: [.shift]),
            "the recording stays",
        )
    }

    func testALaunchPass_breaksARecordingAgainstRecordingTieForTheTierThatFires() throws {
        let store = try makeScratchSettingsStore()
        // Neither stored value carries a timestamp, so the last writer is
        // unknowable. The global tier wins because it is the one that actually
        // dispatches — keeping the composing row would keep a dead key.
        let chord = try TestFixtures.chordNoDefaultHolds(key: "f")
        store.setComposingChord(chord, for: .pageForward)
        recordGlobal(.init(.f, modifiers: chord.modifiers), for: .toggleRomanization)

        ShortcutConflicts.resolveAcrossRegistries(in: store)

        XCTAssertEqual(
            KeyboardShortcuts.getShortcut(for: .toggleRomanization),
            .init(.f, modifiers: chord.modifiers),
        )
        XCTAssertNil(store.composingKeyBindings.chord(for: .pageForward))
    }

    func testALaunchPass_leavesAnUncollidingSetupAlone() throws {
        let store = try makeScratchSettingsStore()
        ShortcutConflicts.resolveAcrossRegistries(in: store)

        XCTAssertEqual(
            KeyboardShortcuts.getShortcut(for: .toggleRomanization),
            ShortcutAction.toggleRomanization.defaultShortcut,
        )
        for action in ComposingAction.allCases {
            XCTAssertEqual(
                store.composingKeyBindings.chord(for: action), action.defaultChord,
                "\(action) lost its default",
            )
        }
    }

    /// A global row recorded on a bare `z` while the eight non-syllable
    /// letters were bindable (before 2026-09-08) would fire through Carbon
    /// before the classifier saw the Telex key — and a `⇧3` before it saw the
    /// slot key. The bridge refuses both as typing keys now, and a refusal is
    /// not "no conflict": the launch pass clears the row, once.
    func testALaunchPass_clearsAGlobalRowLeftOnATypingKey() throws {
        let store = try makeScratchSettingsStore()
        recordGlobal(.init(.z), for: .toggleRomanization)
        recordGlobal(.init(.three, modifiers: [.shift]), for: .toggleTranslateSwapped)
        recordGlobal(.init(.z, modifiers: [.control]), for: .openLastSettingsPane)

        ShortcutConflicts.resolveAcrossRegistries(in: store)

        XCTAssertNil(KeyboardShortcuts.getShortcut(for: .toggleRomanization))
        XCTAssertNil(KeyboardShortcuts.getShortcut(for: .toggleTranslateSwapped))
        XCTAssertEqual(KeyboardShortcuts.getShortcut(for: .openLastSettingsPane), .init(.z, modifiers: [.control]))
        ShortcutConflicts.resolveAcrossRegistries(in: store)
        XCTAssertEqual(KeyboardShortcuts.getShortcut(for: .openLastSettingsPane), .init(.z, modifiers: [.control]))
    }

    func testALaunchPass_isIdempotent() throws {
        let store = try makeScratchSettingsStore()
        // The pass reads the bindings once and then writes through them, so a
        // second launch must find nothing left to do — otherwise the stale
        // snapshot would be eating a row per launch.
        let chord = try TestFixtures.chordNoDefaultHolds(key: "f")
        store.setComposingChord(chord, for: .pageForward)
        recordGlobal(.init(.f, modifiers: chord.modifiers), for: .toggleRomanization)
        ShortcutConflicts.resolveAcrossRegistries(in: store)
        let afterFirst = ComposingAction.allCases.map { store.composingKeyBindings.chord(for: $0) }

        ShortcutConflicts.resolveAcrossRegistries(in: store)

        XCTAssertEqual(
            ComposingAction.allCases.map { store.composingKeyBindings.chord(for: $0) }, afterFirst,
            "a second launch changed the composing rows again",
        )
        XCTAssertEqual(
            KeyboardShortcuts.getShortcut(for: .toggleRomanization),
            .init(.f, modifiers: chord.modifiers),
        )
    }

    func testALaunchPass_resolvesTwoCollisionsAtOnce() throws {
        let store = try makeScratchSettingsStore()
        // Two global rows against two different composing rows: the snapshot is
        // read once, so this is where a stale read would drop the second clear.
        recordGlobal(.init(.tab, modifiers: [.shift]), for: .toggleRomanization)
        recordGlobal(.init(.rightBracket), for: .openLastSettingsPane)

        ShortcutConflicts.resolveAcrossRegistries(in: store)

        XCTAssertNil(store.composingKeyBindings.chord(for: .previousCandidate), "⇧⇥ row not cleared")
        XCTAssertNil(store.composingKeyBindings.chord(for: .pageForward), "] row not cleared")
    }

    // MARK: - Always-bound rows survive being cleared

    func testClearingAnAlwaysBoundRow_restoresItWithoutResurrectingTheConflict() throws {
        let store = try makeScratchSettingsStore()
        // ⇧Return is `commitLiteral`'s default AND a member of the pool
        // `restoreUnbound` refills from, so this is the case where a naive
        // clear could hand the chord straight back. It cannot happen from the
        // recorder — the library refuses shift-only chords
        // (`RecorderCocoa.swift:403-408`) — so the collision is modelled here
        // on a chord a global shortcut CAN hold.
        let chord = try chord("r", [.control, .command])
        store.setComposingChord(chord, for: .commitLiteral)
        recordGlobal(.init(.r, modifiers: [.control, .command]), for: .openLastSettingsPane)

        ShortcutConflicts.resolveAcrossRegistries(in: store)

        let restored = store.composingKeyBindings.chord(for: .commitLiteral)
        XCTAssertNotNil(restored, "an always-bound row must not be left empty")
        XCTAssertNotEqual(restored, chord, "and must not be handed the conflicting chord back")
        XCTAssertNotEqual(
            restored, store.composingKeyBindings.chord(for: .confirmHighlighted),
            "the two always-bound rows still hold different keys",
        )
    }
}
