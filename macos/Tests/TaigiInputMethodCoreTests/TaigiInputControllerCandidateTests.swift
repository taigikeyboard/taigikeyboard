// The candidate slice end to end: keys in, bar content and document text out.

import InputMethodKit
@testable import TaigiInputMethodCore
import XCTest

/// Drives the real controller, the real engine and the real dictionary against a
/// recording bar. What is asserted is the routing — which key changes which part
/// of the list, when the bar goes up and comes down, and who is allowed to take
/// it down — not the ranking the dictionary happens to produce.
@MainActor
final class TaigiInputControllerCandidateTests: XCTestCase {
    override func setUp() {
        super.setUp()
        InstalledLexicon.installOnce()
    }

    // MARK: - Showing

    func testTypingRomanization_putsCandidatesOnTheBar() throws {
        let session = try composedSession()

        let content = try XCTUnwrap(session.presenter.shownContent)
        XCTAssertFalse(content.cells.isEmpty, "the dictionary has entries for taigi")
        XCTAssertEqual(
            session.presenter.selectedIndex,
            0,
            "a fresh list starts on its first candidate",
        )
    }

    /// A cell leads with the script the commit leads with, under one settings
    /// snapshot. A bar showing the romanization while the document gets the
    /// hanji is a visible defect, and what keeps them together is that the cell
    /// and the commit resolve the swap setting from the same snapshot
    /// (`ComposingManager.cellContent(for:)` / `documentText(for:)`).
    ///
    /// Under the shipped defaults the two are the same string; the general
    /// rule — the cell leads with the script the commit leads with, whatever
    /// the output settings — is pinned at the mapping level by
    /// `CandidateCellContentTests`.
    func testBarCellPrimary_isWhatCommittingWrites() throws {
        let session = try composedSession()
        let firstCell = try XCTUnwrap(session.presenter.shownContent).cells[0]
        session.client.clearWrites()

        _ = try session.controller.handle(TestFixtures.keyDownEvent(characters: "\r"), client: session.client)

        XCTAssertEqual(session.client.insertedTexts.last, firstCell.text)
    }

    /// The window shows BOTH scripts by default, matching iOS and Android: a
    /// Taigi word is the `(漢字, 羅馬字)` pair, and a bar showing one of them
    /// makes different words read identically.
    func testBarCells_carryBothScriptsByDefault() throws {
        let session = try composedSession()

        let cells = try XCTUnwrap(session.presenter.shownContent).cells

        XCTAssertTrue(
            cells.contains { $0.annotation?.isEmpty == false },
            "taigi has hanji candidates, so some cell must show its other script",
        )
    }

    func testCandidatesAreAnchoredToTheCaret_notToTheStartOfTheMarkedRegion() throws {
        let session = try composedSession()

        guard case let .show(_, caretRect) = try XCTUnwrap(session.presenter.calls.last) else {
            return XCTFail("the bar must have been shown")
        }
        XCTAssertEqual(caretRect, Self.caretRectAtEndOfComposition)
        XCTAssertEqual(
            session.client.caretRectQueries.last,
            Self.caretIndex,
            "the anchor is the last character of the marked region; index 0 is its start, "
                + "so the bar would drift further from the caret the longer the composition ran",
        )
    }

    /// Clients that cannot place an index answer with a zero rectangle, and the
    /// walk goes back until one of them is real — McBopomofo's loop
    /// (`InputMethodController.swift:886-891`).
    func testCaretAnchor_walksBackUntilTheClientAnswers() throws {
        let session = try composedSession(caretRects: [0: Self.caretRectAtEndOfComposition])

        XCTAssertNotNil(session.presenter.shownContent, "an earlier index answered, so the bar can be placed")
        XCTAssertEqual(
            session.client.caretRectQueries.suffix(2),
            [1, 0],
            "the walk steps back one index at a time rather than giving up at the end",
        )
    }

    func testCaretAnchorUnavailable_keepsTheBarHidden() throws {
        let session = try composedSession(caretRects: [:])

        XCTAssertFalse(
            session.presenter.isShowing,
            "a bar parked at the screen's corner would point at text that is not there",
        )
    }

