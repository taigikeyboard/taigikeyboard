@testable import TaigiKeyboard
import KeyboardKit
import XCTest

/// v3.5.8 Phase 9 Item 4 — pins the Continuous-input suggestion-emission
/// contract that `AutocompleteService.buildContinuousSuggestions` (producer)
/// and `ActionHandler+Suggestions.handleSuggestionSelection` (consumer)
/// share via `Autocomplete.Suggestion.additionalInfo`.
///
/// Mirrors Android `ContinuousSuggestionsContractTest`. If iOS / Android
/// disagree on these key strings, the platform tap path silently mis-aligns
/// `commitContinuous` consumed-byte offsets and corrupts the engine pending
/// buffer — invisible to the user until they hit a bad commit boundary.
///
/// Per `docs/engine/continuous-input-ranking.md` §10.1.2 (supersedes legacy
/// slot-0 model) + §10.3 commit contract: Continuous mode has NO
/// composing-text cell at slot 0. `candidate[0]` is the engine ranker top;
/// Tap-0 commits `candidate[0].display_text` (clarification γ — canonical
/// dictionary string, NOT the roman-with-spaces visual form).
///
/// Scope: producer-side unit tests against the internal
/// `buildContinuousSuggestions(from:)` helper (accessed via `@testable`).
/// Full `ActionHandler` tap-decode tests require mocking
/// `KeyboardContext` / `composingManager` and are deferred.
final class AutocompleteServiceContinuousTests: XCTestCase {

    private var service: AutocompleteService!

    override class func setUp() {
        super.setUp()
        RustEngineBridge.install()
    }

    override func setUp() {
        super.setUp()
        service = AutocompleteService()
    }

    override func tearDown() {
        service = nil
        super.tearDown()
    }

    private func makeCandidate(
        consumedSpanStart: UInt32 = 0,
        consumedSpanEnd: UInt32,
        syllableCount: UInt32 = 1,
        displayText: String,
        score: Float = 1.0,
        mode: RustEngineBridge.CandidateMode = .hant,
        roman: String? = nil,
        hanji: String? = nil,
    ) -> RustEngineBridge.ContinuousCandidate {
        RustEngineBridge.ContinuousCandidate(
            consumedSpanStart: consumedSpanStart,
            consumedSpanEnd: consumedSpanEnd,
            syllableCount: syllableCount,
            displayText: displayText,
            score: score,
            form: 1,
            mode: mode,
            roman: roman ?? displayText,
            hanji: hanji,
        )
    }

    func testSlotZeroIsCandidateTop_NoComposingCell() {
        // v3.5.8 Phase 9 Item 4: §10.1.2 supersedes notice — the Continuous
        // path no longer inserts a `createComposingTextSuggestion` at index 0.
        // `candidate[0]` IS slot 0, carrying `isContinuous="true"` so the
        // ActionHandler routes it through the Continuous commit branch.
        let candidates = [
            makeCandidate(consumedSpanEnd: 4, displayText: "tsua"),
        ]
        let result = service.buildContinuousSuggestions(from: candidates)

        XCTAssertEqual(result.count, 1, "Continuous path emits exactly N cells (no slot-0 composing cell)")
        let slot0 = result[0]
        XCTAssertEqual(slot0.additionalInfo["isContinuous"], "true")
        XCTAssertNil(
            slot0.additionalInfo["isComposingText"],
            "slot 0 must NOT carry isComposingText (legacy slot-0 model superseded)",
        )
    }

    func testContinuousCandidatesCarryExactMetadataKeyStrings() {
        let candidates = [
            makeCandidate(consumedSpanEnd: 4, syllableCount: 1, displayText: "tsua"),
            makeCandidate(consumedSpanEnd: 7, syllableCount: 2, displayText: "珠仔", score: 0.5),
        ]
        let result = service.buildContinuousSuggestions(from: candidates)

        // First candidate (slot 0 — was slot 1 before Item 4)
        XCTAssertEqual(result[0].additionalInfo["isContinuous"], "true")
        XCTAssertEqual(result[0].additionalInfo["consumedBytes"], "4")
        XCTAssertEqual(result[0].additionalInfo["syllableCount"], "1")
        XCTAssertEqual(result[0].additionalInfo["displayText"], "tsua")

        // Second candidate (slot 1 — was slot 2 before Item 4)
        XCTAssertEqual(result[1].additionalInfo["isContinuous"], "true")
        XCTAssertEqual(result[1].additionalInfo["consumedBytes"], "7")
        XCTAssertEqual(result[1].additionalInfo["syllableCount"], "2")
        XCTAssertEqual(result[1].additionalInfo["displayText"], "珠仔")
    }

