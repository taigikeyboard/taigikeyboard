@testable import TaigiKeyboard
import XCTest

/// v3.5.5 NextWord slice — iOS bridge parity tests.
///
/// Replaces the deleted `NextWordEngineTests`, `NextWordScorerTests`, and
/// `AutocompleteContextBoosterTests` (commit 8 swap). Exercises the FFI
/// seam — proto encode, dispatcher routing, value-type synthesis,
/// generation-mismatch state reset on the singleton iOS handle.
///
/// Exhaustive invariant coverage lives in `engine/nextword/src/*` Rust
/// tests; this file is the iOS-side boundary check that the bridge
/// surface, AppConfig population, and synthesized value types behave the
/// way `NextWordController` / `NextWordService` /
/// `TaigiAutocompleteService` expect after the swap.
///
/// Cross-language tolerance 1e-7 per Codex v1 P2 (Rust f64 ↔ Swift Double
/// round-trip via proto).
final class RustEngineBridgeNextWordTests: XCTestCase {
    override class func setUp() {
        super.setUp()
        RustEngineBridge.install()
    }

    /// Cross-language tolerance for Rust f64 ↔ Swift Double round-trip.
    private static let parityTolerance: Double = 1e-7

    /// Per-test envelope generation. Bumped each `setUp()` so the engine
    /// handle's `last_generation` mismatch fires on the test's first
    /// call, silently dropping state from the prior test (and bumping
    /// `current_generation` by 1 so pre-reset in-flight filter results
    /// drop as stale — see `engine/nextword/src/api.rs::Engine::reset`).
    private static var nextEnvelopeGen: UInt64 = 100_000
    private var envelopeGen: UInt64 = 0

    /// Snapshot of `state.current_generation` after the per-test ResetFull
    /// baseline. Tests that previously asserted absolute generation values
    /// (e.g. `==1`, `==2`, `==3`) now assert relative to this baseline so
    /// they stay green across the bump-on-envelope-reset semantic the
    /// Rust crate ships post-PR-#198.
    private var baselineGen: UInt64 = 0

    override func setUp() {
        super.setUp()
        Self.nextEnvelopeGen &+= 1
        envelopeGen = Self.nextEnvelopeGen
        // Baseline: ResetFull lands on a freshly-reset engine state and
        // bumps `current_generation` by 1. The exact baseline value
        // depends on the prior test's terminal state (envelope-reset is
        // bump-not-zero per PR #198 fix).
        let baseline = RustEngineBridge.nextwordResetFull(
            nowMs: 0,
            mode: .tl,
            translateSwapped: false,
            associationRecordingEnabled: true,
            generation: envelopeGen,
        )
        baselineGen = baseline.currentGeneration
        XCTAssertNil(baseline.lastSelectedWord)
        XCTAssertFalse(baseline.isShowing)
    }

    // MARK: - Decide: WordSelected (records association within window)

    func testWordSelected_recordsAssociationWithinWindow() {
        let primed = wordSelected(text: "早", roman: "tsá", nowMs: 0)
        XCTAssertEqual(primed.currentGeneration, baselineGen &+ 1)
        XCTAssertEqual(primed.lastSelectedWord, "早")

        let result = wordSelected(text: "安", roman: "an", nowMs: 5_000)
        XCTAssertEqual(result.currentGeneration, baselineGen &+ 2)
        let expected = RustEngineBridge.NextWordAssociationPair(
            prev: "早", prevTl: "tsá", next: "安", nextTl: "an",
        )
        XCTAssertTrue(
            result.effects.contains(.recordAssociation(expected)),
            "effects=\(result.effects)",
        )
    }

    func testWordSelected_skipsRecordOutsideWindow() {
        _ = wordSelected(text: "早", roman: "tsá", nowMs: 0)
        let result = wordSelected(text: "安", roman: "an", nowMs: 20_000)
        for effect in result.effects {
            if case .recordAssociation = effect {
                XCTFail("association at 20s should be dropped (>= 10s window)")
            }
        }
    }

