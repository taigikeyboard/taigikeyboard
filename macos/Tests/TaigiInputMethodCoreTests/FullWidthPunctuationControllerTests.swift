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
        restoreStandardSettingsAtTeardown()
        InstalledLexicon.installOnce()
        // The armed-space cases commit the §34 literal with Return on a fresh
        // bar; Show Typed Text First ships OFF since 2026-10-02, so it is pinned ON here.
        UserDefaults.standard.set(true, forKey: SettingsStore.Keys.isLiteralRomanCandidateEnabled.name)
    }

    // MARK: - Outside a composition (the consumed pass-through)

    func testPunctuationOutsideAComposition_insertsTheFullWidthForm() throws {
        for (typed, mark) in [(",", "，"), ("'", "、"), ("\\", "、")] {
            let session = try makeSession(configure: { $0.storedIsHanjiFirst = true })

            let handled = try session.controller.handle(
                TestFixtures.keyDownEvent(characters: typed), client: session.client,
            )

            XCTAssertTrue(handled, typed)
            XCTAssertEqual(session.client.insertedTexts, [mark])
        }
    }

    func testAHostChord_isNeverMapped() throws {
        // ⌘. is a host command that inserts nothing; consuming it would eat
        // the shortcut.
        let session = try makeSession(configure: { $0.storedIsHanjiFirst = true })

        let handled = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: ".", modifiers: .command), client: session.client,
        )

        XCTAssertFalse(handled)
        XCTAssertEqual(session.client.insertedTexts, [])
    }

    func testRomanFirstMode_passesPunctuationThrough() throws {
        // Opted into: the shipped default is hanji-first (2026-09-18).
        for typed in [",", "'", "\\", "|"] {
            let session = try makeSession(configure: { $0.storedIsHanjiFirst = false })

            let handled = try session.controller.handle(
                TestFixtures.keyDownEvent(characters: typed), client: session.client,
            )

            XCTAssertFalse(handled, typed)
            XCTAssertEqual(session.client.insertedTexts, [])
        }
    }

    /// Hanji with Romanization forces the candidate projection hanji-first, but the punctuation
    /// width still follows the stored swap the chord toggles: half-width
    /// until it is on, full-width after.
    func testCombinedDisplay_punctuationWidthFollowsTheStoredSwap() throws {
        let halfWidth = try makeSession(configure: {
            $0.candidateDisplayMode = .combined
            $0.storedIsHanjiFirst = false
        })
        XCTAssertFalse(try halfWidth.controller.handle(
            TestFixtures.keyDownEvent(characters: ","), client: halfWidth.client,
        ), "stored swap off under 合用 keeps the host's half-width comma")
        XCTAssertEqual(halfWidth.client.insertedTexts, [])

        let fullWidth = try makeSession(configure: {
            $0.candidateDisplayMode = .combined
            $0.storedIsHanjiFirst = true
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
            $0.storedIsHanjiFirst = true
        })

        let handled = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: ","), client: session.client,
        )

        XCTAssertFalse(handled)
        XCTAssertEqual(session.client.insertedTexts, [])
    }

    func testAnUnmappedCharacter_passesThroughEvenWhenActive() throws {
        // Digits are tone markers and must reach the host as themselves.
        let session = try makeSession(configure: { $0.storedIsHanjiFirst = true })

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
        session.store.storedIsHanjiFirst = true

        let handled = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: ","), client: session.client,
        )

        XCTAssertTrue(handled)
        XCTAssertEqual(session.client.insertedTexts, [", "], "the armed space is still ours")
    }

    // MARK: - Mid-composition (one mutation with the commit)

    /// ⚠ Both rewrites fire here, and they answer to different questions: the
    /// auto space follows what this commit WROTE — the preedit as typed, which
    /// is romanization under TL and POJ — while the
    /// full-width map still follows the output MODE. So Hanji-first gets
    /// `taigi？ `. The map reading the mode rather than the committed string is
    /// the same approximation this round removed from the auto-space gate,
    /// left standing because which marks Hanji mode types is a Full-width Punctuation policy
    /// question, not an auto-space one.
    func testPunctuationMidComposition_commitsWithTheFullWidthForm_inOneMutation() throws {
        // BOTH domains: `withHanjiFirst` moves `.standard` (the lifecycle
        // snapshot), `configure` the controller's own store (what each key
        // request carries). A case about "the user is in Hanji mode" needs them to agree.
        // Auto-space is OFF by default and is what puts the trailing space in
        // `taigi？ `, so this case turns it on to reach that site.
        try withHanjiFirst(true) {
            for (typed, mark) in [("?", "？"), ("'", "、"), ("\\", "、")] {
                let session = try composedSession(configure: {
                    $0.storedIsHanjiFirst = true
                    $0.isAutoSpaceEnabled = true
                })

                _ = try session.controller.handle(TestFixtures.keyDownEvent(characters: typed), client: session.client)

                XCTAssertEqual(session.client.insertedTexts.count, 1, "commit and punctuation stay one mutation")
                XCTAssertEqual(session.client.insertedTexts.last, "taigi\(mark) ")
            }
        }
    }

    func testPunctuationMidComposition_inRomanFirstMode_staysHalfWidth() throws {
        // The auto-space contract of this site is pinned by
        // `AutoSpaceControllerTests`; here only the character itself matters,
        // and auto-space (OFF by default) is turned on to reach that site.
        let session = try composedSession(configure: {
            $0.isAutoSpaceEnabled = true
            $0.storedIsHanjiFirst = false
        })

        _ = try session.controller.handle(TestFixtures.keyDownEvent(characters: "?"), client: session.client)

        let inserted = try XCTUnwrap(session.client.insertedTexts.last)
        XCTAssertTrue(inserted.hasSuffix("? "), "got \(session.client.insertedTexts)")
    }

    // MARK: - ⌃ + punctuation (no other width; TPS's punctuation key)

    /// No chord types the other width (USER 2026-10-07): outside TPS ⌃, is
    /// the host's chord in either width mode — not consumed, nothing written,
    /// the mode unmoved.
    func testControlPunctuationOutsideTps_isTheHostsInEitherWidth() throws {
        for swapped in [true, false] {
            let session = try makeSession(configure: { $0.storedIsHanjiFirst = swapped })

            let handled = try session.controller.handle(
                TestFixtures.keyDownEvent(characters: ",", modifiers: .control, charactersIgnoringModifiers: ","),
                client: session.client,
            )
            XCTAssertFalse(handled, "swapped=\(swapped)")
            XCTAssertEqual(session.client.insertedTexts, [])
            XCTAssertEqual(session.store.storedIsHanjiFirst, swapped)
        }
    }

    /// Mid-composition the chord finishes the composition as typed and goes
    /// back to the host, like any host chord: no mark, no auto space.
    func testControlPunctuationMidComposition_commitsAndHandsTheChordBack() throws {
        for swapped in [true, false] {
            let session = try composedSession(configure: {
                $0.storedIsHanjiFirst = swapped
                $0.isAutoSpaceEnabled = true
            })

            let handled = try session.controller.handle(
                TestFixtures.keyDownEvent(characters: "?", modifiers: [.control, .shift], charactersIgnoringModifiers: "?"),
                client: session.client,
            )

            XCTAssertFalse(handled, "swapped=\(swapped)")
            XCTAssertEqual(session.client.insertedTexts, [Self.composition], "swapped=\(swapped)")
        }
    }

    /// An armed auto space stays put under the chord: the host's key
    /// attaches nothing.
    func testControlPunctuationAfterAnArmedAutoSpace_leavesTheSpace() throws {
        let session = try composedSession(configure: { $0.isAutoSpaceEnabled = true })
        session.client.documentTextForReads = ""
        session.client.selectedRangeToReturn = NSRange(location: 0, length: 0)
        _ = try session.controller.handle(TestFixtures.keyDownEvent(characters: "\r"), client: session.client)
        session.client.clearWrites()

        let handled = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: ",", modifiers: .control, charactersIgnoringModifiers: ","),
            client: session.client,
        )

        XCTAssertFalse(handled)
        XCTAssertEqual(session.client.insertedTexts, [])
    }

    /// Under TPS the bare `,` types a glyph, so ⌃ is how its mark is typed —
    /// full width, the only width TPS writes. The key is read under the
    /// modifier: `⌃[` arrives as Escape and types `「`, cancelling nothing.
    func testUnderTps_controlPunctuationTypesTheFullWidthMark() throws {
        let session = try makeSession(configure: { _ = $0.switchInputMode(.pick(.tps)) })

        for (characters, unmodified, expected) in [(",", ",", "，"), ("\u{1B}", "[", "「"), ("\u{1C}", "\\", "、")] {
            session.client.clearWrites()
            let handled = try session.controller.handle(
                TestFixtures.keyDownEvent(
                    characters: characters, modifiers: .control, charactersIgnoringModifiers: unmodified,
                ),
                client: session.client,
            )
            XCTAssertTrue(handled, unmodified)
            XCTAssertEqual(session.client.insertedTexts, [expected])
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

    /// An activated session over `.standard` — the domain `withHanjiFirst`
    /// writes the swap to — carrying the shipped defaults: full-width
    /// punctuation ON, waiting on the swap.
    private func makeSession(configure: ((SettingsStore) -> Void)? = nil) throws -> Session {
        let client = RecordingTextInputClient()
        client.caretRects = [Self.caretIndex: CGRect(x: 120, y: 400, width: 1, height: 18)]
        let controller = try TestFixtures.makeInputController()
        controller.candidatePresenter = RecordingCandidatePresenter()
        let store = SettingsStore()
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
