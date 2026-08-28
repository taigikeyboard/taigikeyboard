// What a bare digit means mid-composition: the tone of the syllable being
// typed, or — once no tone can follow — the candidate at that slot.

import AppKit
@testable import TaigiInputMethodCore
import XCTest

/// The rule is TL/POJ grammar rather than a mode: a digit is romanization
/// exactly while the raw buffer ends in a letter. These cases walk both sides
/// of that line, and the transitions between them.
final class BareDigitSelectionTests: XCTestCase {
    private func intent(
        _ characters: String,
        modifiers: NSEvent.ModifierFlags = [],
        isComposing: Bool = true,
        isShowingCandidates: Bool = true,
        keySet: CandidateSlotKeySet = .bareKeys,
        rawInput: String,
    ) throws -> ComposingKeyIntent {
        let event = try TestFixtures.keyDownEvent(characters: characters, modifiers: modifiers)
        return ComposingKeyIntent.intent(
            for: KeyEventSnapshot(event),
            isComposing: isComposing,
            isShowingCandidates: isShowingCandidates,
            bindings: ComposingKeyBindings(slotKeySet: keySet),
            rawInput: rawInput,
        )
    }

    // MARK: - The grammar rule itself

    func testADigitIsRomanization_exactlyWhileTheBufferEndsInALetter() {
        // The letter tails include the spellings POJ types as digraphs — `o͘`
        // is keyed `oo`/`ou` and the nasal `ⁿ` as `nn`, so the character
        // before the tone is ASCII either way — plus the 輕聲 marker's own
        // syllable (`--a`) and uppercase, which `isLetter` covers.
        for typed in ["tai", "taigi", "tai5-gi", "a", "", "chhoo", "kinn", "--a", "TAI", "xyz"] {
            XCTAssertTrue(
                ComposingKeyIntent.canTypeToneDigit(after: typed),
                "'\(typed)' can still take a tone digit",
            )
        }
        // A syllable carries at most one trailing tone digit, and both the
        // hyphen and the 輕聲 `--` are followed by a syllable, which starts
        // with a letter.
        for typed in ["tai5", "tai5-", "tai-", "tai52", "--", "tai5-gi2"] {
            XCTAssertFalse(
                ComposingKeyIntent.canTypeToneDigit(after: typed),
                "nothing TL or POJ spells continues '\(typed)' with a digit",
            )
        }
    }

    func testAnInvalidSyllableEndingInALetter_keepsTheRomanizationAffordance() throws {
        // The composing alphabet takes every ASCII letter so a custom-dictionary
        // romanization can be typed; the engine keeps `xyz2` verbatim rather
        // than toning it. The rule only promises the other direction.
        XCTAssertEqual(try intent("2", rawInput: "xyz"), .input("2"))
    }

    func testTheKhinsiannMarker_selectsRatherThanTones() throws {
        // `--` opens a 輕聲 syllable (§21), which starts with a letter — so a
        // digit typed here is not romanization either.
        XCTAssertEqual(try intent("2", rawInput: "--"), .selectCandidateSlot(1))
    }

    func testAToneAfterAHyphenatedSecondSyllable_selects() throws {
        XCTAssertEqual(try intent("3", rawInput: "tai5-gi2"), .selectCandidateSlot(2))
    }

    func testEveryToneDigit_isInputWhileTheTailIsALetter() throws {
        for digit in ["1", "2", "3", "4", "5", "6", "7", "8", "9"] {
            XCTAssertEqual(try intent(digit, rawInput: "tai"), .input(digit), "tone \(digit)")
        }
        // 0 is not a tone the engine converts (`syllable.rs:105` reads 1…9),
        // but the composing alphabet has always passed every digit through
        // while the tail is a letter, and this change does not narrow that:
        // the engine keeps `tai0` verbatim, as it always did.
        XCTAssertEqual(try intent("0", rawInput: "tai"), .input("0"))
    }

    // MARK: - Still romanization

    func testATonelessSyllable_takesTheDigitAsItsTone() throws {
        // trace: raw "tai" ends in a letter → `tai` + `5` is `tâi`, and the
        // candidate list filters to the fifth tone. The toneless affordance.
        XCTAssertEqual(try intent("5", rawInput: "tai"), .input("5"))
    }

    func testAContinuousComposition_takesTheDigitAsTheLastSyllablesTone() throws {
        XCTAssertEqual(try intent("2", rawInput: "tai5gi"), .input("2"))
    }

    func testAHyphenatedSyllableInProgress_stillTakesATone() throws {
        XCTAssertEqual(try intent("2", rawInput: "tai5-gi"), .input("2"))
    }

    // MARK: - Now a selection key

    func testAfterATone_aBareDigitSelectsThatSlot() throws {
        XCTAssertEqual(try intent("2", rawInput: "tai5"), .selectCandidateSlot(1))
        XCTAssertEqual(try intent("1", rawInput: "tai5"), .selectCandidateSlot(0))
        XCTAssertEqual(try intent("9", rawInput: "tai5"), .selectCandidateSlot(8))
    }

