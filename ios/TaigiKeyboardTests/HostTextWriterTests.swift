@testable import TaigiKeyboard
import UIKit
import XCTest

/// Tests for `HostTextWriter` — the proxy call sequence behind every
/// Taigi-controlled host write (issue #352). A commit over a marked region
/// must replace and unmark it, never `insertText`; inside an event the commit
/// and the literals after it must reach the host as one replacement.
final class HostTextWriterTests: XCTestCase {
    private var writer: HostTextWriter!
    private var proxy: RecordingTextDocumentProxy!
    private var clock: TimeInterval = 0

    override func setUp() {
        super.setUp()
        let proxy = RecordingTextDocumentProxy()
        self.proxy = proxy
        clock = 0
        writer = HostTextWriter(proxy: { proxy }, now: { [unowned self] in clock })
    }

    // MARK: - Outside an event

    func testHostTextWriter_commitWithoutMarkedText_insertsAtCaret() {
        writer.commit("，")

        XCTAssertEqual(proxy.calls, [.insert("，")])
    }

    func testHostTextWriter_commitOverMarkedText_replacesThenUnmarksWithoutInsert() {
        writer.update("Taigi")
        writer.commit("Tâi-gí")

        XCTAssertEqual(proxy.calls, [
            .setMarked("Taigi", NSRange(location: 5, length: 0)),
            .setMarked("Tâi-gí", NSRange(location: 6, length: 0)),
            .unmark,
        ])
        XCTAssertFalse(writer.hasMarkedText)
    }

    func testHostTextWriter_commitEmptyTextOverMarkedText_removesPreedit() {
        writer.update("taigi")
        proxy.calls.removeAll()

        writer.commit("")

        XCTAssertEqual(proxy.calls, [.setMarked("", NSRange(location: 0, length: 0)), .unmark])
    }

    func testHostTextWriter_updateEmptyText_leavesNoMarkedRegion() {
        writer.update("t")
        writer.update("")
        proxy.calls.removeAll()

        writer.commit("，")

        XCTAssertEqual(proxy.calls, [.insert("，")])
    }

    func testHostTextWriter_clear_removesMarkedTextWithoutCommitting() {
        writer.update("taigi")
        proxy.calls.removeAll()

        writer.clear()
        writer.commit("，")

        XCTAssertEqual(proxy.calls, [
            .setMarked("", NSRange(location: 0, length: 0)),
            .unmark,
            .insert("，"),
        ])
    }

    func testHostTextWriter_updateCaret_countsUTF16Units() {
        // trace: "𪜶" is U+2A736 — one Character, two UTF-16 units.
        writer.update("𪜶")

        XCTAssertEqual(proxy.calls, [.setMarked("𪜶", NSRange(location: 2, length: 0))])
    }

    // MARK: - Inside an event

    // INVARIANT_COMPOSING_HOST_COMMIT_ONE_WRITE_PER_EVENT (behavioral-invariants.md §13, issue #352)

    func testHostTextWriter_commitThenLiteralInEvent_writesOneReplacement() {
        // (preedit, commit, literals written after it, expected replacement)
        let cases: [(String, String, [String], String)] = [
            ("taigi", "台語", [], "台語"), // candidate tap, no auto space
            ("taigi", "taigi", [" "], "taigi "), // Space
            ("taigi", "tâi-gí", [" "], "tâi-gí "), // candidate + auto space
            ("taigi", "taigi", ["，"], "taigi，"), // punctuation while composing
            ("taigi", "taigi😀", [], "taigi😀"), // emoji / symbol (engine joins them)
        ]
        for (preedit, committed, literals, expected) in cases {
            writer.update(preedit)
            proxy.calls.removeAll()

            writer.beginEvent()
            writer.commit(committed)
            literals.forEach { writer.insert($0) }
            XCTAssertEqual(proxy.calls, [], "nothing reaches the host before the event ends: \(expected)")
            writer.endEvent()

            XCTAssertEqual(
                proxy.calls,
                [.setMarked(expected, NSRange(location: expected.utf16.count, length: 0)), .unmark],
                "one marked replacement for \(expected)",
            )
            XCTAssertFalse(writer.hasMarkedText)
        }
    }

    func testHostTextWriter_insertInEventWithoutCommit_writesImmediately() {
        writer.beginEvent()
        writer.insert(" ")
        writer.endEvent()

        XCTAssertEqual(proxy.calls, [.insert(" ")])
    }

