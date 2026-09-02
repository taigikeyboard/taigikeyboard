// Full-width punctuation end to end: mapped in the swapped mode at both
// insertion sites, untouched everywhere else.

import InputMethodKit
@testable import TaigiInputMethodCore
import XCTest

/// Drives the real controller and engine with the 漢羅對調 swap ON, where the
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
        let session = try makeSession()

        let handled = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: ","), client: session.client,
        )

        XCTAssertFalse(handled, "roman-first output keeps the host's half-width comma")
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
        // The full-width map serves the NEXT 漢字 word, not this one — so the
        // document reads `taigi, `, half-width, exactly as it would have
        // without the flip.
        let session = try composedSession()
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
    /// full-width map still follows the output MODE. So 漢字優先 gets
    /// `taigi？ `. The map reading the mode rather than the committed string is
    /// the same approximation this round removed from the auto-space gate,
    /// left standing because which marks 漢字 mode types is a 全形標點 policy
    /// question, not an auto-space one.
    func testPunctuationMidComposition_commitsWithTheFullWidthForm_inOneMutation() throws {
        // BOTH domains: `withTranslateSwapped` moves the one the shared
        // coordinator's `ComposingManager` reads (which resolves the commit),
        // `configure` the controller's own store (which the full-width map
        // reads). A case about "the user is in 漢字 mode" needs them to agree.
        try withTranslateSwapped(true) {
            let session = try composedSession(configure: { $0.storedIsTranslateSwapped = true })

            _ = try session.controller.handle(TestFixtures.keyDownEvent(characters: "?"), client: session.client)

            XCTAssertEqual(session.client.insertedTexts.count, 1, "commit and punctuation stay one mutation")
            XCTAssertEqual(session.client.insertedTexts.last, "taigi？ ")
        }
    }

    func testPunctuationMidComposition_inRomanFirstMode_staysHalfWidth() throws {
        // The auto-space contract of this site is pinned by
        // `AutoSpaceControllerTests`; here only the character itself matters,
        // and auto-space (OFF by default) is turned on to reach that site.
        let session = try composedSession(configure: { $0.isAutoSpaceEnabled = true })

        _ = try session.controller.handle(TestFixtures.keyDownEvent(characters: "?"), client: session.client)

        let inserted = try XCTUnwrap(session.client.insertedTexts.last)
        XCTAssertTrue(inserted.hasSuffix("? "), "got \(session.client.insertedTexts)")
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