    /// The list has to be dropped with the window, not merely hidden behind it.
    /// The key contract turns on whether candidates are on screen, so a model
    /// left alive behind a hidden bar swallows the arrows and lets Space commit a
    /// candidate nobody can see.
    func testCaretAnchorUnavailable_returnsTheArrowsToTheHost() throws {
        let session = try composedSession(caretRects: [:])

        let handled = try session.controller.handle(
            Self.arrowEvent(.rightArrow),
            client: session.client,
        )

        XCTAssertFalse(handled, "there is no visible list for an arrow to walk")
    }

    func testCaretAnchorUnavailable_leavesSpaceAsTheDocumentsSpace() throws {
        let session = try composedSession(caretRects: [:])
        session.client.clearWrites()

        _ = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: " "),
            client: session.client,
        )

        XCTAssertEqual(
            session.client.insertedTexts.joined(),
            "taigi ",
            "committing an unseen candidate would put a word in the document the user never saw offered",
        )
    }

    /// A client whose caret really is at the screen origin still reports a line
    /// height, and rejecting it would hide the bar for a client that answered.
    func testCaretAtTheScreenOrigin_isARealAnchor() throws {
        let atOrigin = CGRect(x: 0, y: 0, width: 1, height: 18)
        let session = try composedSession(caretRects: [Self.caretIndex: atOrigin])

        guard case let .show(_, caretRect) = try XCTUnwrap(session.presenter.calls.last) else {
            return XCTFail("the bar must have been shown")
        }
        XCTAssertEqual(caretRect, atOrigin)
    }

    // MARK: - Navigating

    func testArrowKeys_moveTheHighlightAndClampAtTheStart() throws {
        let session = try composedSession()

        session.press(.rightArrow)
        XCTAssertEqual(session.presenter.selectedIndex, 1)

        session.press(.leftArrow)
        session.press(.leftArrow)
        XCTAssertEqual(
            session.presenter.selectedIndex,
            0,
            "the highlight stops at the first candidate rather than wrapping to the last",
        )
    }

    /// Where a page turn LANDS depends on measured widths, so the geometry is
    /// pinned by `HorizontalPageLayoutTests` — what this seam owes is that the
    /// key reaches the window as the right direction and is consumed.
    func testPagingKeys_reachTheWindowAsDirections() throws {
        let session = try composedSession()

        session.press(.pageDown)
        session.press(.pageUp)

        XCTAssertTrue(session.presenter.calls.contains(.navigate(.pageDown)))
        XCTAssertTrue(session.presenter.calls.contains(.navigate(.pageUp)))
    }

    func testArrowKeys_reachTheHostWhenNoBarIsUp() throws {
        let session = try makeSession()

        let handled = try session.controller.handle(
            Self.arrowEvent(.rightArrow),
            client: session.client,
        )

        XCTAssertFalse(handled, "with nothing to navigate, the arrow moves the host's caret")
    }

    // MARK: - Committing

    func testReturn_commitsTheHighlightedCandidate() throws {
        let session = try composedSession()
        session.press(.rightArrow)
        let highlighted = try XCTUnwrap(session.presenter.shownContent).cells[1].text
        session.client.clearWrites()

        let handled = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: "\r"),
            client: session.client,
        )

        XCTAssertTrue(handled)
        XCTAssertEqual(
            session.client.insertedTexts.last,
            highlighted,
            "Return takes the candidate the user moved to, not the one the list opened on",
        )
    }

    /// Space walks the bar rather than committing from it — the system Zhuyin
    /// keyboard's space bar, which is the default this ships with
    /// (`ComposingAction.nextCandidate`).
    func testSpace_walksToTheNextCandidateWithoutCommitting() throws {
        let session = try composedSession()
        let cells = try XCTUnwrap(session.presenter.shownContent).cells
        session.client.clearWrites()

        let handled = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: " "),
            client: session.client,
        )

        XCTAssertTrue(handled)
        XCTAssertTrue(session.client.insertedTexts.isEmpty, "walking the bar writes nothing")
        XCTAssertEqual(session.presenter.selectedIndex, 1)
        XCTAssertEqual(try XCTUnwrap(session.presenter.shownContent).cells, cells)
    }

    func testControlDigit_commitsThatSlotOfTheVisiblePage() throws {
        let session = try composedSession()
        let secondCell = try XCTUnwrap(session.presenter.shownContent).cells[1].text
        session.client.clearWrites()

        let handled = try session.controller.handle(
            Self.controlDigitEvent(slot: 1),
            client: session.client,
        )

        XCTAssertTrue(handled)
        XCTAssertEqual(session.client.insertedTexts.last, secondCell)
    }

    /// A candidate that consumes only part of the buffer is nailed rather than
    /// finalized: nothing reaches the document, the rest of the composition
    /// carries on, and the bar has to come back holding candidates for the tail
    /// that is left. Without the refresh it would keep offering spans measured
    /// against bytes the nail has already consumed.
    func testCommittingAPartialCandidate_keepsTheBarUpWithFreshCandidates() throws {
        // Which slot holds a shorter-than-the-buffer candidate is the
        // dictionary's business, so it is searched for rather than hardcoded —
        // one fresh session per slot, since choosing one changes the state.
        var nailed: (session: Session, firstPage: CandidateWindowContent)?
        for slot in 0 ..< HorizontalPageLayout.pageSize where nailed == nil {
            let session = try composedSession()
            let firstPage = try XCTUnwrap(session.presenter.shownContent)
            guard slot < firstPage.cells.count else { break }
            session.client.clearWrites()

            _ = try session.controller.handle(
                Self.controlDigitEvent(slot: slot),
                client: session.client,
            )

            if session.client.insertedTexts.isEmpty, session.presenter.isShowing {
                nailed = (session, firstPage)
            }
        }

        let (session, firstPage) = try XCTUnwrap(
            nailed,
            "taigi must offer a candidate shorter than the whole buffer for a nail to be reachable",
        )
        let tailPage = try XCTUnwrap(
            session.presenter.shownContent,
            "the composition is still running, so the user still needs candidates for its tail",
        )
        XCTAssertNotEqual(tailPage.cells, firstPage.cells, "the tail is a different buffer to segment")
        XCTAssertEqual(session.presenter.selectedIndex, 0, "a fresh list starts on its first candidate")
    }

    /// Choosing candidates until there is no tail left must end — with the
    /// document written and the bar down — rather than loop.
    func testCommittingCandidatesUntilTheBufferRunsOut_endsWithTheBarDown() throws {
        let session = try composedSession()
        let confirm = try TestFixtures.keyDownEvent(characters: "\r")

        // One press per syllable is the most that can be needed; the extra
        // iterations exist so a failure reads as "never ended" rather than
        // "ended one press later than the fixture guessed".
        for _ in 0 ..< (Self.composition.count + 1) where session.presenter.isShowing {
            _ = session.controller.handle(confirm, client: session.client)
        }

        XCTAssertFalse(
            session.presenter.isShowing,
            "an ended composition has nothing left to choose between",
        )
        XCTAssertFalse(
            session.client.insertedTexts.isEmpty,
            "the chosen candidates must reach the document",
        )
    }

    /// 直接送出漢字 and 直接送出羅馬字 write one script whatever the 漢羅
    /// setting says — the user is overriding their preference for this word,
    /// not changing it.
    func testTheHanjiCommit_writesTheHanjiAndLeavesTheSettingAlone() throws {
        let session = try makeScriptCommitSession(.commitHanji, selecting: { $0.annotation != nil })
        let expected = try XCTUnwrap(session.cell.annotation)
        session.client.clearWrites()

        _ = session.controller.handle(session.event, client: session.client)

        XCTAssertEqual(session.client.insertedTexts.last, expected)
        XCTAssertFalse(
            session.controller.settings.isTranslateSwapped,
            "the output setting is untouched — this was one word, not a preference",
        )
    }

    func testTheRomanizationCommit_writesTheRomanization() throws {
        let session = try makeScriptCommitSession(.commitRomanization, selecting: { $0.annotation != nil })
        session.client.clearWrites()

        _ = session.controller.handle(session.event, client: session.client)

        XCTAssertEqual(session.client.insertedTexts.last, session.cell.text)
    }

    private struct ScriptCommitSession {
        let controller: TaigiInputController
        let client: RecordingTextInputClient
        let presenter: RecordingCandidatePresenter
        /// The highlighted cell. Under the shipped output settings it leads
        /// with the romanization and annotates with the Hanji, which is what
        /// lets a case name the two scripts without reaching into the
        /// controller's candidate list.
        let cell: CandidateCellContent
        let event: NSEvent
    }

    /// A composed session with `action` recorded on ⌥Return, moved onto the
    /// first candidate whose cell `matching` accepts.
    private func makeScriptCommitSession(
        _ action: ComposingAction,
        selecting matching: ((CandidateCellContent) -> Bool)? = nil,
    ) throws -> ScriptCommitSession {
        let suiteName = "ScriptCommit.\(UUID().uuidString)"
        let userDefaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
        addTeardownBlock { userDefaults.removePersistentDomain(forName: suiteName) }
        let store = SettingsStore(userDefaults: userDefaults)
        store.setComposingChord(try ComposingKeyChord.make(key: "\r", modifiers: .option).get(), for: action)

        let session = try makeSession()
        session.controller.settings = store
        for character in Self.composition.map(String.init) {
            _ = try session.controller.handle(
                TestFixtures.keyDownEvent(characters: character),
                client: session.client,
            )
        }

        let cells = try XCTUnwrap(session.presenter.shownContent).cells
        var index = 0
        if let matching {
            guard let match = cells.firstIndex(where: matching) else {
                throw XCTSkip("the dictionary produced no candidate this case can use")
            }
            index = match
            for _ in 0 ..< index { session.press(.rightArrow) }
        }

        return ScriptCommitSession(
            controller: session.controller,
            client: session.client,
            presenter: session.presenter,
            cell: cells[index],
            event: try TestFixtures.keyDownEvent(characters: "\r", modifiers: .option),
        )
    }

    func testShiftReturn_commitsTheLiteralAndTakesTheBarDown() throws {
        let session = try composedSession()
        session.client.clearWrites()

        _ = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: "\r", modifiers: .shift),
            client: session.client,
        )

        XCTAssertEqual(
            session.client.insertedTexts.last,
            "taigi",
            "⇧Return keeps the letters that were typed, not the candidate the bar suggested",
        )
        XCTAssertFalse(session.presenter.isShowing)
    }

    func testEscape_takesTheBarDownWithTheComposition() throws {
        let session = try composedSession()

        _ = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: "\u{1B}"),
            client: session.client,
        )

        XCTAssertFalse(session.presenter.isShowing)
    }

    // MARK: - Ownership

    func testDeactivate_afterAnotherSessionTookOver_leavesTheNewBarAlone() throws {
        let presenter = RecordingCandidatePresenter()
        let leaving = try composedSession(presenter: presenter)
        // IMK activates the incoming session before it deactivates the outgoing
        // one, and the arriving session is what takes the bar over.
        _ = try composedSession(presenter: presenter)
        XCTAssertTrue(presenter.isShowing, "the arriving session put its own bar up")

        leaving.controller.deactivateServer(leaving.client)

        XCTAssertTrue(
            presenter.isShowing,
            """
            the outgoing session no longer owns the bar, so its late teardown must not take down \
            the incoming session's candidates
            """,
        )
    }

    func testActivate_takesDownABarLeftByAnEarlierSession() throws {
        let presenter = RecordingCandidatePresenter()
        _ = try composedSession(presenter: presenter)
        XCTAssertTrue(presenter.isShowing)

        _ = try makeSession(presenter: presenter)

        XCTAssertFalse(
            presenter.isShowing,
            "a session that starts typing must not inherit the candidates of the one before it",
        )
    }

    func testHidePalettes_takesTheBarDownWithoutEndingTheComposition() throws {
        let session = try composedSession()
        session.client.clearWrites()

        session.controller.hidePalettes()

        XCTAssertFalse(session.presenter.isShowing)
        XCTAssertTrue(
            session.client.insertedTexts.isEmpty,
            "the system asked for the screen back, not for the user's composition to be finished",
        )
        // The composition is still the engine's, so the next character extends it.
        _ = try session.controller.handle(TestFixtures.keyDownEvent(characters: "a"), client: session.client)
        XCTAssertEqual(session.client.writes.last, .setMarkedText("taigia", selectionLocation: 6))
    }

    /// Switching romanization from the input-source menu has to take the bar
    /// with it. The candidates on screen were fetched under the old
    /// romanization, and Space commits whichever one is highlighted — a bar left
    /// standing would write a candidate the new mode would never have offered.
    ///
    /// Driven through `doCommandBySelector:`, which is how IMK delivers a menu
    /// command (`IMKInputController.h:283-296`), against a real composition —
    /// asserting only that a hide was requested would pass against a bar that
    /// stayed up because the hide named the wrong session.
    func testSwitchingRomanizationFromTheMenu_takesTheBarDown() throws {
        let session = try composedSession()
        let suiteName = "TaigiInputControllerCandidateTests.\(UUID().uuidString)"
        let userDefaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
        defer { userDefaults.removePersistentDomain(forName: suiteName) }
        session.controller.settings = SettingsStore(userDefaults: userDefaults)
        XCTAssertTrue(session.presenter.isShowing, "the composition put a bar up to take down")

        // Found by the command it sends: the menu's titles follow the display language, so a
        // lookup by text would only hold in the language this case was written in.
        let poj = try XCTUnwrap(
            XCTUnwrap(session.controller.menu()).items
                .first { $0.action == Selector(("selectInputModePOJ:")) }?.action,
        )
        session.controller.doCommand(by: poj, command: [:])

        XCTAssertFalse(session.presenter.isShowing)
        XCTAssertEqual(session.controller.settings.inputMode, .poj)
    }

    func testHidePalettes_returnsTheCandidateKeysToTheHost() throws {
        let session = try composedSession()

        session.controller.hidePalettes()
        let handled = try session.controller.handle(Self.arrowEvent(.rightArrow), client: session.client)

        XCTAssertFalse(
            handled,
            "with no bar on screen the arrows are the host's again — navigating an invisible list "
                + "would take a key away for nothing",
        )
    }

    // MARK: - Fixtures

    /// `taigi`, the composition every case here types: it segments into two
    /// syllables, so the dictionary offers both whole-buffer and shorter
    /// candidates.
    private static let composition = "taigi"

    /// The index of the last character of `taigi`'s marked region, which is where
    /// the caret is and therefore the first index the anchor walk asks about.
    private static let caretIndex = composition.utf16.count - 1

    private static let caretRectAtEndOfComposition = CGRect(x: 120, y: 400, width: 1, height: 18)

    /// A `⌃n` chord for a slot, counting from zero. `characters` carries the
    /// control character the chord really arrives as, so the event is the one
    /// AppKit would deliver rather than a convenient fiction.
    private static func controlDigitEvent(slot: Int) throws -> NSEvent {
        let digit = String(slot + 1)
        // ⌃2 through ⌃8 are rewritten by Control; ⌃1 and ⌃9 are not.
        let controlCharacters = [
            "2": "\u{0}", "3": "\u{1B}", "4": "\u{1C}", "5": "\u{1D}",
            "6": "\u{1E}", "7": "\u{1F}", "8": "\u{7F}",
        ]
        return try TestFixtures.keyDownEvent(
            characters: controlCharacters[digit] ?? digit,
            modifiers: .control,
            charactersIgnoringModifiers: digit,
        )
    }

    private static func arrowEvent(_ key: NavigationKey) throws -> NSEvent {
        let functionKey: Int = switch key {
        case .leftArrow: NSLeftArrowFunctionKey
        case .rightArrow: NSRightArrowFunctionKey
        case .upArrow: NSUpArrowFunctionKey
        case .downArrow: NSDownArrowFunctionKey
        case .pageUp: NSPageUpFunctionKey
        case .pageDown: NSPageDownFunctionKey
        }
        return try TestFixtures.keyDownEvent(
            characters: String(UnicodeScalar(functionKey)!),
            modifiers: .function,
        )
    }

    private struct Session {
        let controller: TaigiInputController
        let client: RecordingTextInputClient
        let presenter: RecordingCandidatePresenter

        @MainActor
        func press(_ key: NavigationKey) {
            guard let event = try? arrowEvent(key) else { return XCTFail("could not build \(key)") }
            _ = controller.handle(event, client: client)
        }
    }

    /// An activated session that has typed nothing yet.
    private func makeSession(
        presenter: RecordingCandidatePresenter = RecordingCandidatePresenter(),
        caretRects: [Int: CGRect]? = nil,
    ) throws -> Session {
        let client = RecordingTextInputClient()
        client.caretRects = caretRects ?? [Self.caretIndex: Self.caretRectAtEndOfComposition]
        let controller = try TestFixtures.makeInputController()
        controller.candidatePresenter = presenter
        controller.activateServer(client)
        return Session(controller: controller, client: client, presenter: presenter)
    }

    /// An activated session that has typed `taigi`, so a bar is up.
    private func composedSession(
        presenter: RecordingCandidatePresenter = RecordingCandidatePresenter(),
        caretRects: [Int: CGRect]? = nil,
    ) throws -> Session {
        let session = try makeSession(presenter: presenter, caretRects: caretRects)
        for character in Self.composition.map(String.init) {
            _ = try session.controller.handle(
                TestFixtures.keyDownEvent(characters: character),
                client: session.client,
            )
        }
        return session
    }
}
