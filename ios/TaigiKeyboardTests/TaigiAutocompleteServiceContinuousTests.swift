import KeyboardKit
@testable import TaigiKeyboard
import XCTest

/// v3.5.8 Phase 9 Item 4 — pins the Continuous-input suggestion-emission
/// contract that `TaigiAutocompleteService.buildContinuousSuggestions` (producer)
/// and `ActionHandler+Suggestions.handleSuggestionSelection` (consumer)
/// share via `AutocompleteSuggestion.additionalInfo`.
///
/// Mirrors Android `ContinuousSuggestionsContractTest`. If iOS / Android
/// disagree on these key strings, the platform tap path silently mis-aligns
/// `commitContinuous` consumed-byte offsets and corrupts the engine pending
/// buffer — invisible to the user until they hit a bad commit boundary.
///
/// Per `docs/engine/continuous-commit-and-display.md` §10.1.2 (supersedes legacy
/// slot-0 model) + §10.3 commit contract: Continuous mode has NO
/// composing-text cell at slot 0. `candidate[0]` is the engine ranker top;
/// Tap-0 commits `candidate[0].display_text` (clarification γ — canonical
/// dictionary string, NOT the roman-with-spaces visual form).
///
/// Scope: producer-side unit tests against the internal
/// `buildContinuousSuggestions(from:)` helper (accessed via `@testable`).
/// Full `ActionHandler` tap-decode tests require mocking
/// `KeyboardContext` / `composingManager` and are deferred.
final class TaigiAutocompleteServiceContinuousTests: XCTestCase {
    private var service: TaigiAutocompleteService!

    override class func setUp() {
        super.setUp()
        RustEngineBridge.install()
    }

    override func setUp() {
        super.setUp()
        service = TaigiAutocompleteService()
    }

    override func tearDown() {
        service = nil
        super.tearDown()
    }

