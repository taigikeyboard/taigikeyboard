// Full-width punctuation end to end: mapped in the swapped mode at both
// insertion sites, untouched everywhere else.

import InputMethodKit
@testable import TaigiInputMethodCore
import XCTest

/// Drives the real controller and engine with the Hanji/romanization swap ON, where the
/// shipped full-width default takes effect. The roman-first sites are covered
/// by the negative cases here and by `AutoSpaceControllerTests`, whose suite
/// runs entirely in the mode this feature is inert in.
@MainActor
final class FullWidthPunctuationControllerTests: XCTestCase {
    override func setUp() {
        super.setUp()
        InstalledLexicon.installOnce()
    }

    // MARK: - Outside a composition (the consumed pass-through)

    func testPunctuationOutsideAComposition_insertsTheFullWidthForm() throws {
        let session = try makeSession(configure: { $0.storedIsTranslateSwapped = true })

        let handled = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: ","), client: session.client,
        )

        XCTAssertTrue(handled, "the mapped key is consumed, not passed to the host")
        XCTAssertEqual(session.client.insertedTexts, ["，"])
    }

    func testAHostChord_isNeverMapped() throws {
        // ⌘. is a host command that inserts nothing; consuming it would eat
        // the shortcut.
        let session = try makeSession(configure: { $0.storedIsTranslateSwapped = true })

        let handled = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: ".", modifiers: .command), client: session.client,
        )

        XCTAssertFalse(handled)
        XCTAssertEqual(session.client.insertedTexts, [])
    }

    func testRomanFirstMode_passesPunctuationThrough() throws {
        // Opted into: the shipped default is hanji-first (2026-09-18).
        let session = try makeSession(configure: { $0.storedIsTranslateSwapped = false })

        let handled = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: ","), client: session.client,
        )

        XCTAssertFalse(handled, "roman-first output keeps the host's half-width comma")
        XCTAssertEqual(session.client.insertedTexts, [])
    }

    /// Hanji with Romanization forces the candidate projection hanji-first, but the punctuation
    /// width still follows the stored swap the chord toggles: half-width
    /// until it is on, full-width after.
    func testCombinedDisplay_punctuationWidthFollowsTheStoredSwap() throws {
        let halfWidth = try makeSession(configure: {
            $0.candidateDisplayMode = .combined
            $0.storedIsTranslateSwapped = false
        })
        XCTAssertFalse(try halfWidth.controller.handle(
            TestFixtures.keyDownEvent(characters: ","), client: halfWidth.client,
        ), "stored swap off under 合用 keeps the host's half-width comma")
        XCTAssertEqual(halfWidth.client.insertedTexts, [])

        let fullWidth = try makeSession(configure: {
            $0.candidateDisplayMode = .combined
            $0.storedIsTranslateSwapped = true
        })
        XCTAssertTrue(try fullWidth.controller.handle(
            TestFixtures.keyDownEvent(characters: ","), client: fullWidth.client,
        ))
        XCTAssertEqual(fullWidth.client.insertedTexts, ["，"])
    }

    /// Romanization Only writes romanization, which takes half-width marks whatever is stored.
    func testRomanOnlyDisplay_passesPunctuationThrough() throws {
        let session = try makeSession(configure: {
            $0.candidateDisplayMode = .romanOnly
            $0.storedIsTranslateSwapped = true
        })

        let handled = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: ","), client: session.client,
        )

        XCTAssertFalse(handled)
        XCTAssertEqual(session.client.insertedTexts, [])
    }

    func testAnUnmappedCharacter_passesThroughEvenWhenActive() throws {
        // Digits are tone markers and must reach the host as themselves.
        let session = try makeSession(configure: { $0.storedIsTranslateSwapped = true })

        let handled = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: "5"), client: session.client,
        )

        XCTAssertFalse(handled)
        XCTAssertEqual(session.client.insertedTexts, [])
    }

    func testSwappingModesAfterAnArmedAutoSpace_stillSwaps() throws {
        // A commit in roman-first mode arms the auto-space swap, then the user
        // flips to hanji-first before typing the punctuation. The swap still
        // wins: the word in front of the caret is the romanization that commit
        // wrote, and a display mode changed afterwards does not rewrite it.
        // The full-width map serves the NEXT Hanji word, not this one — so the
        // document reads `taigi, `, half-width, exactly as it would have
        // without the flip. Auto-space (OFF by default) is turned on because
        // an armed space is what this case is about: the arm exists only where
        // the commit earned one.
        let session = try composedSession(configure: { $0.isAutoSpaceEnabled = true })
        session.client.documentTextForReads = ""
        session.client.selectedRangeToReturn = NSRange(location: 0, length: 0)
        _ = try session.controller.handle(TestFixtures.keyDownEvent(characters: "\r"), client: session.client)
        session.client.clearWrites()
        session.store.storedIsTranslateSwapped = true

        let handled = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: ","), client: session.client,
        )

        XCTAssertTrue(handled)
        XCTAssertEqual(session.client.insertedTexts, [", "], "the armed space is still ours")
    }

    // MARK: - Mid-composition (one mutation with the commit)

    /// ⚠ Both rewrites fire here, and they answer to different questions: the
    /// auto space follows what this commit WROTE — the preedit as typed, which
    /// is romanization on a platform shipping TL and POJ only — while the
    /// full-width map still follows the output MODE. So Hanji-first gets
    /// `taigi？ `. The map reading the mode rather than the committed string is
    /// the same approximation this round removed from the auto-space gate,
    /// left standing because which marks Hanji mode types is a Full-width Punctuation policy
    /// question, not an auto-space one.
    func testPunctuationMidComposition_commitsWithTheFullWidthForm_inOneMutation() throws {
        // BOTH domains: `withTranslateSwapped` moves the one the shared
        // coordinator's `ComposingManager` reads (which resolves the commit),
        // `configure` the controller's own store (which the full-width map
        // reads). A case about "the user is in Hanji mode" needs them to agree.
        // Auto-space is OFF by default and is what puts the trailing space in
        // `taigi？ `, so this case turns it on to reach that site.
        try withTranslateSwapped(true) {
            let session = try composedSession(configure: {
                $0.storedIsTranslateSwapped = true
                $0.isAutoSpaceEnabled = true
            })

            _ = try session.controller.handle(TestFixtures.keyDownEvent(characters: "?"), client: session.client)

            XCTAssertEqual(session.client.insertedTexts.count, 1, "commit and punctuation stay one mutation")
            XCTAssertEqual(session.client.insertedTexts.last, "taigi？ ")
        }
    }

    func testPunctuationMidComposition_inRomanFirstMode_staysHalfWidth() throws {
        // The auto-space contract of this site is pinned by
        // `AutoSpaceControllerTests`; here only the character itself matters,
        // and auto-space (OFF by default) is turned on to reach that site.
        let session = try composedSession(configure: {
            $0.isAutoSpaceEnabled = true
            $0.storedIsTranslateSwapped = false
        })

        _ = try session.controller.handle(TestFixtures.keyDownEvent(characters: "?"), client: session.client)

        let inserted = try XCTUnwrap(session.client.insertedTexts.last)
        XCTAssertTrue(inserted.hasSuffix("? "), "got \(session.client.insertedTexts)")
    }

    // MARK: - Width flip (⌃ + punctuation = the other width, once)

    /// The four cells of the contract outside a composition: the bare key
    /// follows the mode, ⌃ types the other width — and is consumed in BOTH
    /// widths, since the host would read the chord as a shortcut. Nothing
    /// stored moves: the next bare key still follows the mode.
    func testControlPunctuationOutsideAComposition_typesTheOtherWidthOnce() throws {
        for (swapped, bare, flipped) in [(true, "，", ","), (false, nil, "，")] {
            let session = try makeSession(configure: { $0.storedIsTranslateSwapped = swapped })

            let handledFlip = try session.controller.handle(
                TestFixtures.keyDownEvent(characters: ",", modifiers: .control, charactersIgnoringModifiers: ","),
                client: session.client,
            )
            XCTAssertTrue(handledFlip, "the flip chord is consumed in either width (swapped=\(swapped))")
            XCTAssertEqual(session.client.insertedTexts, [flipped])
            XCTAssertEqual(session.store.storedIsTranslateSwapped, swapped, "one shot: the mode does not move")

            session.client.clearWrites()
            let handledBare = try session.controller.handle(
                TestFixtures.keyDownEvent(characters: ","), client: session.client,
            )
            XCTAssertEqual(handledBare, bare != nil)
            XCTAssertEqual(session.client.insertedTexts, bare.map { [$0] } ?? [])
        }
    }

    /// The key is read under the modifier: `⌃[` arrives as Escape, and in
    /// romanization mode it types `「` rather than cancelling anything.
    func testControlBracket_typesTheFullWidthBracketInRomanFirstMode() throws {
        let session = try makeSession(configure: { $0.storedIsTranslateSwapped = false })

        let handled = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: "\u{1B}", modifiers: .control, charactersIgnoringModifiers: "["),
            client: session.client,
        )

        XCTAssertTrue(handled)
        XCTAssertEqual(session.client.insertedTexts, ["「"])
    }

    /// Mid-composition the flip rides the commit's single mutation, the other
    /// width from what the bare key would have written at this same site
    /// (`testPunctuationMidComposition_…`). Auto-space off, so the inserted
    /// text is exactly the commit plus the mark.
    func testControlPunctuationMidComposition_commitsWithTheOtherWidth_inOneMutation() throws {
        for (swapped, expected) in [(true, "taigi?"), (false, "taigi？")] {
            let session = try composedSession(configure: {
                $0.storedIsTranslateSwapped = swapped
                $0.isAutoSpaceEnabled = false
            })

            _ = try session.controller.handle(
                TestFixtures.keyDownEvent(characters: "?", modifiers: [.control, .shift], charactersIgnoringModifiers: "?"),
                client: session.client,
            )

            XCTAssertEqual(session.client.insertedTexts.count, 1, "commit and punctuation stay one mutation")
            XCTAssertEqual(session.client.insertedTexts.last, expected, "swapped=\(swapped)")
        }
    }

    /// The flip names its width, so an armed auto space attaches the glyph
    /// the user asked for — the one ordering exception to the bare-key rule
    /// pinned by `testSwappingModesAfterAnArmedAutoSpace_stillSwaps`. In
    /// hanji-first the bare key would have swapped a half-width comma in too,
    /// so there the two agree.
    func testControlPunctuationAfterAnArmedAutoSpace_swapsTheFlippedGlyph() throws {
        for (swapped, expected) in [(false, "， "), (true, ", ")] {
            let session = try composedSession(configure: {
                $0.isAutoSpaceEnabled = true
                $0.storedIsTranslateSwapped = swapped
            })
            session.client.documentTextForReads = ""
            session.client.selectedRangeToReturn = NSRange(location: 0, length: 0)
            _ = try session.controller.handle(TestFixtures.keyDownEvent(characters: "\r"), client: session.client)
            session.client.clearWrites()

            let handled = try session.controller.handle(
                TestFixtures.keyDownEvent(characters: ",", modifiers: .control, charactersIgnoringModifiers: ","),
                client: session.client,
            )

            XCTAssertTrue(handled)
            XCTAssertEqual(session.client.insertedTexts, [expected], "swapped=\(swapped): the flipped comma takes the armed space")
        }
    }

    // MARK: - Helpers

    private struct Session {
        let controller: TaigiInputController
        let client: RecordingTextInputClient
        let store: SettingsStore
    }

    private static let composition = "taigi"
    private static let caretIndex = composition.utf16.count - 1

    /// An activated session under a scratch store carrying the shipped
    /// defaults — full-width punctuation ON, waiting on the swap.
    private func makeSession(configure: ((SettingsStore) -> Void)? = nil) throws -> Session {
        let client = RecordingTextInputClient()
        client.caretRects = [Self.caretIndex: CGRect(x: 120, y: 400, width: 1, height: 18)]
        let controller = try TestFixtures.makeInputController()
        controller.candidatePresenter = RecordingCandidatePresenter()
        let store = try makeScratchSettingsStore()
        configure?(store)
        controller.settings = store
        controller.activateServer(client)
        return Session(controller: controller, client: client, store: store)
    }

    /// A session that has typed `taigi`, so the next punctuation key commits.
    private func composedSession(configure: ((SettingsStore) -> Void)? = nil) throws -> Session {
        let session = try makeSession(configure: configure)
        for character in Self.composition.map(String.init) {
            _ = try session.controller.handle(
                TestFixtures.keyDownEvent(characters: character), client: session.client,
            )
        }
        return session
    }
}