    func testWordSelected_recordingDisabled_emitsNoRecord() {
        _ = wordSelected(
            text: "早", roman: "tsá", nowMs: 0,
            associationRecordingEnabled: false,
        )
        let result = wordSelected(
            text: "安", roman: "an", nowMs: 5_000,
            associationRecordingEnabled: false,
        )
        for effect in result.effects {
            switch effect {
            case .recordAssociation, .recordCompoundAssociations:
                XCTFail("recording disabled — no record effects allowed")
            default: continue
            }
        }
    }

    func testWordSelected_compoundText_emitsSequentialPairs() {
        // iOS splits on `-` only — "tshit-niû" → ["tshit", "niû"].
        let result = wordSelected(text: "tshit-niû", roman: "tshit-niû", nowMs: 0)
        var pairs: [RustEngineBridge.NextWordAssociationPair] = []
        for effect in result.effects {
            if case let .recordCompoundAssociations(p) = effect {
                pairs = p
            }
        }
        XCTAssertEqual(pairs, [
            RustEngineBridge.NextWordAssociationPair(
                prev: "tshit", prevTl: "tshit", next: "niû", nextTl: "niû",
            ),
        ])
    }

    func testWordSelected_sentenceEndPunctuation_resetsAndCancelsTimer() {
        // Prime: a non-sentence-end word + showing flag.
        _ = wordSelected(text: "早", roman: "tsá", nowMs: 0)
        let result = wordSelected(text: "。", roman: "", nowMs: 100)
        XCTAssertNil(result.lastSelectedWord)
        XCTAssertTrue(result.effects.contains(.cancelContextTimeout))
    }

    func testWordSelected_noiseText_skipsSilently() {
        let result = wordSelected(text: ",", roman: "", nowMs: 0)
        XCTAssertEqual(result.effects, [])
        XCTAssertEqual(result.currentGeneration, baselineGen, "no-op preserves baseline")
    }

    func testWordSelected_requireRomanModeInSwappedMode_isNoop() {
        let result = wordSelected(
            text: "abc", roman: "abc", nowMs: 0,
            requireRomanMode: true, translateSwapped: true,
        )
        XCTAssertEqual(result.effects, [])
        XCTAssertEqual(result.currentGeneration, baselineGen)
    }

    func testWordSelected_triggerPredictionFalse_recordsButSkipsQuery() {
        let result = wordSelected(
            text: "早", roman: "tsá", nowMs: 0,
            triggerPrediction: false,
        )
        var sawReschedule = false
        for effect in result.effects {
            switch effect {
            case .rescheduleContextTimeout: sawReschedule = true
            case .queryPredictions: XCTFail("triggerPrediction=false must not query")
            default: continue
            }
        }
        XCTAssertTrue(sawReschedule)
    }

    func testWordSelected_emptyRoman_carriesEmptyTL() {
        // Mirrors `ActionHandler+Suggestions` hanzi-only path:
        // associationRoman="" → nextTl="" preserved end-to-end.
        _ = wordSelected(text: "早", roman: "tsá", nowMs: 0)
        let result = wordSelected(text: "安", roman: "", nowMs: 5_000)
        let expected = RustEngineBridge.NextWordAssociationPair(
            prev: "早", prevTl: "tsá", next: "安", nextTl: "",
        )
        XCTAssertTrue(result.effects.contains(.recordAssociation(expected)))
    }

    // MARK: - Decide: Backspace

    func testBackspace_emitsQueryPredictionsButNoRecord() {
        _ = wordSelected(text: "早", roman: "tsá", nowMs: 0)
        let result = backspace(lastChar: "安", nowMs: 100)

        var sawQuery = false
        for effect in result.effects {
            switch effect {
            case .recordAssociation, .recordCompoundAssociations:
                XCTFail("backspace must never record")
            case let .queryPredictions(word, roman, generation, _):
                XCTAssertEqual(word, "安")
                XCTAssertEqual(roman, "")
                XCTAssertEqual(generation, result.currentGeneration)
                sawQuery = true
            default: continue
            }
        }
        XCTAssertTrue(sawQuery)
    }

