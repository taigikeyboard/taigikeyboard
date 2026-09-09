// The symbol picker end to end: the chord that opens it, the keys that walk
// and pick, what a pick writes, and who takes it down.

import Carbon.HIToolbox
import InputMethodKit
import KeyboardShortcuts
@testable import TaigiInputMethodCore
import XCTest

/// Drives the real controller and the real engine against a recording
/// picker presenter. What is pinned is the routing — when the list goes up
/// and comes down, which key reaches it, what lands in the document — not
/// how the window draws.
@MainActor
final class TaigiInputControllerSymbolPickerTests: XCTestCase {
    private var suiteName = ""
    private var userDefaults = UserDefaults.standard
    /// What the machine running these tests had recorded on the picker row.
    /// The library writes to `UserDefaults.standard` and takes no suite, so
    /// the row is pinned to its default for the case and handed back after.
    private var savedPickerShortcut: KeyboardShortcuts.Shortcut?

    override func setUpWithError() throws {
        try super.setUpWithError()
        InstalledLexicon.installOnce()
        suiteName = "TaigiInputControllerSymbolPickerTests.\(UUID().uuidString)"
        userDefaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
        savedPickerShortcut = KeyboardShortcuts.getShortcut(for: .showSymbolPicker)
        KeyboardShortcuts.setShortcut(ShortcutAction.showSymbolPicker.defaultShortcut, for: .showSymbolPicker)
    }

    override func tearDown() {
        KeyboardShortcuts.setShortcut(savedPickerShortcut, for: .showSymbolPicker)
        userDefaults.removePersistentDomain(forName: suiteName)
        super.tearDown()
    }

    // MARK: - The chord

    func testTheChord_opensTheCategoryList_anchoredAtTheCaret() throws {
        let session = try makeSession()

        let handled = try session.pressPickerChord()

        XCTAssertTrue(handled)
        XCTAssertEqual(session.controller.symbolPickerLevel, .categories)
        let content = try XCTUnwrap(session.picker.shownContent)
        XCTAssertEqual(content.cells.map(\.text), ["標點符號", "括號", "特殊符號"])
        XCTAssertEqual(content.slotKeySet, .bareKeys, "the picker is picked with the bar's own keys")
        XCTAssertFalse(content.leadCellIsUnkeyed)
        XCTAssertEqual(session.picker.calls.last, .show(content, caretRect: Self.caretRect))
        XCTAssertEqual(session.client.writes, [], "opening writes nothing")
    }

    func testTheChordAgain_closesIt() throws {
        let session = try makeSession()
        try session.pressPickerChord()

        let handled = try session.pressPickerChord()

        XCTAssertTrue(handled)
        XCTAssertFalse(session.picker.isShowing)
        XCTAssertNil(session.controller.symbolPickerLevel)
    }

    /// A held chord is the keyboard's auto-repeat, and one press: a picker
    /// that toggled on every repeat would flicker and land wherever the key
    /// happened to come up.
    func testAHeldChord_isOnePress() throws {
        let session = try makeSession()
        try session.pressPickerChord()

        let handled = try session.pressPickerChord(isARepeat: true)

        XCTAssertTrue(handled, "the repeat is still the picker's key")
        XCTAssertTrue(session.picker.isShowing)
    }

    /// A user who moved the row keeps their key, and the shipped one goes
    /// back to the host. Read at activation, which recording a chord in the
    /// settings window always precedes on the way back to the document.
    func testTheRecordedChord_isTheOneRead() throws {
        KeyboardShortcuts.setShortcut(.init(.period, modifiers: [.control, .command]), for: .showSymbolPicker)
        let session = try makeSession()

        XCTAssertFalse(try session.pressPickerChord(), "the shipped chord is no longer the picker's")
        XCTAssertFalse(session.picker.isShowing)

        let handled = try session.controller.handle(
            TestFixtures.keyDownEvent(
                characters: ".", modifiers: [.control, .command], keyCode: UInt16(kVK_ANSI_Period),
            ),
            client: session.client,
        )
        XCTAssertTrue(handled)
        XCTAssertTrue(session.picker.isShowing)
    }

    /// A client that cannot say where its caret is cannot host the picker,
    /// for the reason it cannot host the bar. The chord is still consumed.
    func testAClientWithNoCaret_opensNothing() throws {
        let session = try makeSession(caretRects: [:])

        let handled = try session.pressPickerChord()

        XCTAssertTrue(handled)
        XCTAssertFalse(session.picker.isShowing)
        XCTAssertNil(session.controller.symbolPickerLevel)
    }

