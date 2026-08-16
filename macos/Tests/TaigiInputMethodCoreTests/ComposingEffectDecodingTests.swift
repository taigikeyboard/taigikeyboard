// Pins the wire-to-Swift mapping of every effect the engine can emit.

import SwiftProtobuf
@testable import TaigiInputMethodCore
import XCTest

/// The composing ops macOS ships today emit six of the ten effects; the other
/// four arrive with the candidate window and the next-word learner. Driving the
/// decoder directly is what keeps the untriggered four honest — a swapped
/// `text`/`roman` pair or a hardcoded `triggerPrediction` would otherwise pass
/// every test in this suite and only surface later as associations learned
/// against the wrong word.
final class ComposingEffectDecodingTests: XCTestCase {
    func testDecodeTransition_everyEffectKind_mapsWithPayloadsAndKeepsWireOrder() {
        let response = makeResponse(effects: [
            makeEffect(.updatePreedit(payload(Taigi_Engine_UpdatePreedit()) { $0.display = "tâi" })),
            makeEffect(.clearPreeditWithoutCommit_p(Taigi_Engine_ClearPreeditWithoutCommit())),
            makeEffect(.commitTextReplacingPreedit(
                payload(Taigi_Engine_CommitTextReplacingPreedit()) { $0.text = "台語" },
            )),
            makeEffect(.deleteBackwardFromDocument(Taigi_Engine_DeleteBackwardFromDocument())),
            makeEffect(.resetAutocomplete(Taigi_Engine_ResetAutocomplete())),
            makeEffect(.performAutocomplete(Taigi_Engine_PerformAutocomplete())),
            makeEffect(.resetAutocompleteContext(Taigi_Engine_ResetAutocompleteContext())),
            makeEffect(.nextWordUpdateLastSelectedWord(
                payload(Taigi_Engine_NextWordUpdateLastSelectedWord()) {
                    $0.text = "台"
                    $0.roman = "tâi"
                },
            )),
            makeEffect(.nextWordWordSelected(
                payload(Taigi_Engine_NextWordWordSelected()) {
                    $0.text = "語"
                    $0.roman = "gí"
                    $0.triggerPrediction = true
                },
            )),
            makeEffect(.nextWordClearForNewComposing(Taigi_Engine_NextWordClearForNewComposing())),
        ])

        let transition = RustEngineBridge.decodeTransition(response)

        XCTAssertEqual(transition.effects, [
            .updatePreedit("tâi"),
            .clearPreeditWithoutCommit,
            .commitTextReplacingPreedit("台語"),
            .deleteBackwardFromDocument,
            .resetAutocomplete,
            .performAutocomplete,
            .resetAutocompleteContext,
            .nextWordUpdateLastSelectedWord(text: "台", roman: "tâi"),
            .nextWordWordSelected(text: "語", roman: "gí", triggerPrediction: true),
            .nextWordClearForNewComposing,
        ])
    }

    func testDecodeTransition_triggerPredictionFalse_isCarriedNotAssumed() {
        // The engine decides whether a commit should fire a prediction; a decoder
        // that defaulted this to `true` would look correct against the case above.
        let response = makeResponse(effects: [
            makeEffect(.nextWordWordSelected(
                payload(Taigi_Engine_NextWordWordSelected()) {
                    $0.text = "語"
                    $0.roman = "gí"
                    $0.triggerPrediction = false
                },
            )),
        ])

        XCTAssertEqual(
            RustEngineBridge.decodeTransition(response).effects,
            [.nextWordWordSelected(text: "語", roman: "gí", triggerPrediction: false)],
        )
    }

    func testDecodeTransition_effectWithNoKind_isDroppedWithoutDroppingItsNeighbours() {
        let response = makeResponse(effects: [
            Taigi_Engine_Effect(),
            makeEffect(.resetAutocomplete(Taigi_Engine_ResetAutocomplete())),
        ])

        XCTAssertEqual(RustEngineBridge.decodeTransition(response).effects, [.resetAutocomplete])
    }

    private func makeResponse(effects: [Taigi_Engine_Effect]) -> Taigi_Engine_ComposingResponse {
        var response = Taigi_Engine_ComposingResponse()
        response.effect = effects
        return response
    }

    private func makeEffect(_ kind: Taigi_Engine_Effect.OneOf_Kind) -> Taigi_Engine_Effect {
        var effect = Taigi_Engine_Effect()
        effect.kind = kind
        return effect
    }

    private func payload<Message: SwiftProtobuf.Message>(
        _ message: Message,
        _ configure: (inout Message) -> Void,
    ) -> Message {
        var message = message
        configure(&message)
        return message
    }
}