    // MARK: - Decide: ContextTimeoutFired

    func testContextTimeoutFired_resetsStateAndCancelsTimer() {
        _ = wordSelected(text: "早", roman: "tsá", nowMs: 0)
        let result = contextTimeoutFired(nowMs: 30_000)
        XCTAssertNil(result.lastSelectedWord)
        XCTAssertTrue(result.effects.contains(.cancelContextTimeout))
    }

    // MARK: - Decide: ClearForNewComposing

    func testClearForNewComposing_whenNotShowing_emitsNothing() {
        let result = clearForNewComposing(nowMs: 0)
        XCTAssertEqual(result.effects, [])
        XCTAssertEqual(result.currentGeneration, baselineGen &+ 1, "still bumps even with no UI clear")
        XCTAssertFalse(result.isShowing)
    }

    // MARK: - Decide: ResetFull

    func testResetFull_zeroesStateAndCancelsTimer() {
        _ = wordSelected(text: "早", roman: "tsá", nowMs: 0)
        let result = RustEngineBridge.nextwordResetFull(
            nowMs: 100,
            mode: .tl,
            translateSwapped: false,
            associationRecordingEnabled: true,
            generation: envelopeGen,
        )
        XCTAssertNil(result.lastSelectedWord)
        XCTAssertTrue(result.effects.contains(.cancelContextTimeout))
    }

    // MARK: - SetIsShowing → ClearForNewComposing gate

    func testSetIsShowing_thenClearForNewComposing_emitsClearUIEffect() {
        // Pre-fix: bridge had no setter for state.is_showing, so the gate
        // never tripped after swap. Guards the v3.5.5 bridge gap fix.
        _ = RustEngineBridge.nextwordSetIsShowing(
            true,
            mode: .tl,
            translateSwapped: false,
            associationRecordingEnabled: true,
            generation: envelopeGen,
        )
        let result = clearForNewComposing(nowMs: 100)
        var sawClear = false
        for effect in result.effects {
            if case .clearPredictionsUI = effect { sawClear = true }
        }
        XCTAssertTrue(sawClear, "ClearForNewComposing must emit clearPredictionsUI when is_showing=true")
    }

    func testSetIsShowing_doesNotBumpGeneration() {
        let baselineGen = currentGen()
        let result = RustEngineBridge.nextwordSetIsShowing(
            true,
            mode: .tl,
            translateSwapped: false,
            associationRecordingEnabled: true,
            generation: envelopeGen,
        )
        XCTAssertEqual(result.currentGeneration, baselineGen, "no generation bump")
        XCTAssertTrue(result.isShowing)
        XCTAssertEqual(result.effects, [])
    }

    // MARK: - QueryState read-back

    func testQueryState_reflectsLastWordSelected() {
        _ = wordSelected(text: "早", roman: "tsá", nowMs: 0)
        let snapshot = RustEngineBridge.nextwordQueryState(
            mode: .tl,
            translateSwapped: false,
            associationRecordingEnabled: true,
            generation: envelopeGen,
        )
        XCTAssertEqual(snapshot.lastSelectedWord, "早")
        XCTAssertFalse(snapshot.isShowing, "is_showing is platform-driven, not engine-set")
        XCTAssertEqual(snapshot.currentGeneration, baselineGen &+ 1)
    }

    // MARK: - Filter: scoring + ordering

    func testFilter_userOutranksDictAtEqualCount() {
        let gen = currentGen()
        let result = RustEngineBridge.nextwordFilter(
            raw: [
                row(hanzi: "好", tl: "hó", count: 100, source: .dict),
                row(hanzi: "早", tl: "tsá", count: 1, lastUsedMs: 1_000, source: .user),
            ],
            queryGeneration: gen,
            nowMs: 1_000,
            limit: 10,
            mode: .tl, translateSwapped: false, associationRecordingEnabled: true,
            generation: envelopeGen,
        )
        XCTAssertFalse(result.wasStale)
        XCTAssertEqual(result.predictions.first?.hanzi, "早", "user entry must outrank dict via learningBonus")
    }

