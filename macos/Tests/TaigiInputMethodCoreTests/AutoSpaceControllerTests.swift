// Auto-space end to end: commits earn a trailing space, attaching punctuation
// swaps with it, and every unverifiable client degrades to pass-through.

import InputMethodKit
@testable import TaigiInputMethodCore
import XCTest

/// Drives the real controller and engine under the shipped auto-space default
/// (ON). The sibling controller suites pin their stores OFF, so this one owns
/// the feature's whole observable surface.
@MainActor
final class AutoSpaceControllerTests: XCTestCase {
    override func setUp() {
        super.setUp()
        InstalledLexicon.installOnce()
    }

    // MARK: - Trailing space after a commit

    func testCommittingACandidate_appendsTheTrailingSpace() throws {
        let session = try composedSession()

        _ = try session.controller.handle(TestFixtures.keyDownEvent(characters: "\r"), client: session.client)

        XCTAssertEqual(session.client.insertedTexts.count, 2, "one commit, one auto space")
        XCTAssertEqual(session.client.insertedTexts.last, " ")
    }

    func testALiteralCommitEndingInAHyphen_earnsNoSpace() throws {
        // trace: "tai-" is a syllable the user is about to continue; the
        // literal-commit chord (⇧Return) writes it verbatim.
        let session = try makeSession()
        for character in "tai-".map(String.init) {
            _ = try session.controller.handle(
                TestFixtures.keyDownEvent(characters: character), client: session.client,
            )
        }

        _ = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: "\r", modifiers: .shift), client: session.client,
        )

        XCTAssertEqual(session.client.insertedTexts, ["tai-"])
    }

    func testSwappedMode_disablesTheSpace() throws {
        let session = try composedSession(configure: { $0.storedIsTranslateSwapped = true })

        _ = try session.controller.handle(TestFixtures.keyDownEvent(characters: "\r"), client: session.client)

        XCTAssertEqual(session.client.insertedTexts.count, 1, "swapped 漢字 mode commits without a space")
        XCTAssertNotEqual(session.client.insertedTexts.last, " ")
    }

    func testTheToggleOff_disablesTheSpace() throws {
        let session = try composedSession(configure: { $0.isAutoSpaceEnabled = false })

        _ = try session.controller.handle(TestFixtures.keyDownEvent(characters: "\r"), client: session.client)

        XCTAssertEqual(session.client.insertedTexts.count, 1)
        XCTAssertNotEqual(session.client.insertedTexts.last, " ")
    }

    // MARK: - Punctuation mid-composition (one mutation)

    func testAttachingPunctuationMidComposition_landsBeforeTheSpace_inOneMutation() throws {
        let session = try composedSession()

        _ = try session.controller.handle(TestFixtures.keyDownEvent(characters: "?"), client: session.client)

        XCTAssertEqual(session.client.insertedTexts.count, 1, "commit and punctuation are one mutation")
        XCTAssertTrue(
            try XCTUnwrap(session.client.insertedTexts.last).hasSuffix("? "),
            "got \(session.client.insertedTexts)",
        )
    }

    func testAnOpeningBracketMidComposition_keepsTheLeadingSpace() throws {
        let session = try composedSession()

        _ = try session.controller.handle(TestFixtures.keyDownEvent(characters: "("), client: session.client)

        XCTAssertEqual(session.client.insertedTexts.count, 1)
        XCTAssertTrue(
            try XCTUnwrap(session.client.insertedTexts.last).hasSuffix(" ("),
            "got \(session.client.insertedTexts)",
        )
    }

    // MARK: - The smart-punctuation swap

    func testAttachingPunctuationAfterTheAutoSpace_swapsWithIt() throws {
        let session = try swappableCommittedSession()
        let documentBefore = try XCTUnwrap(session.client.documentTextForReads)

        let handled = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: "?"), client: session.client,
        )

        XCTAssertTrue(handled, "the swap consumes the key")
        XCTAssertEqual(session.client.insertedTexts.last, "? ")
        XCTAssertEqual(
            session.client.insertReplacementRanges.last,
            NSRange(location: (documentBefore as NSString).length - 1, length: 1),
            "the rewrite must replace exactly the auto space",
        )
        XCTAssertEqual(
            session.client.documentTextForReads,
            documentBefore.dropLast() + "? ",
            "the space moves to AFTER the punctuation",
        )
    }

    func testConsecutiveAttachingPunctuation_keepsSwapping() throws {
        // The simulated document and caret follow each rewrite, so the second
        // swap is verified against the state the first one produced — a `?!`
        // chain re-arms on the NEW caret each time.
        let session = try swappableCommittedSession()
        _ = try session.controller.handle(TestFixtures.keyDownEvent(characters: "?"), client: session.client)

        let handled = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: "!"), client: session.client,
        )

        XCTAssertTrue(handled, "`?!` chains re-arm on each successful swap")
        XCTAssertEqual(session.client.insertedTexts.last, "! ")
        XCTAssertTrue(
            try XCTUnwrap(session.client.documentTextForReads).hasSuffix("?! "),
            "got \(session.client.documentTextForReads ?? "nil")",
        )
    }

    func testAClientThatCannotAnswerItsSelection_neverArms() throws {
        // The default recording client answers NSNotFound — the state of a
        // host without TSMDocumentAccess.
        let session = try composedSession()
        _ = try session.controller.handle(TestFixtures.keyDownEvent(characters: "\r"), client: session.client)
        session.client.clearWrites()

        let handled = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: "?"), client: session.client,
        )

        XCTAssertFalse(handled, "no verified space to swap with — the host gets the key")
        XCTAssertTrue(session.client.writes.isEmpty, "degrading must not touch the document")
    }

    func testAMovedCaret_declinesTheSwap() throws {
        let session = try swappableCommittedSession()
        session.client.selectedRangeToReturn = NSRange(
            location: session.client.selectedRangeToReturn.location + 3, length: 0,
        )

        let handled = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: "?"), client: session.client,
        )

        XCTAssertFalse(handled, "the caret is no longer where the space was measured")
        XCTAssertTrue(session.client.writes.isEmpty)
    }

    func testACharacterThatIsNotOurSpace_declinesTheSwap() throws {
        let session = try swappableCommittedSession()
        // Same caret, but the character in front of it is no longer a space —
        // the state a mouse edit this keydown-only controller never saw leaves.
        session.client.documentTextForReads = String(
            repeating: "x", count: session.client.selectedRangeToReturn.location,
        )

        let handled = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: "?"), client: session.client,
        )

        XCTAssertFalse(handled, "replacing anything but the verified space corrupts the document")
        XCTAssertTrue(session.client.writes.isEmpty)
    }

    func testTheToggleFlippedOffAfterTheCommit_declinesTheSwap() throws {
        let session = try swappableCommittedSession()
        session.store.isAutoSpaceEnabled = false

        let handled = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: "?"), client: session.client,
        )

        XCTAssertFalse(handled)
        XCTAssertTrue(session.client.writes.isEmpty)
    }

    func testAnInterveningKey_disarmsTheSwap() throws {
        let session = try swappableCommittedSession()
        // A letter starts a new composition — the caret has moved on.
        _ = try session.controller.handle(TestFixtures.keyDownEvent(characters: "t"), client: session.client)
        _ = try session.controller.handle(TestFixtures.keyDownEvent(characters: "\u{1B}"), client: session.client)
        session.client.clearWrites()

        let handled = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: "?"), client: session.client,
        )

        XCTAssertFalse(handled, "the arm is one key event wide")
        XCTAssertTrue(session.client.writes.isEmpty)
    }

    // MARK: - Lifecycle and pass-through commits never append

    func testAClickOutsideTheComposition_commitsWithoutTheSpace() throws {
        let session = try composedSession()

        // IMK's click-outside entry point (`commitComposition(_:)`).
        session.controller.commitComposition(session.client)

        XCTAssertEqual(session.client.insertedTexts.count, 1, "the composition alone reaches the document")
        XCTAssertNotEqual(session.client.insertedTexts.last, " ")
    }

    func testAHostShortcutMidComposition_commitsWithoutTheSpace() throws {
        let session = try composedSession()

        let handled = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: "s", modifiers: .command), client: session.client,
        )

        XCTAssertFalse(handled, "⌘S stays the host's")
        XCTAssertEqual(session.client.insertedTexts.count, 1, "commitThenPassThrough must not append")
        XCTAssertNotEqual(session.client.insertedTexts.last, " ")
    }

    // MARK: - The 漢羅 key

    /// Auto-space follows the DOCUMENT, not the output mode: spacing is a
    /// property of romanization (`guá beh khì` needs the gaps, 我欲去 does
    /// not), and the 漢羅 key is the one commit whose script disagrees with the
    /// mode. So 漢字 mode writing a romanization is spaced — the direction the
    /// old mode-read would have refused (USER 2026-08-25).
    func testAlternateCommitOfARomanization_earnsItsSpace() throws {
        let session = try composedSession {
            $0.isAutoSpaceEnabled = true
            $0.storedIsTranslateSwapped = true
        }
        session.client.clearWrites()

        _ = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: " "), client: session.client,
        )

        XCTAssertEqual(session.client.insertedTexts.last, " ")
    }

    /// And the other direction takes none: a hanji written while the settings
    /// lead with romanization is still a hanji, whatever the mode says.
    func testAlternateCommitOfAHanji_takesNoSpace() throws {
        let session = try composedSession {
            $0.isAutoSpaceEnabled = true
            $0.storedIsTranslateSwapped = false
        }
        session.client.clearWrites()

        _ = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: " "), client: session.client,
        )

        XCTAssertFalse(session.client.insertedTexts.contains(" "))
    }

    /// The toggle still outranks everything: OFF means no space from the 漢羅
    /// key either, in the direction that would otherwise earn one.
    func testAlternateCommit_withTheToggleOff_takesNoSpace() throws {
        let session = try composedSession {
            $0.isAutoSpaceEnabled = false
            $0.storedIsTranslateSwapped = true
        }
        session.client.clearWrites()

        _ = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: " "), client: session.client,
        )

        XCTAssertFalse(session.client.insertedTexts.contains(" "))
    }

    /// The punctuation swap has to follow it. The space the 漢羅 key wrote is
    /// this controller's, so `?` must swap with it (`我ê? `) — re-reading the
    /// gate under the OUTPUT MODE instead would refuse, because 漢字 mode says
    /// no commit earns a space, and the one that just did would be denied its
    /// own. What the armed script is stored for.
    func testTheSwapFollowsASpaceTheAlternateCommitWrote() throws {
        let session = try composedSession {
            $0.isAutoSpaceEnabled = true
            $0.storedIsTranslateSwapped = true
        }
        session.client.documentTextForReads = ""
        session.client.selectedRangeToReturn = NSRange(location: 0, length: 0)
        _ = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: " "), client: session.client,
        )
        session.client.clearWrites()

        let handled = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: "?"), client: session.client,
        )

        XCTAssertTrue(handled)
        XCTAssertEqual(session.client.insertedTexts, ["? "])
    }

    /// The flip rule cuts both ways: a space armed by the 漢羅 key is
    /// invalidated by a mode change too, exactly as a `.primary`-armed one is
    /// (`FullWidthPunctuationControllerTests.testSwappingModesAfterAnArmed…`).
    /// The gate is re-read for the ARMED script under the CURRENT settings, so
    /// flipping to romanization-lead makes that alternate commit read as a
    /// hanji — and its space stops being ours to swap.
    func testTheSwapDeclinesWhenTheModeFlipsUnderAnAlternateArmedSpace() throws {
        let session = try composedSession {
            $0.isAutoSpaceEnabled = true
            $0.storedIsTranslateSwapped = true
        }
        session.client.documentTextForReads = ""
        session.client.selectedRangeToReturn = NSRange(location: 0, length: 0)
        _ = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: " "), client: session.client,
        )
        session.client.clearWrites()
        session.store.storedIsTranslateSwapped = false

        let handled = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: "?"), client: session.client,
        )

        XCTAssertFalse(handled, "the space is no longer ours under the settings as they stand")
        XCTAssertTrue(session.client.writes.isEmpty)
    }

    /// And the precedence the 漢羅 key newly makes reachable: in 漢字 mode a
    /// Space-written romanization arms a space, so the `?` that follows matches
    /// BOTH the auto-space swap and the full-width map. The swap wins and the
    /// punctuation stays half-width — the word in front of the caret is
    /// romanization, which reads as Latin text.
    func testTheSwapOutranksTheFullWidthMap_afterAnAlternateCommit() throws {
        let session = try composedSession {
            $0.isAutoSpaceEnabled = true
            $0.storedIsTranslateSwapped = true
        }
        session.client.documentTextForReads = ""
        session.client.selectedRangeToReturn = NSRange(location: 0, length: 0)
        _ = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: " "), client: session.client,
        )
        session.client.clearWrites()

        _ = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: ","), client: session.client,
        )

        XCTAssertEqual(
            session.client.insertedTexts, [", "],
            "half-width — the full-width map would have written 「，」",
        )
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
    /// defaults — auto-space ON.
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

    /// A session that has typed `taigi`, so a commit is one Return away.
    private func composedSession(configure: ((SettingsStore) -> Void)? = nil) throws -> Session {
        let session = try makeSession(configure: configure)
        for character in Self.composition.map(String.init) {
            _ = try session.controller.handle(
                TestFixtures.keyDownEvent(characters: character), client: session.client,
            )
        }
        return session
    }

    /// A session whose commit has landed with the auto space armed for the
    /// swap. The client simulates an empty document from the start, so the
    /// commit, the auto space, and every later rewrite evolve one document —
    /// caret and substring answers included.
    private func swappableCommittedSession() throws -> Session {
        let session = try composedSession()
        session.client.documentTextForReads = ""
        session.client.selectedRangeToReturn = NSRange(location: 0, length: 0)
        _ = try session.controller.handle(TestFixtures.keyDownEvent(characters: "\r"), client: session.client)
        session.client.clearWrites()
        return session
    }
}