    /// A list the window refused to put up — a caret on no display — leaves
    /// no level behind: a picker recorded as open over nothing would go on
    /// swallowing the slot keys until the user found Escape.
    func testAListTheWindowRefused_leavesNoPickerBehind() throws {
        let picker = RecordingCandidatePresenter()
        picker.refusesToShow = true
        let session = try makeSession(picker: picker)

        try session.pressPickerChord()

        XCTAssertNil(session.controller.symbolPickerLevel)
        XCTAssertTrue(try session.type("q"), "q starts a composition, as it does with no picker")
        XCTAssertEqual(session.client.writes, [.setMarkedText("q", selectionLocation: 1)])
    }

    /// The picker is not the candidate window: switching that one off does
    /// not take the symbols away.
    func testTheCandidateWindowToggle_doesNotGateThePicker() throws {
        let session = try makeSession()
        userDefaults.set(false, forKey: SettingsStore.Keys.isCandidateWindowEnabled.name)

        try session.pressPickerChord()

        XCTAssertTrue(session.picker.isShowing)
    }

    // MARK: - Walking and picking

    func testASlotKey_descendsIntoTheCategory() throws {
        let session = try makeSession()
        try session.pressPickerChord()

        try session.type("w")

        XCTAssertEqual(session.controller.symbolPickerLevel, .items(.brackets))
        XCTAssertEqual(
            try XCTUnwrap(session.picker.shownContent).cells.map(\.text),
            try XCTUnwrap(TestFixtures.shippedSymbolTable().category(.brackets)).symbols,
        )
        XCTAssertEqual(session.picker.selectedIndex, 0, "a fresh list selects its first cell")
        XCTAssertEqual(session.client.writes, [])
    }

    func testASlotKeyOnTheSymbols_writesItAndCloses() throws {
        let session = try makeSession()
        try session.pressPickerChord()
        try session.type("q")

        try session.type("q")

        XCTAssertEqual(session.client.insertedTexts, ["，"])
        XCTAssertFalse(session.picker.isShowing)
        XCTAssertNil(session.controller.symbolPickerLevel)
    }

    /// One pick, both halves (USER 2026-09-09) — and one write, so the pair
    /// cannot be split by anything the host does between two inserts.
    func testABracketPair_isOneWrite() throws {
        let session = try makeSession()
        try session.pressPickerChord()
        try session.type("w")

        try session.type("q")

        XCTAssertEqual(session.client.writes, [.insertText("「」")])
    }

    func testReturn_writesTheHighlightedSymbol() throws {
        let session = try makeSession()
        try session.pressPickerChord()
        try session.type("q")

        try session.type("\t")
        try session.type("\r")

        XCTAssertEqual(session.client.insertedTexts, ["。"])
        XCTAssertFalse(session.picker.isShowing)
    }

    /// The arrows walk the list, and walking writes nothing.
    func testTheArrows_walkTheList() throws {
        let session = try makeSession()
        try session.pressPickerChord()
        try session.type("q")

        try session.press(.rightArrow)
        try session.press(.rightArrow)

        XCTAssertEqual(session.picker.selectedIndex, 2)
        XCTAssertEqual(session.client.writes, [])
    }

    /// Escape closes from either level (USER 2026-09-09) and is swallowed:
    /// the host must not see the Escape that closed a list of ours.
    func testEscape_closesFromEitherLevel_andIsSwallowed() throws {
        let session = try makeSession()
        try session.pressPickerChord()
        try session.type("q")
        XCTAssertEqual(session.controller.symbolPickerLevel, .items(.punctuation))

        XCTAssertTrue(try session.type("\u{1B}"))
        XCTAssertNil(session.controller.symbolPickerLevel)
        XCTAssertFalse(session.picker.isShowing)

        try session.pressPickerChord()
        XCTAssertTrue(try session.type("\u{1B}"))
        XCTAssertNil(session.controller.symbolPickerLevel)
        XCTAssertEqual(session.client.writes, [])
    }

    /// A letter is not the picker's: the list comes down and the letter goes
    /// on to start the composition it would have started anyway.
    func testALetter_closesThePicker_andStartsComposing() throws {
        let session = try makeSession()
        try session.pressPickerChord()

        let handled = try session.type("t")

        XCTAssertTrue(handled)
        XCTAssertFalse(session.picker.isShowing)
        XCTAssertEqual(session.client.writes, [.setMarkedText("t", selectionLocation: 1)])
    }