    func testFilter_dictAndUserMergeSumScores() {
        let gen = currentGen()
        let result = RustEngineBridge.nextwordFilter(
            raw: [
                row(hanzi: "好", tl: "hó", count: 5, source: .dict),
                row(hanzi: "好", tl: "hó", count: 1, lastUsedMs: 1_000, source: .user),
            ],
            queryGeneration: gen,
            nowMs: 1_000,
            limit: 10,
            mode: .tl, translateSwapped: false, associationRecordingEnabled: true,
            generation: envelopeGen,
        )
        XCTAssertEqual(result.predictions.count, 1, "(好, hó) merges across sources")
        // Score = scoreDict(5) + calculateUserScore(1, lastUsedMs=now, nowMs=now)
        //       = 5*1 + (1*50*1 + 300) = 355.
        let expected: Double = 5.0 + (1.0 * 50.0 + 300.0)
        XCTAssertEqual(
            result.predictions[0].score, expected,
            accuracy: Self.parityTolerance,
            "merged score must equal sum of dict + user contributions",
        )
    }

    func testFilter_emptyTLDroppedInRomanMode() {
        let gen = currentGen()
        let result = RustEngineBridge.nextwordFilter(
            raw: [
                row(hanzi: "好", tl: "hó", count: 5, source: .dict),
                row(hanzi: "安", tl: "", count: 5, source: .dict),
            ],
            queryGeneration: gen,
            nowMs: 1_000,
            limit: 10,
            mode: .tl, translateSwapped: false, associationRecordingEnabled: true,
            generation: envelopeGen,
        )
        XCTAssertEqual(result.predictions.count, 1)
        XCTAssertEqual(result.predictions[0].hanzi, "好")
    }

    func testFilter_emptyTLKeptInHanjiMode() {
        let gen = currentGen()
        let result = RustEngineBridge.nextwordFilter(
            raw: [
                row(hanzi: "好", tl: "hó", count: 5, source: .dict),
                row(hanzi: "安", tl: "", count: 5, source: .dict),
            ],
            queryGeneration: gen,
            nowMs: 1_000,
            limit: 10,
            mode: .tl, translateSwapped: true, associationRecordingEnabled: true,
            generation: envelopeGen,
        )
        XCTAssertEqual(result.predictions.count, 2)
    }

    func testFilter_pojModeEmitsPojRoman() {
        let gen = currentGen()
        let result = RustEngineBridge.nextwordFilter(
            raw: [row(hanzi: "好", tl: "tsiok", count: 5, source: .dict)],
            queryGeneration: gen,
            nowMs: 1_000,
            limit: 10,
            mode: .poj, translateSwapped: false, associationRecordingEnabled: true,
            generation: envelopeGen,
        )
        XCTAssertEqual(result.predictions.count, 1)
        // tl_display_to_poj_display("tsiok") → "chiok"; the assertion guards the
        // commit-path bug fix that made `NextWordEnginePrediction.text` mode-correct.
        XCTAssertEqual(result.predictions[0].text, "chiok")
        XCTAssertEqual(result.predictions[0].tl, "tsiok")
    }

    func testFilter_staleQueryGenerationReturnsWasStale() {
        let gen = currentGen()
        let result = RustEngineBridge.nextwordFilter(
            raw: [row(hanzi: "好", tl: "hó", count: 5, source: .dict)],
            queryGeneration: gen &- 1, // mismatch
            nowMs: 1_000,
            limit: 10,
            mode: .tl, translateSwapped: false, associationRecordingEnabled: true,
            generation: envelopeGen,
        )
        XCTAssertTrue(result.wasStale)
        XCTAssertTrue(result.predictions.isEmpty)
    }

    func testFilter_limitTruncates() {
        let gen = currentGen()
        let raw = (0 ..< 10).map { i in
            row(hanzi: "X\(i)", tl: "x\(i)", count: Int64(i + 1), source: .dict)
        }
        let result = RustEngineBridge.nextwordFilter(
            raw: raw,
            queryGeneration: gen,
            nowMs: 1_000,
            limit: 3,
            mode: .tl, translateSwapped: false, associationRecordingEnabled: true,
            generation: envelopeGen,
        )
        XCTAssertEqual(result.predictions.count, 3)
    }

