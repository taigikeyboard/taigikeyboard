// Pins the host writes of an English-mode suggestion tap (issue #826): the pick
// replaces the word fragment before the caret. Android parity:
// `CandidateClickHandler.handleEnglishCandidateClick`.

@testable import TaigiKeyboard
import XCTest

@MainActor
final class ActionHandlerEnglishPickTests: XCTestCase {
    func testWriteEnglishPick_replacesTypedWord() {
        let proxy = proxy(before: "Win")

        ActionHandler.writeEnglishPick("Windows", to: proxy)

        XCTAssertEqual(proxy.calls, [.deleteBackward, .deleteBackward, .deleteBackward, .insert("Windows")])
    }

    func testWriteEnglishPick_replacesOnlyLastWord() {
        let proxy = proxy(before: "hello Win")

        ActionHandler.writeEnglishPick("Windows", to: proxy)

        XCTAssertEqual(proxy.calls, [.deleteBackward, .deleteBackward, .deleteBackward, .insert("Windows")])
    }

    /// Space leaves the strip on screen; a tap then must not eat the word
    /// before the space.
    func testWriteEnglishPick_afterSpace_insertsWithoutDeleting() {
        let proxy = proxy(before: "Win ")

        ActionHandler.writeEnglishPick("Windows", to: proxy)

        XCTAssertEqual(proxy.calls, [.insert("Windows")])
    }

    /// Caret mid-word: only the part before the caret is replaced, as on Android.
    func testWriteEnglishPick_caretMidWord_keepsTextAfterCaret() {
        let proxy = proxy(before: "Wi", after: "n")

        ActionHandler.writeEnglishPick("Windows", to: proxy)

        XCTAssertEqual(proxy.calls, [.deleteBackward, .deleteBackward, .insert("Windows")])
    }

    /// A host that reports no context still gets the pick.
    func testWriteEnglishPick_noContext_inserts() {
        let proxy = proxy(before: nil)

        ActionHandler.writeEnglishPick("Windows", to: proxy)

        XCTAssertEqual(proxy.calls, [.insert("Windows")])
    }

    private func proxy(before: String?, after: String? = nil) -> RecordingTextDocumentProxy {
        let proxy = RecordingTextDocumentProxy()
        proxy.textBeforeCaret = before
        proxy.textAfterCaret = after
        return proxy
    }
}
