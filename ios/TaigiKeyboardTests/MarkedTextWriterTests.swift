@testable import TaigiKeyboard
import UIKit
import XCTest

/// Tests for `MarkedTextWriter` — the proxy call sequence behind the
/// composing effects. A commit over a marked region must replace and unmark
/// it, never `insertText` (issue #352: Flutter hosts kept the preedit).
final class MarkedTextWriterTests: XCTestCase {
    private var writer: MarkedTextWriter!
    private var proxy: RecordingTextDocumentProxy!

    override func setUp() {
        super.setUp()
        writer = MarkedTextWriter()
        proxy = RecordingTextDocumentProxy()
    }

    func testCommit_withoutMarkedText_insertsAtCaret() {
        writer.commit("，", on: proxy)

        XCTAssertEqual(proxy.calls, [.insert("，")])
    }

    func testCommit_overMarkedText_replacesThenUnmarksWithoutInsert() {
        writer.update("Taigi", on: proxy)
        writer.commit("Tâi-gí", on: proxy)

        XCTAssertEqual(proxy.calls, [
            .setMarked("Taigi", NSRange(location: 5, length: 0)),
            .setMarked("Tâi-gí", NSRange(location: 6, length: 0)),
            .unmark,
        ])
        XCTAssertFalse(writer.hasMarkedText)
    }

    func testCommit_afterCommit_insertsAtCaret() {
        writer.update("taigi", on: proxy)
        writer.commit("台語", on: proxy)
        proxy.calls.removeAll()

        writer.commit(" ", on: proxy)

        XCTAssertEqual(proxy.calls, [.insert(" ")])
    }

    func testCommit_emptyTextOverMarkedText_removesPreedit() {
        writer.update("taigi", on: proxy)
        proxy.calls.removeAll()

        writer.commit("", on: proxy)

        XCTAssertEqual(proxy.calls, [.setMarked("", NSRange(location: 0, length: 0)), .unmark])
    }

    func testUpdate_emptyText_leavesNoMarkedRegion() {
        writer.update("t", on: proxy)
        writer.update("", on: proxy)
        proxy.calls.removeAll()

        writer.commit("，", on: proxy)

        XCTAssertEqual(proxy.calls, [.insert("，")])
    }

    func testClear_removesMarkedTextWithoutCommitting() {
        writer.update("taigi", on: proxy)
        proxy.calls.removeAll()

        writer.clear(on: proxy)
        writer.commit("，", on: proxy)

        XCTAssertEqual(proxy.calls, [
            .setMarked("", NSRange(location: 0, length: 0)),
            .unmark,
            .insert("，"),
        ])
    }

    func testUpdate_caretCountsUTF16Units() {
        // trace: "𪜶" is U+2A736 — one Character, two UTF-16 units.
        writer.update("𪜶", on: proxy)

        XCTAssertEqual(proxy.calls, [.setMarked("𪜶", NSRange(location: 2, length: 0))])
    }
}

private final class RecordingTextDocumentProxy: NSObject, UITextDocumentProxy {
    enum Call: Equatable {
        case setMarked(String, NSRange)
        case unmark
        case insert(String)
    }

    var calls: [Call] = []

    var documentContextBeforeInput: String? {
        nil
    }

    var documentContextAfterInput: String? {
        nil
    }

    var selectedText: String? {
        nil
    }

    var documentInputMode: UITextInputMode? {
        nil
    }

    var documentIdentifier: UUID {
        UUID()
    }

    var hasText: Bool {
        false
    }

    func adjustTextPosition(byCharacterOffset _: Int) {}

    func setMarkedText(_ markedText: String, selectedRange: NSRange) {
        calls.append(.setMarked(markedText, selectedRange))
    }

    func unmarkText() {
        calls.append(.unmark)
    }

    func insertText(_ text: String) {
        calls.append(.insert(text))
    }

    func deleteBackward() {}
}