    /// A pick is what the user chose, not what the full-width map would
    /// make of the key that typed it: `()` stays `()` under 漢字優先 too.
    func testAPick_bypassesTheFullWidthMap() throws {
        let session = try makeSession()
        session.controller.settings.storedIsTranslateSwapped = true
        try session.pressPickerChord()
        try session.type("w")
        let asciiParentheses = try XCTUnwrap(
            TestFixtures.shippedSymbolTable().category(.brackets)?.symbols.firstIndex(of: "()"),
        )

        for _ in 0 ..< asciiParentheses {
            try session.type("\t")
        }
        try session.type("\r")

        XCTAssertEqual(session.client.insertedTexts, ["()"])
    }

    // MARK: - Mid-composition

    /// Commit first, then open (vChewing's order): the symbol must land after
    /// the word, not inside a composition still marked in the document.
    func testMidComposition_theChordCommitsWhatIsHighlighted_thenOpens() throws {
        let session = try composedSession()
        XCTAssertTrue(session.presenter.isShowing)

        try session.pressPickerChord()

        XCTAssertEqual(session.client.insertedTexts, [Self.composition], "Return's commit: the highlighted literal")
        XCTAssertFalse(session.presenter.isShowing, "the bar went with the composition")
        XCTAssertTrue(session.picker.isShowing)

        try session.type("q")
        try session.type("q")
        XCTAssertEqual(session.client.insertedTexts, [Self.composition, "，"])
    }

    /// With no bar to take a highlight from — the candidate window switched
    /// off — the composition is committed as typed, the way ⇧Return would.
    func testMidComposition_withNoBar_theChordCommitsAsTyped_thenOpens() throws {
        userDefaults.set(false, forKey: SettingsStore.Keys.isCandidateWindowEnabled.name)
        let session = try composedSession()
        XCTAssertFalse(session.presenter.isShowing)

        try session.pressPickerChord()

        XCTAssertEqual(session.client.insertedTexts, [Self.composition])
        XCTAssertTrue(session.picker.isShowing)
    }

    /// A highlight that only NAILS a segment leaves the composition running,
    /// and the picker does not open over it: the next key still belongs to
    /// the composition's tail.
    func testMidComposition_aNailingCommit_keepsComposing_andOpensNothing() throws {
        // Which cell nails is the dictionary's business, so it is searched
        // for — one fresh session per cell, since choosing one changes state
        // (`TaigiInputControllerCandidateTests` searches the same way).
        var nailed: Session?
        for cell in 0 ..< HorizontalPageLayout.pageSize where nailed == nil {
            let session = try composedSession()
            guard try cell < XCTUnwrap(session.presenter.shownContent).cells.count else { break }
            try session.walk(cells: cell)

            try session.pressPickerChord()

            if session.client.insertedTexts.isEmpty, session.presenter.isShowing {
                nailed = session
            }
        }

        let session = try XCTUnwrap(nailed, "taigi must offer a candidate shorter than the whole buffer")
        XCTAssertFalse(session.picker.isShowing)
        XCTAssertNil(session.controller.symbolPickerLevel)
    }

    /// A picked attaching mark swaps with the auto space the commit left, as
    /// a typed one would (`guá ` + `，` → `guá，`) — walking the list in
    /// between does not spend the arm, since it touched neither the document
    /// nor the caret.
    func testAPickedAttachingMark_swapsWithTheAutoSpace() throws {
        let session = try composedSession()
        session.controller.settings.isAutoSpaceEnabled = true
        session.client.documentTextForReads = ""
        session.client.selectedRangeToReturn = NSRange(location: 0, length: 0)

        try session.pressPickerChord()
        XCTAssertEqual(session.client.documentTextForReads, "\(Self.composition) ")

        try session.type("q")
        try session.press(.rightArrow)
        try session.press(.leftArrow)
        try session.type("q")

        XCTAssertEqual(session.client.documentTextForReads, "\(Self.composition)， ")
    }

    /// The chord after an ordinary commit does not spend the arm either: the
    /// chord itself writes nothing, so the space Return left is still the one
    /// a picked `，` swaps with.
    func testTheChordAfterAReturnCommit_keepsTheAutoSpaceArm() throws {
        let session = try composedSession()
        session.controller.settings.isAutoSpaceEnabled = true
        session.client.documentTextForReads = ""
        session.client.selectedRangeToReturn = NSRange(location: 0, length: 0)
        try session.type("\r")
        XCTAssertEqual(session.client.documentTextForReads, "\(Self.composition) ")

        try session.pressPickerChord()
        try session.pressPickerChord(isARepeat: true)
        try session.type("q")
        try session.type("q")

        XCTAssertEqual(session.client.documentTextForReads, "\(Self.composition)， ")
    }

