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
        // Auto-space ships ON and would append " " after every commit here —
        // orthogonal noise to the candidate routing this suite pins, so it is
        // OFF, which doubles as OFF-gate coverage; `AutoSpaceControllerTests`
        // owns the ON behaviour. Written to `.standard` rather than through an
        // injected scratch store because the swap-rerender case needs the
        // controller and the shared coordinator's engine reading ONE domain.
        UserDefaults.standard.set(false, forKey: SettingsStore.Keys.isAutoSpaceEnabled.name)
    }

    override func tearDown() {
        UserDefaults.standard.removeObject(forKey: SettingsStore.Keys.isAutoSpaceEnabled.name)
        super.tearDown()
    }

    // MARK: - Which key picks

    /// The window draws the chosen set whatever the buffer — a tone digit
    /// changes nothing about which keys pick.
    func testSlotKeyHint_isTheChosenSet_whateverTheBuffer() throws {
        for keySet in CandidateSlotKeySet.allCases {
            let session = try composedSession(under: keySet)
            XCTAssertEqual(try XCTUnwrap(session.presenter.shownContent).slotKeySet, keySet)

            _ = try session.controller.handle(
                TestFixtures.keyDownEvent(characters: "5"), client: session.client,
            )
            XCTAssertEqual(
                try XCTUnwrap(session.presenter.shownContent).slotKeySet, keySet,
                "`taigi5` is picked from with the same keys as `taigi`",
            )
        }
    }

    /// A bare digit after a tone is still typed, not a pick (USER 2026-08-28,
    /// retiring the 2026-08-24 rule): `taigi5` + `1` composes `taigi51`, and
    /// nothing reaches the document.
    func testAfterATone_aBareDigitIsStillTyped_underEverySet() throws {
        for keySet in CandidateSlotKeySet.allCases {
            let session = try composedSession(under: keySet)
            _ = try session.controller.handle(
                TestFixtures.keyDownEvent(characters: "5"), client: session.client,
            )
            session.client.clearWrites()

            _ = try session.controller.handle(
                TestFixtures.keyDownEvent(characters: "1"), client: session.client,
            )

            XCTAssertEqual(session.client.insertedTexts.last, nil, "\(keySet)")
            guard case .setMarkedText = session.client.writes.last else {
                return XCTFail("\(keySet): the digit should have re-rendered the composition — got \(session.client.writes)")
            }
        }
    }

    /// The drawn keys are the set the user chose for the slots.
    func testSlotKeyHint_carriesTheChosenKeySet() throws {
        try withSlotKeySet(.option) {
            let session = try composedSession()

            XCTAssertEqual(
                try XCTUnwrap(session.presenter.shownContent).slotKeySet,
                .option,
            )
        }
    }

    /// The hint is a per-show snapshot, so a rebind reaches it on the very next
    /// keystroke — the window is never told a key set that has been replaced.
    func testSlotKeyHint_followsARebindOnTheNextKeystroke() throws {
        let session = try composedSession()
        XCTAssertEqual(try XCTUnwrap(session.presenter.shownContent).slotKeySet, .bareKeys)

        try withSlotKeySet(.option) {
            _ = try session.controller.handle(
                TestFixtures.keyDownEvent(characters: "k"), client: session.client,
            )

            XCTAssertEqual(
                try XCTUnwrap(session.presenter.shownContent).slotKeySet, .option,
            )
        }
    }

    // MARK: - The 漢羅 key

    /// Space writes the highlighted candidate in the script Return does not —
    /// what makes `我ê名` cost one key for the romanized word instead of a round
    /// trip through the output setting.
    func testSpace_commitsTheOtherScript() throws {
        let session = try composedSession()
        let shown = try XCTUnwrap(session.presenter.shownContent)
        // Default output leads with the romanization, so the cell's annotation
        // IS the other script — the one on screen under the primary.
        let otherScript = try XCTUnwrap(shown.cells[0].annotation)
        session.client.clearWrites()

        _ = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: " "), client: session.client,
        )

        XCTAssertEqual(session.client.insertedTexts.last, otherScript)
    }

    /// And Return still writes the primary one, from the same list — the two
    /// keys are the two scripts, and neither moves the output setting.
    func testReturnAndSpace_writeTheTwoScriptsOfTheSameCandidate() throws {
        let viaReturn = try composedSession()
        viaReturn.client.clearWrites()
        _ = try viaReturn.controller.handle(
            TestFixtures.keyDownEvent(characters: "\r"), client: viaReturn.client,
        )

        let viaSpace = try composedSession()
        viaSpace.client.clearWrites()
        _ = try viaSpace.controller.handle(
            TestFixtures.keyDownEvent(characters: " "), client: viaSpace.client,
        )

        XCTAssertNotEqual(
            viaSpace.client.insertedTexts.last,
            viaReturn.client.insertedTexts.last,
            "the same candidate, the two scripts",
        )
        XCTAssertEqual(
            viaSpace.controller.settings.isTranslateSwapped,
            viaReturn.controller.settings.isTranslateSwapped,
            "neither key moves the output setting — that is the point",
        )
    }

    /// The mode never moves, however many times the key is used: this is a
    /// per-word choice, not a toggle wearing a different hat.
    func testSpace_leavesTheOutputSettingAlone() throws {
        let session = try composedSession()
        let before = session.controller.settings.isTranslateSwapped

        _ = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: " "), client: session.client,
        )

        XCTAssertEqual(session.controller.settings.isTranslateSwapped, before)
    }

    /// The mirror image, in 漢字 mode — the direction the `我ê名` example is
    /// actually typed in: Return writes the hanji, Space writes the
    /// romanization, and the key is the same key either way.
    func testSwappedMode_ReturnWritesHanjiAndSpaceWritesRomanization() throws {
        try withRestoredSwapSetting {
            UserDefaults.standard.set(true, forKey: SettingsStore.Keys.isTranslateSwapped.name)

            let viaReturn = try composedSession()
            let cell = try XCTUnwrap(viaReturn.presenter.shownContent).cells[0]
            viaReturn.client.clearWrites()
            _ = try viaReturn.controller.handle(
                TestFixtures.keyDownEvent(characters: "\r"), client: viaReturn.client,
            )

            let viaSpace = try composedSession()
            viaSpace.client.clearWrites()
            _ = try viaSpace.controller.handle(
                TestFixtures.keyDownEvent(characters: " "), client: viaSpace.client,
            )

            // Swapped, the cell leads with the hanji and annotates the roman.
            XCTAssertEqual(viaReturn.client.insertedTexts.last, cell.text, "Return: the hanji")
            XCTAssertEqual(
                viaSpace.client.insertedTexts.last, cell.annotation, "Space: the romanization",
            )
        }
    }

    /// With 括號標注 on, Return writes the bracketed pair and Space still writes
    /// ONE script — the bracket setting says how to show a candidate that
    /// carries both, and Space is the request for the other one by itself.
    func testBothScriptsMode_SpaceStillWritesASingleScript() throws {
        try withSetting(SettingsStore.Keys.isOutputBothScripts.name, to: true) {
            let session = try composedSession()
            let cell = try XCTUnwrap(session.presenter.shownContent).cells[0]
            session.client.clearWrites()

            _ = try session.controller.handle(
                TestFixtures.keyDownEvent(characters: " "), client: session.client,
            )

            let written = try XCTUnwrap(session.client.insertedTexts.last)
            XCTAssertEqual(written, cell.annotation)
            XCTAssertFalse(written.contains("("), "no bracketed pair — one script was asked for")
        }
    }

    /// A candidate with only one script declines the key rather than writing
    /// that script twice — the answer `⌃7` gets on a page with no seventh slot.
    /// Letting it through instead would drop a raw space into a document that
    /// still has a composition marked in it.
    ///
    /// Driven through the real production source of a hanji-less candidate: the
    /// §34 literal-romanization candidate, which the engine prepends at index 0
    /// when the setting is on, and which the bar opens highlighted.
    func testSpace_onASingleScriptCandidate_writesNothing() throws {
        try withSetting(SettingsStore.Keys.isLiteralRomanCandidateEnabled.name, to: true) {
            let session = try composedSession()
            let leading = try XCTUnwrap(session.presenter.shownContent).cells[0]
            XCTAssertNil(leading.annotation, "the literal candidate has no second script to offer")
            session.client.clearWrites()

            _ = try session.controller.handle(
                TestFixtures.keyDownEvent(characters: " "), client: session.client,
            )

            XCTAssertTrue(session.client.insertedTexts.isEmpty)
        }
    }

    // MARK: - No selection mode

    /// `↓` walks into the list and nothing more (USER 2026-08-28, retiring the
    /// latch that made a bare digit pick after it): the `1` that follows is
    /// still the tone of the syllable, and the hint has not moved.
    func testDownArrow_doesNotTurnABareDigitIntoAPick() throws {
        for keySet in CandidateSlotKeySet.allCases {
            let session = try composedSession(under: keySet)
            session.press(.downArrow)
            XCTAssertEqual(try XCTUnwrap(session.presenter.shownContent).slotKeySet, keySet)
            session.client.clearWrites()

            _ = try session.controller.handle(
                TestFixtures.keyDownEvent(characters: "1"), client: session.client,
            )

            XCTAssertEqual(
                session.client.insertedTexts.last, nil,
                "`taigi` + `1` tones the last syllable under \(keySet) — nothing is written",
            )
        }
    }

    /// Walking the bar is how a Taigi typist LOOKS at the homophones before
    /// deciding which tone to add — a glance, not a mode change.
    func testWalkingTheBar_leavesTheDigitsAsTones() throws {
        let session = try composedSession()

        _ = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: "\t"), client: session.client,
        )

        XCTAssertEqual(try XCTUnwrap(session.presenter.shownContent).slotKeySet, .bareKeys)
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

    /// ⇥ walks the bar rather than committing from it, pairing with the ⇧⇥ that
    /// already walked back (`ComposingAction.nextCandidate`). Space held this
    /// job until 2026-08-25, when it became the 漢羅 key.
    func testTab_walksToTheNextCandidateWithoutCommitting() throws {
        let session = try composedSession()
        let cells = try XCTUnwrap(session.presenter.shownContent).cells
        session.client.clearWrites()

        let handled = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: "\t"),
            client: session.client,
        )

        XCTAssertTrue(handled)
        XCTAssertTrue(session.client.insertedTexts.isEmpty, "walking the bar writes nothing")
        XCTAssertEqual(session.presenter.selectedIndex, 1)
        XCTAssertEqual(try XCTUnwrap(session.presenter.shownContent).cells, cells)
    }

    /// Each key that names the second slot commits the second candidate,
    /// under the set that holds it: the shipped bare key, `⇧2`, `⌃2`.
    func testEverySlotKey_commitsThatSlotOfTheVisiblePage() throws {
        let keys: [(name: String, keySet: CandidateSlotKeySet, event: () throws -> NSEvent)] = [
            ("w", .bareKeys, { try TestFixtures.keyDownEvent(characters: "w") }),
            ("⇧2", .shift, { try TestFixtures.shiftedDigitKeyDownEvent(slot: 1) }),
            ("⌃2", .control, { try Self.controlDigitEvent(slot: 1) }),
        ]
        for (name, keySet, event) in keys {
            try withSlotKeySet(keySet) {
                let session = try composedSession()
                let secondCell = try XCTUnwrap(session.presenter.shownContent).cells[1].text
                session.client.clearWrites()

                let handled = try session.controller.handle(event(), client: session.client)

                XCTAssertTrue(handled, name)
                XCTAssertEqual(session.client.insertedTexts.last, secondCell, name)
            }
        }
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
                TestFixtures.keyDownEvent(characters: CandidateSlotKeySet.bareKeyRow[slot]),
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

    /// Switching romanization has to take the bar with it. The candidates on
    /// screen were fetched under the old romanization, and Space commits
    /// whichever one is highlighted — a bar left standing would write a
    /// candidate the new mode would never have offered.
    ///
    /// Driven through `performShortcutAction`, the path the ⌃⌘R hotkey takes —
    /// the input-source menu no longer carries the switch (USER 2026-08-21) —
    /// against a real composition: asserting only that a hide was requested
    /// would pass against a bar that stayed up because the hide named the
    /// wrong session.
    func testSwitchingRomanization_takesTheBarDown() throws {
        let session = try composedSession()
        session.controller.settings = try makeScratchSettingsStore()
        XCTAssertTrue(session.presenter.isShowing, "the composition put a bar up to take down")

        session.controller.performShortcutAction(.toggleRomanization)

        XCTAssertFalse(session.presenter.isShowing)
        XCTAssertEqual(session.controller.settings.inputMode, .poj)
    }

    /// The 漢羅 swap keeps the bar UP: it changes how a candidate displays and
    /// commits, never which candidates exist — dismissing here read as the
    /// window vanishing on the hotkey (real device, 2026-08-21). The same list
    /// re-renders with the scripts flipped, and the selection keeps its index.
    /// Saves and restores the REAL stored swap value around `body`: the toggle
    /// must land in the same defaults domain the manager's settings provider
    /// reads — `.standard` — so a suite cannot stand in, and a flip left
    /// behind would fail unrelated cases on the NEXT run (the
    /// `savedShortcuts` pattern).
    private func withRestoredSwapSetting(_ body: () throws -> Void) rethrows {
        try withSetting(SettingsStore.Keys.isTranslateSwapped.name, to: nil, body)
    }

    /// Runs `body` with `key` restored afterwards to whatever it held —
    /// including "held nothing", which a bare `removeObject` would turn into a
    /// value a later case never chose. `.standard` rather than a scratch suite
    /// because the controller and the shared coordinator's engine must read ONE
    /// domain for these cases to mean anything (see `withRestoredSwapSetting`'s
    /// callers).
    /// The slot key set the sessions inside `body` read, put back afterwards.
    private func withSlotKeySet(_ keySet: CandidateSlotKeySet, _ body: () throws -> Void) rethrows {
        try withSetting(SettingsStore.Keys.candidateSlotModifier.name, to: keySet.rawValue, body)
    }

    private func withSetting(
        _ key: String,
        to value: Any?,
        _ body: () throws -> Void,
    ) rethrows {
        let saved = UserDefaults.standard.object(forKey: key)
        defer {
            if let saved {
                UserDefaults.standard.set(saved, forKey: key)
            } else {
                UserDefaults.standard.removeObject(forKey: key)
            }
        }
        if let value { UserDefaults.standard.set(value, forKey: key) }
        try body()
    }

    func testTogglingTranslateSwapped_rerendersTheBarInPlace() throws {
        try withRestoredSwapSetting { try togglingTranslateSwappedRerendersTheBarInPlace() }
    }

    private func togglingTranslateSwappedRerendersTheBarInPlace() throws {
        let session = try composedSession()
        _ = try session.controller.handle(Self.arrowEvent(.rightArrow), client: session.client)
        let before = try XCTUnwrap(session.presenter.shownContent).cells
        let keptIndex = session.presenter.selectedIndex
        let swappedBefore = session.controller.settings.isTranslateSwapped
        let callsBefore = session.presenter.calls.count

        session.controller.performShortcutAction(.toggleTranslateSwapped)

        XCTAssertEqual(session.controller.settings.isTranslateSwapped, !swappedBefore)
        XCTAssertTrue(session.presenter.isShowing, "the bar must stay up across a display-only flip")
        XCTAssertFalse(
            session.presenter.calls.dropFirst(callsBefore)
                .contains { if case .hide = $0 { true } else { false } },
            "a swap must not route through dismissal",
        )
        XCTAssertEqual(session.presenter.selectedIndex, keptIndex)
        let after = try XCTUnwrap(session.presenter.shownContent).cells
        XCTAssertEqual(after.count, before.count)
        for (befores, afters) in zip(before, after) where befores.annotation != nil {
            XCTAssertEqual(afters.text, befores.annotation, "primary and annotation must flip")
            XCTAssertEqual(afters.annotation, befores.text)
        }
    }

    /// With a composition but no bar on screen, the swap flips the setting and
    /// nothing else — no window may appear from a hotkey.
    func testTogglingTranslateSwapped_withNoBar_showsNothing() throws {
        try withRestoredSwapSetting {
            let session = try composedSession()
            session.controller.hidePalettes()
            XCTAssertFalse(session.presenter.isShowing)
            let callsBefore = session.presenter.calls.count

            session.controller.performShortcutAction(.toggleTranslateSwapped)

            XCTAssertFalse(session.presenter.isShowing)
            XCTAssertEqual(
                session.presenter.calls.count, callsBefore,
                "a hotkey with no bar on screen must not touch the presenter",
            )
        }
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
    /// A session under `keySet`, which stays in force for the whole case —
    /// every keystroke re-reads it — and is put back at teardown.
    private func composedSession(under keySet: CandidateSlotKeySet) throws -> Session {
        let key = SettingsStore.Keys.candidateSlotModifier.name
        let saved = UserDefaults.standard.object(forKey: key)
        addTeardownBlock {
            if let saved {
                UserDefaults.standard.set(saved, forKey: key)
            } else {
                UserDefaults.standard.removeObject(forKey: key)
            }
        }
        UserDefaults.standard.set(keySet.rawValue, forKey: key)
        return try composedSession()
    }

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
