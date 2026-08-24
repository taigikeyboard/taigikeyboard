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
        let session = try makeSession(configure: { $0.isTranslateSwapped = true })

        let handled = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: ","), client: session.client,
        )

        XCTAssertTrue(handled, "the mapped key is consumed, not passed to the host")
        XCTAssertEqual(session.client.insertedTexts, ["，"])
    }

    func testAHostChord_isNeverMapped() throws {
        // ⌘. is a host command that inserts nothing; consuming it would eat
        // the shortcut.
        let session = try makeSession(configure: { $0.isTranslateSwapped = true })

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
        let session = try makeSession(configure: { $0.isTranslateSwapped = true })

        let handled = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: "5"), client: session.client,
        )

        XCTAssertFalse(handled)
        XCTAssertEqual(session.client.insertedTexts, [])
    }

    func testSwappingModesAfterAnArmedAutoSpace_mapsInsteadOfSwapping() throws {
        // The transient the two complementary gates leave behind: a commit in
        // roman-first mode arms the auto-space swap, then the user flips to
        // hanji-first before typing the punctuation. The old space must NOT
        // swap — its gate is off now — and the key maps instead, so the
        // document reads `guá ，`. Pinned so a future reordering of the
        // pass-through branches cannot quietly resolve the race the other way.
        let session = try composedSession()
        session.client.documentTextForReads = ""
        session.client.selectedRangeToReturn = NSRange(location: 0, length: 0)
        _ = try session.controller.handle(TestFixtures.keyDownEvent(characters: "\r"), client: session.client)
        session.client.clearWrites()
        session.store.isTranslateSwapped = true

        let handled = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: ","), client: session.client,
        )

        XCTAssertTrue(handled)
        XCTAssertEqual(session.client.insertedTexts, ["，"], "the stale armed space must not swap")
    }

    // MARK: - Mid-composition (one mutation with the commit)

    func testPunctuationMidComposition_commitsWithTheFullWidthForm_inOneMutation() throws {
        let session = try composedSession(configure: { $0.isTranslateSwapped = true })

        _ = try session.controller.handle(TestFixtures.keyDownEvent(characters: "?"), client: session.client)

        XCTAssertEqual(session.client.insertedTexts.count, 1, "commit and punctuation stay one mutation")
        let inserted = try XCTUnwrap(session.client.insertedTexts.last)
        XCTAssertTrue(inserted.hasSuffix("？"), "got \(session.client.insertedTexts)")
        XCTAssertFalse(inserted.contains("? "), "the swapped mode earns no auto space")
    }

    func testPunctuationMidComposition_inRomanFirstMode_staysHalfWidth() throws {
        // The auto-space contract of this site is pinned by
        // `AutoSpaceControllerTests`; here only the character itself matters.
        let session = try composedSession()

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
