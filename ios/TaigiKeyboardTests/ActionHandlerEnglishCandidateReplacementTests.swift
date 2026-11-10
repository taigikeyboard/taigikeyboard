import KeyboardKit
@testable import TaigiKeyboard
import UIKit
import XCTest

final class ActionHandlerEnglishCandidateReplacementTests: XCTestCase {
    func testEnglishCandidateSelection_replacesTypedPreedit() async {
        await MainActor.run {
            let settings = SharedSettings.shared
            let originalInputMode = settings.inputMode
            let originalKeyboardLayoutType = settings.keyboardLayoutType
            let originalAutoSpaceEnabled = settings.isAutoSpaceEnabled
            defer {
                settings.inputMode = originalInputMode
                settings.keyboardLayoutType = originalKeyboardLayoutType
                settings.isAutoSpaceEnabled = originalAutoSpaceEnabled
            }
            settings.inputMode = .english
            settings.isAutoSpaceEnabled = false

            let proxy = CandidateReplacementTextDocumentProxy()
            let state = KeyboardState()
            state.controllerContext.setCustomTextDocumentProxy(proxy)
            let handler = ActionHandler(controller: nil, services: KeyboardServices(state: state), state: state)

            // Given typing "win" leaves "Win" as the host's marked preedit.
            handler.hostText.update("Win")

            // When the user selects the English completion "Windows".
            handler.handle(AutocompleteSuggestion(text: "Windows"))

            // Then the completion replaces the preedit rather than being appended to it.
            XCTAssertEqual(proxy.visibleText, "Windows")
        }
    }
}

private final class CandidateReplacementTextDocumentProxy: NSObject, UITextDocumentProxy {
    private(set) var committedText = ""
    private(set) var markedText = ""

    var visibleText: String {
        committedText + markedText
    }

    var documentContextBeforeInput: String? {
        committedText
    }

    var documentContextAfterInput: String? {
        ""
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
        !visibleText.isEmpty
    }

    func adjustTextPosition(byCharacterOffset _: Int) {}

    func setMarkedText(_ markedText: String, selectedRange _: NSRange) {
        self.markedText = markedText
    }

    func unmarkText() {
        committedText += markedText
        markedText = ""
    }

    func insertText(_ text: String) {
        if markedText.isEmpty {
            committedText += text
        } else {
            // Model the affected host behavior: insertText appends to marked preedit.
            markedText += text
        }
    }

    func deleteBackward() {
        if !markedText.isEmpty {
            markedText.removeLast()
        } else if !committedText.isEmpty {
            committedText.removeLast()
        }
    }
}