    private func makeCandidate(
        consumedSpanEnd: UInt32,
        syllableCount: UInt32 = 1,
        displayText: String,
        roman: String? = nil,
        hanji: String? = nil,
        canonicalTl: String? = nil,
        cellScript: Taigi_Engine_CellScript = .paired,
    ) -> RustEngineBridge.ContinuousCandidate {
        RustEngineBridge.ContinuousCandidate(
            consumedSpanEnd: consumedSpanEnd,
            syllableCount: syllableCount,
            displayText: displayText,
            roman: roman ?? displayText,
            hanji: hanji,
            canonicalTl: canonicalTl ?? roman ?? displayText,
            cellScript: cellScript,
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
            makeCandidate(consumedSpanEnd: 7, syllableCount: 2, displayText: "珠仔"),
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

    // INVARIANT_NEXTWORD_CONTINUOUS_CANONICAL_TL (behavioral-invariants.md §25)
    func testCanonicalTlSidechannelCarriesIdentityTl() {
        // R2: the canonical TL identity rides `additionalInfo["canonicalTl"]`
        // so the tap path can forward it as `commitContinuous(associationTl:)`.
        // It is independent of the display `roman` — a POJ-rendered candidate
        // shows `roman` = POJ but keeps `canonicalTl` = TL.
        let candidates = [
            makeCandidate(consumedSpanEnd: 6, displayText: "鵝", roman: "gô͘", canonicalTl: "gôo"),
        ]
        let result = service.buildContinuousSuggestions(from: candidates)
        XCTAssertEqual(
            result[0].additionalInfo["canonicalTl"],
            "gôo",
            "canonicalTl sidechannel must carry the canonical TL, not the POJ display roman",
        )
    }

    func testConsumedBytesUsesConsumedSpanEnd() {
        // Engine spans are byte-relative-to-pending-buffer; commit consumes
        // bytes up to `end`. The consumer needs `end` to know how many
        // bytes to drop from pending. Mid-commit candidate.
        let candidates = [
            makeCandidate(consumedSpanEnd: 7, displayText: "uan"),
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
        // v3.5.8 Phase 9 §10.3 clarification γ at the producer boundary.
        // After Item 6, the producer populates `Suggestion.text` from
        // `candidate.roman` (TL romanization, visual form) and
        // `additionalInfo["displayText"]` from `candidate.displayText`
        // (= hanji ?? roman, canonical commit string). On a HANT
        // candidate they DIVERGE — text shows the roman, sidechannel
        // carries the hanji. The consumer
        // (`ActionHandler+Suggestions.handleSuggestionSelection`) reads
        // `additionalInfo["displayText"]` exclusively — no
        // `?? suggestion.text` fallback — so γ holds regardless of
        // which field mutates. This test pins the sidechannel emission
        // as the authoritative commit-string carrier.
        let candidates = [
            makeCandidate(
                consumedSpanEnd: 7,
                syllableCount: 2,
                displayText: "臺灣",
                roman: "tâi-uân",
                hanji: "臺灣",
            ),
        ]
        let result = service.buildContinuousSuggestions(from: candidates)
        XCTAssertEqual(
            result[0].additionalInfo["displayText"],
            "臺灣",
            "displayText sidechannel is the canonical commit string (γ)",
        )
        XCTAssertEqual(
            result[0].text,
            "tâi-uân",
            "Item 6: Suggestion.text carries roman (visual form), diverging from sidechannel (γ)",
        )
        XCTAssertNotEqual(
            result[0].text,
            result[0].additionalInfo["displayText"],
            "Item 6 divergence — visual text MUST NOT collapse onto canonical commit string",
        )
    }

    // MARK: - v3.5.8 Phase 9 Item 6 — dual-line carrier shape

    /// HANT candidate (`hanji = Some("臺灣")`) renders dual-line:
    /// `text/title = roman`, `subtitle = hanji`. Tap-0 commits via the
    /// sidechannel `displayText` = `hanji`.
    func testItem6_HANTCandidate_DualLine() {
        let candidates = [
            makeCandidate(
                consumedSpanEnd: 7,
                syllableCount: 2,
                displayText: "臺灣",
                roman: "tâi-uân",
                hanji: "臺灣",
            ),
        ]
        let result = service.buildContinuousSuggestions(from: candidates)
        XCTAssertEqual(result[0].text, "tâi-uân", "HANT text = roman")
        XCTAssertEqual(result[0].title, "tâi-uân", "HANT title = roman")
        XCTAssertEqual(result[0].subtitle, "臺灣", "HANT subtitle = hanji")
        XCTAssertEqual(result[0].additionalInfo["displayText"], "臺灣")
    }

    /// TAILO candidate (`hanji = None`) renders single-line: subtitle
    /// stays nil; tap commits the engine's displayText sidechannel
    /// (= roman for TAILO).
    func testItem6_TAILOCandidate_SingleLine() {
        let candidates = [
            makeCandidate(
                consumedSpanEnd: 4,
                syllableCount: 1,
                displayText: "tāi",
                roman: "tāi",
                hanji: nil,
            ),
        ]
        let result = service.buildContinuousSuggestions(from: candidates)
        XCTAssertEqual(result[0].text, "tāi", "TAILO text = roman")
        XCTAssertEqual(result[0].title, "tāi", "TAILO title = roman")
        XCTAssertNil(result[0].subtitle, "TAILO subtitle = nil (no hanji)")
        XCTAssertEqual(result[0].additionalInfo["displayText"], "tāi")
    }

    /// Candidate whose hanji contains Latin letters.
    /// Renders dual-line the same way HANT does.
    func testItem6_MIXEDCandidate_DualLine() {
        let candidates = [
            makeCandidate(
                consumedSpanEnd: 9,
                syllableCount: 2,
                displayText: "hip相",
                roman: "hip-siòng",
                hanji: "hip相",
            ),
        ]
        let result = service.buildContinuousSuggestions(from: candidates)
        XCTAssertEqual(result[0].text, "hip-siòng", "MIXED text = roman")
        XCTAssertEqual(result[0].title, "hip-siòng", "MIXED title = roman")
        XCTAssertEqual(result[0].subtitle, "hip相", "MIXED subtitle = hanji")
        XCTAssertEqual(result[0].additionalInfo["displayText"], "hip相")
    }

    /// Defensive: a wire defect where `hanji = Some("")` (engine
    /// invariant says `None` for TAILO, but a faulty producer might
    /// emit an empty string) collapses to a nil subtitle so the cell
    /// renders single-line rather than showing a blank hanji line.
    /// Mirrors the spec §4.4 `(c.hanji?.isEmpty == false)` guard.
    func testItem6_HanjiPresentEmpty_CollapsesToNilSubtitle() {
        let candidates = [
            makeCandidate(
                consumedSpanEnd: 4,
                syllableCount: 1,
                displayText: "tāi",
                roman: "tāi",
                hanji: "",
            ),
        ]
        let result = service.buildContinuousSuggestions(from: candidates)
        XCTAssertNil(
            result[0].subtitle,
            "present-empty hanji must collapse to nil so the cell stays single-line",
        )
    }

    // MARK: - v3.5.8 Phase 9 Item 6 — CandidateCellHelper render parity

    /// `isHanjiFirst = true` on a HANT continuous suggestion
    /// swaps the visible title/subtitle through
    /// `CandidateCellHelper.displayTitle / displaySubtitle`. Pins that
    /// dual-line continuous candidates pick up the same swap rule as
    /// lexicon-path candidates — neither path requires a Continuous-
    /// specific code branch in the helper.
    func testItem6_TranslateSwapped_HantCandidate_ShowsHanjiPrimary() {
        let candidates = [
            makeCandidate(
                consumedSpanEnd: 7,
                syllableCount: 2,
                displayText: "臺灣",
                roman: "tâi-uân",
                hanji: "臺灣",
            ),
        ]
        let suggestion = service.buildContinuousSuggestions(from: candidates)[0]
        let title = CandidateCellHelper.displayTitle(
            for: suggestion,
            isHanjiFirst: true,
            isTPSLayout: false,
            candidateDisplayMode: .sideBySide,
        )
        let subtitle = CandidateCellHelper.displaySubtitle(
            for: suggestion,
            isHanjiFirst: true,
            isTPSLayout: false,
            candidateDisplayMode: .sideBySide,
        )
        XCTAssertEqual(title, "臺灣", "swap: title = hanji")
        XCTAssertEqual(subtitle, "tâi-uân", "swap: subtitle = roman")
    }

    /// TPS-layout × HANT continuous: `displayTitle` returns hanji
    /// (subtitle is non-empty) and `displaySubtitle` returns nil.
    /// Pins behavior matches lexicon path under TPS keyboard.
    func testItem6_TPSLayout_HantCandidate_ShowsHanjiOnly() {
        let candidates = [
            makeCandidate(
                consumedSpanEnd: 7,
                syllableCount: 2,
                displayText: "臺灣",
                roman: "tâi-uân",
                hanji: "臺灣",
            ),
        ]
        let suggestion = service.buildContinuousSuggestions(from: candidates)[0]
        let title = CandidateCellHelper.displayTitle(
            for: suggestion,
            isHanjiFirst: false,
            isTPSLayout: true,
            candidateDisplayMode: .sideBySide,
        )
        let subtitle = CandidateCellHelper.displaySubtitle(
            for: suggestion,
            isHanjiFirst: false,
            isTPSLayout: true,
            candidateDisplayMode: .sideBySide,
        )
        XCTAssertEqual(title, "臺灣", "TPS: title = hanji (subtitle present)")
        XCTAssertNil(subtitle, "TPS never shows a subtitle")
    }

    // MARK: - §42 Hanji with Romanization split cells

    /// Under Hanji with Romanization the engine lists a hanji-bearing candidate as a Hanji row then
    /// a romanization row (`CandidateMessage.cell_script`, deduped per script engine-side —
    /// `protos::split_single_script_cells`). Each becomes a single-script cell, neither with a
    /// subtitle, and the SEMANTIC sidechannels (identity + engine offsets) ride on both.
    func testCombined_SplitRows_BecomeHanjiThenRomanCell() {
        let row = { (script: Taigi_Engine_CellScript) in
            self.makeCandidate(
                consumedSpanEnd: 7,
                syllableCount: 2,
                displayText: "台語",
                roman: "tâi-gí",
                hanji: "台語",
                canonicalTl: "tâi-gí",
                cellScript: script,
            )
        }
        let result = service.buildContinuousSuggestions(from: [row(.hanji), row(.roman)])
        XCTAssertEqual(result.count, 2, "one row per cell")

        let hanjiCell = result[0]
        XCTAssertEqual(hanjiCell.text, "台語")
        XCTAssertEqual(hanjiCell.title, "台語")
        XCTAssertNil(hanjiCell.subtitle, "split cells carry no subtitle")
        XCTAssertEqual(hanjiCell.additionalInfo[CandidateCellScript.infoKey], CandidateCellScript.hanji)
        XCTAssertEqual(
            hanjiCell.additionalInfo[CandidateCellScript.bracketRomanKey],
            "tâi-gí",
            "bracket-form carrier",
        )

        let romanCell = result[1]
        XCTAssertEqual(romanCell.text, "tâi-gí")
        XCTAssertEqual(romanCell.title, "tâi-gí")
        XCTAssertNil(romanCell.subtitle, "split cells carry no subtitle")
        XCTAssertEqual(romanCell.additionalInfo[CandidateCellScript.infoKey], CandidateCellScript.roman)

        for cell in result {
            XCTAssertEqual(cell.additionalInfo["isContinuous"], "true")
            XCTAssertEqual(cell.additionalInfo["consumedBytes"], "7")
            XCTAssertEqual(cell.additionalInfo["syllableCount"], "2")
            XCTAssertEqual(cell.additionalInfo["displayText"], "台語", "identity never moves")
            XCTAssertEqual(cell.additionalInfo["canonicalTl"], "tâi-gí", "identity never moves")
        }
    }

    func testCombined_HanjiLessRomanRow_IsOneRomanCell() {
        let candidates = [
            makeCandidate(consumedSpanEnd: 4, displayText: "tāi", roman: "tāi", hanji: nil, cellScript: .roman),
        ]
        let result = service.buildContinuousSuggestions(from: candidates)
        XCTAssertEqual(result.count, 1)
        XCTAssertEqual(result[0].text, "tāi")
        XCTAssertNil(result[0].subtitle)
        XCTAssertEqual(result[0].additionalInfo[CandidateCellScript.infoKey], CandidateCellScript.roman)
    }

    /// A `.paired` row (every display but Hanji with Romanization) is today's dual-script
    /// suggestion with no `cellScript` marker. Mirrors Android's "S27 combined split OFF" pin.
    func testPairedRow_EmitsDualScriptShapeWithoutMarker() {
        let candidates = [
            makeCandidate(
                consumedSpanEnd: 7,
                syllableCount: 2,
                displayText: "臺灣",
                roman: "tâi-uân",
                hanji: "臺灣",
            ),
        ]
        let result = service.buildContinuousSuggestions(from: candidates)
        XCTAssertEqual(result.count, 1)
        XCTAssertEqual(result[0].text, "tâi-uân")
        XCTAssertEqual(result[0].subtitle, "臺灣")
        XCTAssertNil(result[0].additionalInfo[CandidateCellScript.infoKey], "no marker on a paired row")
    }

    // MARK: - Misc

    func testEmptyCandidateList_EmitsEmptyList() {
        // v3.5.8 Phase 9 Item 4 / Item 13: §10.7 edge case "Empty buffer" /
        // partial prefix — the strip is empty when the engine returns no
        // candidates. After Item 13 the engine is the single candidate
        // source (no lexicon fallback, no synthetic slot-0 cell). Helper
        // contract: zero candidates → zero suggestions.
        let result = service.buildContinuousSuggestions(from: [])
        XCTAssertTrue(result.isEmpty, "Empty candidates → empty suggestions")
    }

    // MARK: - v3.5.8 Phase 9 Item 13 — fallback retire (§15.6)

    /// Minimal stub that supplies both the composing state and the
    /// Continuous fetch surface, so `autocomplete(text:in:)` can be exercised
    /// end-to-end without a real `ComposingManager`.
    private final class StubComposing: ComposingStateProvider, ContinuousCandidateFetcher {
        var isComposing = true
        var rawInput = "gua"
        var composingText = "gua"
        var fetchResult: [RustEngineBridge.ContinuousCandidate] = []
        func fetchContinuousCandidates() -> [RustEngineBridge.ContinuousCandidate] {
            fetchResult
        }
    }

    /// `platform_autocomplete_no_lexicon_branch` (§15.6): after the
    /// fallback retire, an engine that returns no candidates yields an
    /// empty strip — there is NO platform lexicon path and NO slot-0
    /// composing-text cell. Pins that `autocomplete(text:in:)` is engine-only.
    @MainActor
    func testAutocomplete_EmptyEngine_NoLexiconBranch_EmptyResult() async throws {
        let stub = StubComposing()
        stub.fetchResult = []
        service.setComposingManager(stub)
        let result = try await service.autocomplete(text: "gua", in: .current)
        XCTAssertTrue(
            result.suggestions.isEmpty,
            "empty engine → empty strip (no lexicon fallback, no slot-0 cell)",
        )
    }

    /// Positive control: a non-empty engine result flows straight through
    /// `buildContinuousSuggestions` with no slot-0 cell injected.
    @MainActor
    func testAutocomplete_EngineCandidates_SingleSourcePassthrough() async throws {
        let stub = StubComposing()
        stub.fetchResult = [makeCandidate(consumedSpanEnd: 3, displayText: "guá")]
        service.setComposingManager(stub)
        let result = try await service.autocomplete(text: "gua", in: .current)
        XCTAssertEqual(result.suggestions.count, 1, "engine candidates pass through 1:1")
        XCTAssertEqual(result.suggestions[0].additionalInfo["isContinuous"], "true")
        XCTAssertNil(
            result.suggestions[0].additionalInfo["isComposingText"],
            "no slot-0 composing-text cell on the single-source path",
        )
    }
}
