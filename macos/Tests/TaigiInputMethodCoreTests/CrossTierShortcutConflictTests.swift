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
        try TestFixtures.chord(key, modifiers)
    }

    /// The composing rows `store` holds, as the core resolves them.
    private func shortcuts(in store: SettingsStore) throws -> ComposingShortcuts {
        try TestFixtures.composingShortcuts(in: store)
    }

    /// The chord on the row named `name` in `store`.
    private func chord(onRow name: String, in store: SettingsStore) throws -> ComposingKeyChord? {
        try TestFixtures.row(name, in: shortcuts(in: store)).first.chord
    }

    /// Records `chord` on the row named `name` in `store`.
    private func record(_ chord: ComposingKeyChord?, onRow name: String, in store: SettingsStore) throws {
        try store.setComposingChord(chord, for: TestFixtures.row(name, in: shortcuts(in: store)).first)
    }

    /// The scalars of a chord's key, read off its stored form.
    private func keyScalars(_ chord: ComposingKeyChord?) throws -> [UnicodeScalar] {
        let fields = try XCTUnwrap(chord?.rawValue.split(separator: "|", omittingEmptySubsequences: false).last)
        return try fields.split(separator: ",").map { try XCTUnwrap(UInt32($0, radix: 16).flatMap(UnicodeScalar.init)) }
    }

    /// The keys the core's Mac grammar refuses whatever modifiers are held,
    /// so the bridge must answer nil for them.
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

        XCTAssertEqual(chord.rawValue, "dc|0072", "the key is the character it types unmodified")
    }

    func testTheNamedKeys_bridgeToTheCharactersTheComposingTierStores() throws {
        // Return and Tab are where the two registries are most likely to meet:
        // the composing tier keeps its commit keys in the Return family.
        let returnChord = try XCTUnwrap(ShortcutConflicts.composingChord(
            occupiedBy: KeyboardShortcuts.Shortcut(.return, modifiers: [.shift]),
        ))
        let rows = try shortcuts(in: makeScratchSettingsStore())
        XCTAssertEqual(
            returnChord,
            try TestFixtures.row("commitLiteral", in: rows).first.shippedChord,
            "⇧↩ commits what was typed",
        )

        let tabChord = try XCTUnwrap(ShortcutConflicts.composingChord(
            occupiedBy: KeyboardShortcuts.Shortcut(.tab, modifiers: [.shift]),
        ))
        XCTAssertEqual(tabChord, try TestFixtures.row("previousCandidate", in: rows).first.shippedChord, "⇧⇥ walks back")
    }

    func testTheBacktick_bridges() throws {
        // The shipped Hanji/romanization swap default, and a chord the composing recorder
        // accepts — the collision that needs no exotic user.
        let chord = try XCTUnwrap(ShortcutConflicts.composingChord(
            occupiedBy: KeyboardShortcuts.Shortcut(.backtick),
        ))

        XCTAssertEqual(chord.rawValue, "|0060")
    }

    func testAKeypadDigit_bridgesToTheSameChordAsTheTopRow() throws {
        // Neither tier keeps the `.numericPad` flag, so the keypad's 3 and the
        // top row's 3 are one chord.
        let chord = try XCTUnwrap(ShortcutConflicts.composingChord(
            occupiedBy: KeyboardShortcuts.Shortcut(.keypad3, modifiers: [.control]),
        ))

        XCTAssertEqual(chord.rawValue, "c|0033")
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
            XCTAssertEqual(try keyScalars(chord), Array(character.unicodeScalars), "keypad \(character)")
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
                try keyScalars(chord), try [XCTUnwrap(UnicodeScalar(functionKey))],
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
            XCTAssertNotNil(chord, "no bridge for a bindable key")
            let scalar = try XCTUnwrap(keyScalars(chord).first)
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

        XCTAssertEqual(chord.rawValue, "c|0072", "Caps Lock and the function flag are not chords")
    }

    func testAKeyNoBindingCanHold_bridgesToNoConflict() throws {
        // The invariant the bridge rests on: every chord a composing action
        // can hold went through the core's gate, so a chord it refuses is one
        // no binding holds — and answering nil is answering "nothing to clear".
        XCTAssertNil(
            ShortcutConflicts.composingChord(occupiedBy: .init(.leftArrow, modifiers: [.command])),
            "an arrow is reserved whatever modifiers are held",
        )
        XCTAssertEqual(
            try XCTUnwrap(KeyRules.chord(key: "a", modifiers: [])),
            .failure(.typesRomanization),
            "and the gate that decides this is the recorder's own",
        )
    }

    // MARK: - The queries, both directions

    func testAGlobalShortcut_isSeenByTheComposingRowHoldingItsChord() throws {
        let holders = try ShortcutConflicts.composingFieldsHolding(
            KeyboardShortcuts.Shortcut(.rightBracket),
            in: shortcuts(in: makeScratchSettingsStore()),
        )

        XCTAssertEqual(holders.map(\.settingsKey), ["composingShortcut.pageForward"])
    }

    func testAComposingChordNoGlobalShortcutHolds_findsNothing() throws {
        let holders = try ShortcutConflicts.composingFieldsHolding(
            KeyboardShortcuts.Shortcut(.f, modifiers: [.control, .command]),
            in: shortcuts(in: makeScratchSettingsStore()),
        )

        XCTAssertEqual(holders, [])
    }

    func testAComposingChord_isSeenByTheGlobalRowHoldingIt() throws {
        recordGlobal(.init(.r, modifiers: [.control, .command]), for: .toggleRomanization)

        let holders = try ShortcutConflicts.globalActionsHolding(chord("r", [.control, .command]))

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
        let composingDefaults = try Set(shortcuts(in: makeScratchSettingsStore()).rows.flatMap(\.fields).compactMap(\.shippedChord))

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
        try record(chord, onRow: "pageForward", in: store)
        recordGlobal(.init(.c, modifiers: [.control, .command]), for: .toggleRomanization)

        ShortcutConflicts.resolveAcrossRegistries(in: store)

        XCTAssertNil(KeyboardShortcuts.getShortcut(for: .toggleRomanization), "the default gives way")
        XCTAssertEqual(try self.chord(onRow: "pageForward", in: store), chord, "the recording stays")
    }

    func testALaunchPass_clearsTheComposingDefaultShadowedByAGlobalRecording() throws {
        let store = try makeScratchSettingsStore()
        // The mirror image: the composing row is on the chord it shipped with,
        // and the user put that chord on a global action by hand.
        recordGlobal(.init(.tab, modifiers: [.shift]), for: .toggleRomanization)

        ShortcutConflicts.resolveAcrossRegistries(in: store)

        XCTAssertNil(try chord(onRow: "previousCandidate", in: store), "the default gives way")
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
        let modifiers = try TestFixtures.modifiersNoDefaultHolds(key: "f")
        try record(chord("f", modifiers), onRow: "pageForward", in: store)
        recordGlobal(.init(.f, modifiers: modifiers), for: .toggleRomanization)

        ShortcutConflicts.resolveAcrossRegistries(in: store)

        XCTAssertEqual(
            KeyboardShortcuts.getShortcut(for: .toggleRomanization),
            .init(.f, modifiers: modifiers),
        )
        XCTAssertNil(try chord(onRow: "pageForward", in: store))
    }

    func testALaunchPass_leavesAnUncollidingSetupAlone() throws {
        let store = try makeScratchSettingsStore()
        ShortcutConflicts.resolveAcrossRegistries(in: store)

        XCTAssertEqual(
            KeyboardShortcuts.getShortcut(for: .toggleRomanization),
            ShortcutAction.toggleRomanization.defaultShortcut,
        )
        let rows = try shortcuts(in: store).rows
        XCTAssertEqual(rows.count, 7)
        for row in rows {
            for field in row.fields {
                XCTAssertEqual(field.chord, field.shippedChord, "\(field.settingsKey) lost its default")
            }
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
        let modifiers = try TestFixtures.modifiersNoDefaultHolds(key: "f")
        try record(chord("f", modifiers), onRow: "pageForward", in: store)
        recordGlobal(.init(.f, modifiers: modifiers), for: .toggleRomanization)
        ShortcutConflicts.resolveAcrossRegistries(in: store)
        let afterFirst = try shortcuts(in: store).rows

        ShortcutConflicts.resolveAcrossRegistries(in: store)

        XCTAssertEqual(
            try shortcuts(in: store).rows, afterFirst,
            "a second launch changed the composing rows again",
        )
        XCTAssertEqual(
            KeyboardShortcuts.getShortcut(for: .toggleRomanization),
            .init(.f, modifiers: modifiers),
        )
    }

    func testALaunchPass_resolvesTwoCollisionsAtOnce() throws {
        let store = try makeScratchSettingsStore()
        // Two global rows against two different composing rows: the snapshot is
        // read once, so this is where a stale read would drop the second clear.
        recordGlobal(.init(.tab, modifiers: [.shift]), for: .toggleRomanization)
        recordGlobal(.init(.rightBracket), for: .openLastSettingsPane)

        ShortcutConflicts.resolveAcrossRegistries(in: store)

        XCTAssertNil(try chord(onRow: "previousCandidate", in: store), "⇧⇥ row not cleared")
        XCTAssertNil(try chord(onRow: "pageForward", in: store), "] row not cleared")
    }

    // MARK: - Commit rows refill after being cleared

    func testClearingACommitRow_refillsItWithoutResurrectingTheConflict() throws {
        let store = try makeScratchSettingsStore()
        // ⇧Return is `commitLiteral`'s default AND a member of the pool
        // `restoreUnbound` refills from, so this is the case where a naive
        // clear could hand the chord straight back. It cannot happen from the
        // recorder — the library refuses shift-only chords
        // (`RecorderCocoa.swift:403-408`) — so the collision is modelled here
        // on a chord a global shortcut CAN hold.
        let chord = try chord("r", [.control, .command])
        try record(chord, onRow: "commitLiteral", in: store)
        recordGlobal(.init(.r, modifiers: [.control, .command]), for: .openLastSettingsPane)

        ShortcutConflicts.resolveAcrossRegistries(in: store)

        let restored = try self.chord(onRow: "commitLiteral", in: store)
        XCTAssertNotNil(restored, "a commit row is refilled when its default is free")
        XCTAssertNotEqual(restored, chord, "and must not be handed the conflicting chord back")
        XCTAssertNotEqual(
            restored, try self.chord(onRow: "confirmHighlighted", in: store),
            "the two commit rows still hold different keys",
        )
    }
}
