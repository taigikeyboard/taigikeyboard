// What each engine effect does to the client document.

import InputMethodKit
import XCTest

@testable import TaigiInputMethodCore

@MainActor
final class ClientEffectExecutorTests: XCTestCase {
    func testUpdatePreedit_marksTheTextUnderlinedWithTheCaretAtTheEnd() {
        let client = RecordingTextInputClient()

        ClientEffectExecutor(client: client).execute(.updatePreedit("tâi"))

        XCTAssertEqual(
            client.writes,
            [.setMarkedText("tâi", selectionLocation: 3)],
            "the composition must be marked, not inserted, and the caret sits after it",
        )
        XCTAssertEqual(
            client.lastMarkedTextAttributes[.underlineStyle] as? Int,
            NSUnderlineStyle.single.rawValue,
            "an unconverted composition is underlined — without it the user cannot tell it is provisional",
        )
        XCTAssertEqual(
            client.lastMarkedTextAttributes[.markedClauseSegment] as? Int,
            0,
            "the composition is one clause, not a run of unrelated characters",
        )
    }

    func testCommit_writesOnceAndDoesNotClearTheMarkedRegionFirst() {
        let client = RecordingTextInputClient()
        let executor = ClientEffectExecutor(client: client)

        executor.execute(.updatePreedit("tâi"))
        executor.execute(.commitTextReplacingPreedit("台"))

        XCTAssertEqual(
            client.writes,
            [.setMarkedText("tâi", selectionLocation: 3), .insertText("台")],
            """
            `insertText` already replaces the marked region, so committing must be one \
            document mutation — an extra clearing write flickers and undoes in two steps
            """,
        )
    }

    func testClearPreedit_removesTheMarkedRegionWithoutWritingText() {
        let client = RecordingTextInputClient()

        ClientEffectExecutor(client: client).execute(.clearPreeditWithoutCommit)

        XCTAssertEqual(
            client.writes,
            [.setMarkedText("", selectionLocation: 0)],
            "an abort must leave the document untouched",
        )
    }

    func testDeleteBackwardFromDocument_touchesNothing() {
        let client = RecordingTextInputClient()

        ClientEffectExecutor(client: client).execute(.deleteBackwardFromDocument)

        XCTAssertEqual(
            client.writes,
            [],
            """
            NAMED DIVERGENCE from iOS: the preedit lived only in the marked region, so \
            forwarding this would eat a character the user typed before composing
            """,
        )
    }

    func testEffectsWithNoMacOSSurfaceYet_touchNothing() {
        let client = RecordingTextInputClient()
        let executor = ClientEffectExecutor(client: client)
        let unwired: [ComposingTransition.Effect] = [
            .resetAutocomplete,
            .performAutocomplete,
            .resetAutocompleteContext,
            .nextWordUpdateLastSelectedWord(text: "台", roman: "tâi"),
            .nextWordWordSelected(text: "台", roman: "tâi", triggerPrediction: true),
            .nextWordClearForNewComposing,
        ]

        for effect in unwired {
            executor.execute(effect)
        }

        XCTAssertEqual(
            client.writes,
            [],
            "an effect this platform has no surface for must stay invisible to the document",
        )
        XCTAssertEqual(client.readCallCount, 0, "the executor never asks the client anything")
    }
}