    func testHostTextWriter_commitWithoutMarkedTextInEvent_insertsImmediately() {
        // A leading hyphen from Idle: the engine commits with no preedit,
        // then composition begins — the insert must land before the mark.
        writer.beginEvent()
        writer.commit("-")
        writer.update("a")
        writer.endEvent()

        XCTAssertEqual(proxy.calls, [.insert("-"), .setMarked("a", NSRange(location: 1, length: 0))])
    }

    func testHostTextWriter_endEventWithoutCommit_writesNothing() {
        writer.update("taigi")
        proxy.calls.removeAll()

        writer.beginEvent()
        writer.endEvent()

        XCTAssertEqual(proxy.calls, [])
        XCTAssertTrue(writer.hasMarkedText)
    }

    // MARK: - Echo of the keyboard's own commit

    /// Commits `我` over the preedit `gua` typed after `台語`, with `你好`
    /// after the caret.
    private func commitGuaAsWo() {
        proxy.textBeforeCaret = "台語"
        proxy.textAfterCaret = "你好"
        writer.update("gua")
        writer.beginEvent()
        writer.commit("我")
        writer.endEvent()
    }

    func testHostTextWriter_echoOfOwnCommit_firstCallbackInWindowWithCommitBeforeCaret() {
        commitGuaAsWo()
        clock = 0.06
        proxy.textBeforeCaret = "台語我"

        XCTAssertTrue(writer.isEchoOfOwnCommit(), "the commit's own echo")
        XCTAssertFalse(writer.isEchoOfOwnCommit(), "an echo is consumed once")
    }

    func testHostTextWriter_echoOfOwnCommit_rejectsRealEdits() {
        // (case, seconds after the commit, text before the caret, text after it)
        let cases: [(String, TimeInterval, String?, String?)] = [
            ("caret moved elsewhere", 0.06, "你", "你好"),
            ("nothing before the caret", 0.06, nil, "你好"),
            ("text before the commit edited", 0.06, "中文我", "你好"),
            ("text after the caret edited", 0.06, "台語我", "你"),
            ("callback long after the commit (host never echoed)", 5, "台語我", "你好"),
        ]
        for (name, elapsed, before, after) in cases {
            clock = 0
            commitGuaAsWo()
            clock = elapsed
            proxy.textBeforeCaret = before
            proxy.textAfterCaret = after

            XCTAssertFalse(writer.isEchoOfOwnCommit(), name)
        }
    }

    func testHostTextWriter_echoOfOwnCommit_onlyFirstCallbackIsChecked() {
        // Flutter never echoes: the next callback is a real edit, and a later
        // one that happens to look like the echo must not be swallowed.
        commitGuaAsWo()
        clock = 0.06
        proxy.textBeforeCaret = "你"
        XCTAssertFalse(writer.isEchoOfOwnCommit(), "real edit")

        proxy.textBeforeCaret = "台語我"
        XCTAssertFalse(writer.isEchoOfOwnCommit(), "an echo-like edit after it")
    }

    func testHostTextWriter_echoOfOwnCommit_neverMatchesWithoutAMarkedReplacement() {
        proxy.textBeforeCaret = "我"
        let steps: [(String, () -> Void)] = [
            ("insert", { self.writer.insert("我") }),
            ("commit without marked text", { self.writer.commit("我") }),
            ("clear", { self.writer.update("gua"); self.writer.clear() }),
        ]
        for (step, write) in steps {
            write()
            XCTAssertFalse(writer.isEchoOfOwnCommit(), step)
        }
    }

    func testHostTextWriter_insertAfterEvent_goesStraightToHost() {
        writer.update("taigi")
        writer.beginEvent()
        writer.commit("台語")
        writer.endEvent()
        proxy.calls.removeAll()

        writer.insert("，")

        XCTAssertEqual(proxy.calls, [.insert("，")])
    }
}

final class RecordingTextDocumentProxy: NSObject, UITextDocumentProxy {
    enum Call: Equatable {
        case setMarked(String, NSRange)
        case unmark
        case insert(String)
        case deleteBackward
    }

    var calls: [Call] = []
    var textBeforeCaret: String?
    var textAfterCaret: String?

    var documentContextBeforeInput: String? {
        textBeforeCaret
    }

    var documentContextAfterInput: String? {
        textAfterCaret
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

    func deleteBackward() {
        calls.append(.deleteBackward)
    }
}