    // MARK: - Who takes it down

    func testHidePalettes_closesThePicker() throws {
        let session = try makeSession()
        try session.pressPickerChord()

        session.controller.hidePalettes()

        XCTAssertFalse(session.picker.isShowing)
        XCTAssertNil(session.controller.symbolPickerLevel)
    }

    /// A Carbon chord bypasses the key path that would take the picker down,
    /// so the action does it itself.
    func testAnotherGlobalAction_closesThePicker() throws {
        let session = try makeSession()
        try session.pressPickerChord()

        session.controller.performShortcutAction(.toggleRomanization)

        XCTAssertFalse(session.picker.isShowing)
        XCTAssertNil(session.controller.symbolPickerLevel)
        XCTAssertEqual(session.controller.settings.inputMode, .poj, "the switch itself still ran")
    }

    /// The settings doorway never reaches the key path, and the window it
    /// opens taking focus does not guarantee this session ends — so the
    /// doorway takes the picker down itself.
    func testOpeningSettings_closesThePicker() throws {
        let session = try makeSession()
        session.controller.settingsPresenterOverride = {}
        try session.pressPickerChord()

        session.controller.showPreferences(nil)

        XCTAssertFalse(session.picker.isShowing)
        XCTAssertNil(session.controller.symbolPickerLevel)
    }

    /// IMK activates the incoming session before it deactivates the outgoing
    /// one, so the outgoing session's teardown must not close a picker the
    /// arriving session raised — the bar's rule, on the picker's own owner.
    func testDeactivate_ofAnotherSession_leavesTheLiveSessionsPickerAlone() throws {
        let picker = RecordingCandidatePresenter()
        let leaving = try makeSession(picker: picker)
        let arriving = try makeSession(picker: picker)
        try arriving.pressPickerChord()

        leaving.controller.deactivateServer(leaving.client)

        XCTAssertTrue(picker.isShowing, "a session that did not raise the picker took it down")

        arriving.controller.deactivateServer(arriving.client)

        XCTAssertFalse(picker.isShowing, "the picker goes with the focus of the session that raised it")
    }

    // MARK: - Harness

    private struct Session: CandidateBarSession {
        let controller: TaigiInputController
        let client: RecordingTextInputClient
        /// The composing bar.
        let presenter: RecordingCandidatePresenter
        let picker: RecordingCandidatePresenter

        @MainActor
        @discardableResult
        func pressPickerChord(isARepeat: Bool = false) throws -> Bool {
            try controller.handle(
                TestFixtures.keyDownEvent(
                    characters: ",", modifiers: [.control, .command],
                    keyCode: UInt16(kVK_ANSI_Comma), isARepeat: isARepeat,
                ),
                client: client,
            )
        }
    }

    /// An activated session that has typed nothing yet, reading its settings
    /// from this case's own suite and the repository's symbol table.
    private func makeSession(
        picker: RecordingCandidatePresenter = RecordingCandidatePresenter(),
        caretRects: [Int: CGRect]? = nil,
    ) throws -> Session {
        let client = RecordingTextInputClient()
        client.caretRects = caretRects ?? [0: Self.caretRect, Self.compositionCaretIndex: Self.caretRect]
        let controller = try TestFixtures.makeInputController()
        controller.settings = SettingsStore(userDefaults: userDefaults)
        controller.displayLanguageOverride = TestFixtures.makeDisplayLanguageStore(.hanji, userDefaults: userDefaults)
        controller.symbolTableOverride = try TestFixtures.shippedSymbolTable()
        let bar = RecordingCandidatePresenter()
        controller.candidatePresenter = bar
        controller.symbolPickerPresenter = picker
        controller.activateServer(client)
        return Session(controller: controller, client: client, presenter: bar, picker: picker)
    }

    /// An activated session that has typed `taigi`, so a bar is up.
    private func composedSession() throws -> Session {
        let session = try makeSession()
        try session.type(Self.composition)
        return session
    }

    private static let composition = "taigi"
    private static let compositionCaretIndex = composition.utf16.count - 1
    private static let caretRect = CGRect(x: 120, y: 400, width: 1, height: 18)
}