    // MARK: - Boost: partition reorder

    func testBoost_emptyPredictedReturnsInputUntouched() {
        let words = ["alpha", "beta", "gamma"]
        let result = RustEngineBridge.nextwordBoostCandidates(
            words: words,
            predictedFirstChars: [],
            mode: .tl, translateSwapped: false, associationRecordingEnabled: true,
            generation: envelopeGen,
        )
        XCTAssertEqual(result, words)
    }

    func testBoost_partitionsMatchingFirstCharToFront() {
        let words = ["甲級", "九份", "alpha", "九龍"]
        let result = RustEngineBridge.nextwordBoostCandidates(
            words: words,
            predictedFirstChars: ["九"],
            mode: .tl, translateSwapped: false, associationRecordingEnabled: true,
            generation: envelopeGen,
        )
        // 九-prefixed entries float to the top, original order preserved.
        XCTAssertEqual(result, ["九份", "九龍", "甲級", "alpha"])
    }

    func testBoost_noMatchesReturnsInputOrderUnchanged() {
        let words = ["alpha", "beta"]
        let result = RustEngineBridge.nextwordBoostCandidates(
            words: words,
            predictedFirstChars: ["九"],
            mode: .tl, translateSwapped: false, associationRecordingEnabled: true,
            generation: envelopeGen,
        )
        XCTAssertEqual(result, words)
    }

    // MARK: - Helpers

    private func wordSelected(
        text: String,
        roman: String,
        nowMs: Int64,
        requireRomanMode: Bool = false,
        triggerPrediction: Bool = true,
        translateSwapped: Bool = false,
        associationRecordingEnabled: Bool = true,
    ) -> RustEngineBridge.NextWordDecideResult {
        RustEngineBridge.nextwordWordSelected(
            text: text,
            roman: roman,
            requireRomanMode: requireRomanMode,
            triggerPrediction: triggerPrediction,
            nowMs: nowMs,
            mode: .tl,
            translateSwapped: translateSwapped,
            associationRecordingEnabled: associationRecordingEnabled,
            generation: envelopeGen,
        )
    }

    private func backspace(
        lastChar: String,
        nowMs: Int64,
    ) -> RustEngineBridge.NextWordDecideResult {
        RustEngineBridge.nextwordBackspace(
            lastChar: lastChar,
            nowMs: nowMs,
            mode: .tl,
            translateSwapped: false,
            associationRecordingEnabled: true,
            generation: envelopeGen,
        )
    }

    private func contextTimeoutFired(nowMs: Int64) -> RustEngineBridge.NextWordDecideResult {
        RustEngineBridge.nextwordContextTimeoutFired(
            nowMs: nowMs,
            mode: .tl,
            translateSwapped: false,
            associationRecordingEnabled: true,
            generation: envelopeGen,
        )
    }

    private func clearForNewComposing(nowMs: Int64) -> RustEngineBridge.NextWordDecideResult {
        RustEngineBridge.nextwordClearForNewComposing(
            nowMs: nowMs,
            mode: .tl,
            translateSwapped: false,
            associationRecordingEnabled: true,
            generation: envelopeGen,
        )
    }

    private func currentGen() -> UInt64 {
        RustEngineBridge.nextwordQueryState(
            mode: .tl,
            translateSwapped: false,
            associationRecordingEnabled: true,
            generation: envelopeGen,
        ).currentGeneration
    }

    private func row(
        hanzi: String,
        tl: String,
        count: Int64,
        lastUsedMs: Int64 = 0,
        source: RustEngineBridge.NextWordRawRow.Source,
    ) -> RustEngineBridge.NextWordRawRow {
        RustEngineBridge.NextWordRawRow(
            hanzi: hanzi, tl: tl,
            count: count, lastUsedMs: lastUsedMs,
            source: source,
        )
    }
}