    func testAfterAHyphen_aBareDigitSelects() throws {
        // A hyphen is followed by a new syllable, which starts with a letter —
        // so a digit here was never going to be input either.
        XCTAssertEqual(try intent("2", rawInput: "tai5-"), .selectCandidateSlot(1))
    }

    func testZeroAfterATone_isDocumentTextRatherThanASlot() throws {
        // The slots are 1–9; 0 names none of them, and it cannot be a tone
        // here, so it ends the composition as the digit the user typed.
        XCTAssertEqual(try intent("0", rawInput: "tai5"), .commitThenInsert("0"))
    }

    func testWithNoBarUp_aDigitThatCannotBeAToneIsDocumentText() throws {
        // Nothing to select, so the key keeps the meaning it has always had.
        XCTAssertEqual(
            try intent("2", isShowingCandidates: false, rawInput: "tai5"),
            .commitThenInsert("2"),
        )
    }

    func testOutsideAComposition_digitsStayTheHosts() throws {
        XCTAssertEqual(
            try intent("2", isComposing: false, isShowingCandidates: false, rawInput: ""),
            .passThrough,
        )
    }

    // MARK: - The tiers around it

    func testTheModifierSlotChords_workOnBothSidesOfTheRule() throws {
        // ⌃2 selects whether or not a tone could still be typed — it is the
        // path for the toneless composition, where a bare digit is a tone.
        XCTAssertEqual(
            try intent("2", modifiers: .control, keySet: .control, rawInput: "tai"),
            .selectCandidateSlot(1),
        )
        XCTAssertEqual(
            try intent("2", modifiers: .control, keySet: .control, rawInput: "tai5"),
            .selectCandidateSlot(1),
        )
    }

    func testCapsLockLatched_aBareDigitStillSelects() throws {
        // Caps Lock is a latched state, not a held chord.
        XCTAssertEqual(
            try intent("2", modifiers: .capsLock, rawInput: "tai5"),
            .selectCandidateSlot(1),
        )
    }

    // MARK: - End to end, including the way back

    @MainActor
    func testTypingAToneThenADigit_commitsTheCandidateAtThatSlot() throws {
        let session = try makeSession()
        try type("tai5", into: session)
        // Read off the bar the user is looking at, so the assertion names the
        // candidate that slot actually holds rather than a hard-coded word.
        let secondCandidate = try XCTUnwrap(session.presenter.shownContent?.cells[1].text)

        let handled = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: "2"), client: session.client,
        )

        XCTAssertTrue(handled, "a bare 2 after a tone picks slot two")
        XCTAssertEqual(
            session.client.insertedTexts, [secondCandidate],
            "the pick commits the candidate in slot two",
        )
    }

    @MainActor
    func testBackspacingTheTone_putsTheDigitBackToBeingATone() throws {
        // The rule follows the buffer, so undoing the tone undoes the meaning:
        // this is what keeps the transition from reading as a sticky mode.
        let session = try makeSession()
        try type("tai5", into: session)
        _ = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: "\u{8}"), client: session.client,
        )
        session.client.clearWrites()

        _ = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: "2"), client: session.client,
        )

        XCTAssertEqual(
            session.client.writes.last,
            .setMarkedText("tái", selectionLocation: 3),
            "the digit is the tone again — got \(session.client.writes)",
        )
    }

    @MainActor
    func testATonelessComposition_takesTheDigitAsAToneEndToEnd() throws {
        let session = try makeSession()
        try type("tai", into: session)
        session.client.clearWrites()

        _ = try session.controller.handle(
            TestFixtures.keyDownEvent(characters: "5"), client: session.client,
        )

        XCTAssertEqual(
            session.client.writes.last,
            .setMarkedText("tâi", selectionLocation: 3),
            "got \(session.client.writes)",
        )
        XCTAssertTrue(session.client.insertedTexts.isEmpty, "nothing was committed")
    }

    // MARK: - Helpers

    private struct Session {
        let controller: TaigiInputController
        let client: RecordingTextInputClient
        let presenter: RecordingCandidatePresenter
    }

    @MainActor
    private func makeSession() throws -> Session {
        InstalledLexicon.installOnce()
        let client = RecordingTextInputClient()
        // Every index answers, so the caret walk finds a rectangle whatever
        // length the marked region has as the composition grows.
        client.caretRects = Dictionary(
            uniqueKeysWithValues: (0 ... 8).map { ($0, CGRect(x: 120, y: 400, width: 1, height: 18)) },
        )
        let controller = try TestFixtures.makeInputController()
        let presenter = RecordingCandidatePresenter()
        controller.candidatePresenter = presenter
        let store = try makeScratchSettingsStore()
        // Auto-space is orthogonal to what a digit means, and covered by its
        // own suite; off here so a commit is one write to assert on.
        store.isAutoSpaceEnabled = false
        controller.settings = store
        controller.activateServer(client)
        return Session(controller: controller, client: client, presenter: presenter)
    }

    @MainActor
    private func type(_ text: String, into session: Session) throws {
        for character in text.map(String.init) {
            _ = try session.controller.handle(
                TestFixtures.keyDownEvent(characters: character), client: session.client,
            )
        }
    }
}