    func testConsumedBytesUsesConsumedSpanEnd() {
        // Engine spans are byte-relative-to-pending-buffer; commit consumes
        // bytes [start, end). The consumer needs `end` to know how many
        // bytes to drop from pending. Mid-commit candidate.
        let candidates = [
            makeCandidate(consumedSpanStart: 3, consumedSpanEnd: 7, displayText: "uan"),
        ]
        let result = service.buildContinuousSuggestions(from: candidates)
        XCTAssertEqual(result[0].additionalInfo["consumedBytes"], "7")
    }

    func testDisplayTextSidechannelIsEngineSuppliedRaw() {
        // Codex P1 fix `f01559cf` — TPS layout would view-rewrite the
        // `Suggestion.text` field, breaking commitContinuous alignment.
        // Sidechannel `displayText` is the contract.
        let candidates = [
            makeCandidate(consumedSpanEnd: 6, syllableCount: 2, displayText: "tâi-uân"),
        ]
        let result = service.buildContinuousSuggestions(from: candidates)
        XCTAssertEqual(
            result[0].additionalInfo["displayText"],
            "tâi-uân",
            "displayText sidechannel must equal engine's raw displayText",
        )
    }

    func testGammaClarification_DisplayTextSidechannelDecouplesFromText() {
        // v3.5.8 Phase 9 Item 4 — pin §10.3 clarification γ at the producer
        // boundary. Today the producer initializes both `Suggestion.text`
        // and `additionalInfo["displayText"]` from `candidate.displayText`,
        // so they coincide. Once Item 5/6 add proto `roman`/`hanji` fields
        // and slot-0 renders the segmented visual form, the producer will
        // diverge `Suggestion.text` (visual form, may carry word spaces)
        // from `additionalInfo["displayText"]` (canonical commit string).
        // The consumer (`ActionHandler+Suggestions`) must already be reading
        // `additionalInfo["displayText"]` exclusively — no `??
        // suggestion.text` fallback — so γ holds regardless of which field
        // mutates first. This test pins the sidechannel emission as the
        // authoritative commit-string carrier so Item 6 cannot accidentally
        // route through `Suggestion.text`.
        let candidates = [
            makeCandidate(consumedSpanEnd: 6, syllableCount: 2, displayText: "tâi-gí"),
        ]
        let result = service.buildContinuousSuggestions(from: candidates)
        XCTAssertEqual(
            result[0].additionalInfo["displayText"],
            "tâi-gí",
            "displayText sidechannel is the canonical commit string (γ)",
        )
        // Sanity: the producer's current shape (`text = displayText`)
        // is exercised — Item 6 will diverge `text` from `displayText` and
        // this assertion will need updating; the assertion above pins the
        // contract the consumer relies on (sidechannel, not `text`).
        XCTAssertEqual(
            result[0].text,
            "tâi-gí",
            "Item 4 producer still couples Suggestion.text to displayText (Item 6 will diverge)",
        )
    }

    func testEmptyCandidateList_EmitsEmptyList() {
        // v3.5.8 Phase 9 Item 4: §10.7 edge case "Empty buffer" / partial
        // prefix — strip is empty when engine returns no candidates. Caller
        // (`AutocompleteService.autocomplete`) already guards
        // `if !candidates.isEmpty` before invoking this helper and falls
        // through to the lexicon path when empty, which re-inserts its own
        // slot-0 composing-text cell per §10.5 mode gating. The helper
        // contract here is simply: zero candidates → zero suggestions, no
        // synthetic slot-0 fallback.
        let result = service.buildContinuousSuggestions(from: [])
        XCTAssertTrue(result.isEmpty, "Empty candidates → empty suggestions")
    }
}
