@testable import TaigiKeyboard
import SwiftProtobuf
import XCTest

/// v3.5.8 Phase 7A — iOS bridge surface tests for the continuous-input slice.
///
/// Scope (per pre-impl Codex consult, Q4 = A):
/// - 4 ContinuousRequest wrappers compile + round-trip via the FFI seam,
/// - `ContinuousFetchResult` tri-state semantics (nil / empty / non-empty)
///   on the `candidates` field,
/// - 3 Phase 4 NextWord effect cases decode through `synthComposing`
///   instead of being dropped to nil (the pre-7A behavior).
///
/// Functional verification of dispatch logic is in
/// `engine/composing/tests/dispatch_continuous.rs`; engine state machine
/// coverage is in `engine/composing/tests/continuous_phase.rs`. This file
/// is the iOS-side boundary check.
final class RustEngineBridgeContinuousTests: XCTestCase {
    override class func setUp() {
        super.setUp()
        RustEngineBridge.install()
    }

    /// Per-test envelope generation. Engine `EngineHandle::handle` resets
    /// state on mismatch BEFORE applying the request, so every test starts
    /// with a clean phase regardless of prior test residue.
    private static var nextEnvelopeGen: UInt64 = 200_000
    private var envelopeGen: UInt64 = 0
    private let toggles = ToneToggles(isDoubleTapOOEnabled: false, isDoubleTapNNEnabled: false)

    override func setUp() {
        super.setUp()
        Self.nextEnvelopeGen &+= 1
        envelopeGen = Self.nextEnvelopeGen
    }

    // MARK: - EnterContinuous

    func testEnterContinuous_FromComposing_PreservesRawAndStaysComposing() {
        _ = RustEngineBridge.composingStart(
            "tsua", mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        let enter = RustEngineBridge.composingEnterContinuous(
            mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        XCTAssertTrue(enter.isComposing)
        XCTAssertEqual(enter.rawInput, "tsua")
        // EnterContinuous is a phase swap — no document mutation.
        XCTAssertTrue(enter.effects.isEmpty, "expected no effects, got \(enter.effects)")
    }

    func testEnterContinuous_FromIdle_NoOp() {
        let enter = RustEngineBridge.composingEnterContinuous(
            mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        XCTAssertFalse(enter.isComposing)
        XCTAssertEqual(enter.rawInput, "")
        XCTAssertTrue(enter.effects.isEmpty)
    }

    // MARK: - FetchAtPos tri-state

    func testFetchAtPos_FromIdle_CandidatesIsNil() {
        // Phase::Idle → engine returns snapshot without `continuous` carrier.
        let result = RustEngineBridge.composingFetchAtPos(
            mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        XCTAssertNil(result.candidates, "Idle phase must yield nil candidates (carrier absent)")
        XCTAssertFalse(result.transition.isComposing)
        // Read-only RPC emits no effects.
        XCTAssertTrue(result.transition.effects.isEmpty)
        XCTAssertFalse(
            result.isBridgeFailure,
            "Successful Idle dispatch must not flag as bridge failure (Codex r3216857164)",
        )
    }

    func testFetchAtPos_FromComposing_NotYetContinuous_CandidatesIsNil() {
        _ = RustEngineBridge.composingStart(
            "tsua", mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        // Composing phase is NOT Continuous — handle_fetch_at_pos returns
        // snapshot without continuous carrier.
        let result = RustEngineBridge.composingFetchAtPos(
            mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        XCTAssertNil(result.candidates, "Composing phase must yield nil candidates")
        XCTAssertTrue(result.transition.isComposing)
        XCTAssertFalse(
            result.isBridgeFailure,
            "Successful non-Continuous dispatch must not flag as bridge failure",
        )
    }

    func testFetchAtPos_FromContinuous_CandidatesNonNil() {
        // Continuous phase → carrier is present (Some). Whether it contains
        // hits depends on whether the lexicon was installed with a syllable
        // inventory. Unit tests do not install lexicon, so the carrier is
        // present-but-empty (the tri-state distinction we want to lock).
        _ = RustEngineBridge.composingStart(
            "tsua", mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        _ = RustEngineBridge.composingEnterContinuous(
            mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        let result = RustEngineBridge.composingFetchAtPos(
            mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        XCTAssertNotNil(
            result.candidates,
            "Continuous phase must yield non-nil candidates carrier (engine sets continuous=Some)",
        )
        XCTAssertTrue(result.transition.isComposing)
        XCTAssertTrue(
            result.transition.effects.isEmpty,
            "FetchAtPos is read-only; effects must be empty",
        )
        XCTAssertFalse(
            result.isBridgeFailure,
            "Successful Continuous dispatch must not flag as bridge failure",
        )
    }

    // MARK: - ResetContinuous emits NextWordClearForNewComposing

    func testResetContinuous_FromContinuous_EmitsNextWordClearForNewComposing() {
        _ = RustEngineBridge.composingStart(
            "tsua", mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        _ = RustEngineBridge.composingEnterContinuous(
            mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        let reset = RustEngineBridge.composingResetContinuous(generation: envelopeGen)
        XCTAssertFalse(reset.isComposing, "ResetContinuous must exit to Idle")
        let hasClear = reset.effects.contains { effect in
            if case .nextWordClearForNewComposing = effect { return true }
            return false
        }
        XCTAssertTrue(
            hasClear,
            "Phase 4 contract: ResetContinuous emits NextWordClearForNewComposing; got \(reset.effects)",
        )
    }

    // MARK: - CommitContinuous final-commit emits NextWordWordSelected

    func testCommitContinuous_FullConsume_EmitsNextWordWordSelected() {
        _ = RustEngineBridge.composingStart(
            "tai", mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        _ = RustEngineBridge.composingEnterContinuous(
            mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        // consumed_bytes = "tai".utf8.count → final commit
        let commit = RustEngineBridge.composingCommitContinuous(
            displayText: "台",
            consumedBytes: UInt32("tai".utf8.count),
            syllableCount: 1,
            mode: .tl,
            toggles: toggles,
            generation: envelopeGen,
        )
        let hasSelected = commit.effects.contains { effect in
            if case .nextWordWordSelected = effect { return true }
            return false
        }
        XCTAssertTrue(
            hasSelected,
            "Phase 4 final-commit contract: CommitContinuous emits NextWordWordSelected; got \(commit.effects)",
        )
    }

    // MARK: - CommitContinuous mid-commit emits NextWordUpdateLastSelectedWord

    func testCommitContinuous_PartialConsume_EmitsNextWordUpdateLastSelectedWord() {
        // Multi-syllable buffer; consume only the leading syllable so the
        // engine stays in Continuous (mid-commit branch). Phase 4 contract
        // emits NextWordUpdateLastSelectedWord (NOT WordSelected) in this
        // path because the user is composing a sentence, not finalizing.
        _ = RustEngineBridge.composingStart(
            "taibak", mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        _ = RustEngineBridge.composingEnterContinuous(
            mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        let commit = RustEngineBridge.composingCommitContinuous(
            displayText: "台",
            consumedBytes: UInt32("tai".utf8.count),
            syllableCount: 1,
            mode: .tl,
            toggles: toggles,
            generation: envelopeGen,
        )
        XCTAssertTrue(commit.isComposing, "Mid-commit must stay in Continuous phase")
        XCTAssertEqual(commit.rawInput, "bak", "Pending tail should remain after mid-commit")
        let hasUpdate = commit.effects.contains { effect in
            if case .nextWordUpdateLastSelectedWord = effect { return true }
            return false
        }
        XCTAssertTrue(
            hasUpdate,
            "Phase 4 mid-commit contract: emits NextWordUpdateLastSelectedWord; got \(commit.effects)",
        )
    }

    // MARK: - Effect mapping completeness (no nil drops)

    /// Phase 7A removed the three `nil` returns at the bottom of
    /// `synthComposing` (pre-Phase-7A behavior). Verify by triggering a
    /// reset path that the engine populates with NextWord effects, and
    /// confirming none are silently dropped.
    func testNextWordEffects_AreNeverDroppedToNil() {
        _ = RustEngineBridge.composingStart(
            "tai", mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        _ = RustEngineBridge.composingEnterContinuous(
            mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        let reset = RustEngineBridge.composingResetContinuous(generation: envelopeGen)
        // Pre-Phase-7A this would silently filter NextWord effects out.
        // Now they must appear in the effect list — the count cannot be
        // less than the engine emits.
        let nextWordEffects = reset.effects.filter { effect in
            switch effect {
            case .nextWordUpdateLastSelectedWord,
                 .nextWordWordSelected,
                 .nextWordClearForNewComposing:
                return true
            default:
                return false
            }
        }
        XCTAssertFalse(
            nextWordEffects.isEmpty,
            "ResetContinuous must surface at least one NextWord effect post-Phase-7A",
        )
    }

    // MARK: - FetchAtPos read-only invariance

    func testFetchAtPos_DoesNotMutateBuffer() {
        _ = RustEngineBridge.composingStart(
            "tsua", mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        _ = RustEngineBridge.composingEnterContinuous(
            mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        let before = RustEngineBridge.composingQueryState(generation: envelopeGen)
        _ = RustEngineBridge.composingFetchAtPos(
            mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        let after = RustEngineBridge.composingQueryState(generation: envelopeGen)
        XCTAssertEqual(before.rawInput, after.rawInput, "rawInput must not change across FetchAtPos")
        XCTAssertEqual(before.isComposing, after.isComposing, "isComposing must not change")
        XCTAssertEqual(
            before.selectedCandidateIndex,
            after.selectedCandidateIndex,
            "selectedCandidateIndex must not change",
        )
    }

    // MARK: - Phase 9.2 mode carrier

    /// Decode mapping from `Taigi_Engine_CandidateMode` (wire integer) to
    /// the Swift `RustEngineBridge.CandidateMode` enum. Pins the four
    /// wire values (UNSPECIFIED=0, HANT=1, TAILO=2, MIXED=3) so a future
    /// proto reshuffle would fail this test before reaching the UI layer.
    func testCandidateModeDecode_AllWireValues() {
        XCTAssertEqual(RustEngineBridge.CandidateMode.decode(0), .unspecified)
        XCTAssertEqual(RustEngineBridge.CandidateMode.decode(1), .hant)
        XCTAssertEqual(RustEngineBridge.CandidateMode.decode(2), .tailo)
        XCTAssertEqual(RustEngineBridge.CandidateMode.decode(3), .mixed)
    }

    /// Forward-compat: a wire value the platform binding doesn't recognize
    /// (e.g. a newer engine added a fourth variant) must collapse to
    /// `.unspecified` rather than crash or randomly map. Pins F8 of the
    /// Codex pre-impl decision matrix.
    func testCandidateModeDecode_UnknownWireValueFallsBackToUnspecified() {
        XCTAssertEqual(RustEngineBridge.CandidateMode.decode(99), .unspecified)
        XCTAssertEqual(RustEngineBridge.CandidateMode.decode(-1), .unspecified)
    }

    // MARK: - Phase 9.3b user-freq snapshot wire shape

    /// Bridge-acceptance only: the v3.5.8 Phase 9.3b widening of
    /// `composingFetchAtPos` accepts a populated `frequencyEntries` +
    /// `nowMs` snapshot, round-trips it through the FFI envelope, and
    /// preserves the carrier-present + read-only invariants. The actual
    /// `user_freq_boost` / `recency_rank` ranking behaviour is pinned
    /// hermetically by Rust in `engine/lexicon/tests/user_freq_plumb.rs`;
    /// this test does NOT re-assert ranking math.
    func testFetchAtPos_AcceptsFrequencyEntriesAndNowMs() {
        _ = RustEngineBridge.composingStart(
            "tsua", mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        _ = RustEngineBridge.composingEnterContinuous(
            mode: .tl, toggles: toggles, generation: envelopeGen,
        )

        var entry = Taigi_Engine_FrequencyEntry()
        entry.displayTextKey = "珠仔"
        entry.count = 7
        entry.lastUsedMs = 1_700_000_000_000

        let result = RustEngineBridge.composingFetchAtPos(
            mode: .tl,
            toggles: toggles,
            generation: envelopeGen,
            frequencyEntries: [entry],
            nowMs: 1_700_000_001_000,
        )

        // Carrier present proves the dispatcher reached
        // `handle_fetch_at_pos` with the populated payload (Rust counterpart:
        // `fetch_at_pos_carries_user_freq_snapshot_through_decode` in
        // `engine/composing/tests/dispatch_continuous.rs`). Unit tests do
        // not install the lexicon, so the candidate list itself is empty.
        XCTAssertNotNil(
            result.candidates,
            "Continuous phase must yield a non-nil candidates carrier even with empty FST",
        )
        XCTAssertTrue(result.transition.isComposing)
        // FetchAtPos read-only contract holds regardless of snapshot payload.
        XCTAssertTrue(
            result.transition.effects.isEmpty,
            "FetchAtPos with user-freq snapshot is still read-only; effects must be empty",
        )
        XCTAssertFalse(
            result.isBridgeFailure,
            "Successful populated dispatch must not flag as bridge failure",
        )
    }

    /// Default-parameter call site continues to compile + behave identically
    /// to PR-9.2. Pins the API surface against accidental removal of the
    /// `[]` / `0` defaults (which the platform 7B call sites rely on).
    func testFetchAtPos_DefaultParameters_PreserveNeutralBehavior() {
        _ = RustEngineBridge.composingStart(
            "tsua", mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        _ = RustEngineBridge.composingEnterContinuous(
            mode: .tl, toggles: toggles, generation: envelopeGen,
        )

        let result = RustEngineBridge.composingFetchAtPos(
            mode: .tl, toggles: toggles, generation: envelopeGen,
        )
        XCTAssertNotNil(result.candidates, "Default-parameter call must still reach Continuous")
        XCTAssertTrue(result.transition.effects.isEmpty)
        XCTAssertFalse(result.isBridgeFailure)
    }

    // MARK: - Phase 9.3b isBridgeFailure flag

    /// Static `.noop` is the only producer of `isBridgeFailure == true`.
    /// Pins the invariant that a successful round-trip — including an
    /// Idle snapshot — never appears as the static `.noop`. Caller code
    /// in `ComposingManager.fetchContinuousCandidates` relies on this
    /// to distinguish FFI failure from engine reset. Codex PR #265
    /// r3216857164.
    func testNoopStatic_IsFlaggedAsBridgeFailure() {
        XCTAssertTrue(
            RustEngineBridge.ContinuousFetchResult.noop.isBridgeFailure,
            "ContinuousFetchResult.noop must signal bridge failure — the static is " +
                "synthesized only when composingProtoRoundtrip fails",
        )
        XCTAssertNil(RustEngineBridge.ContinuousFetchResult.noop.candidates)
        XCTAssertEqual(
            RustEngineBridge.ContinuousFetchResult.noop.transition,
            RustEngineBridge.ComposingTransition.noop,
        )
    }

    // MARK: - v3.5.8 Phase 9 Item 5 — `roman` / `hanji` wire schema

    /// `string roman = 8` is non-optional; SwiftProtobuf round-trips it
    /// verbatim. Empty string is the default; explicit assignment of a
    /// non-empty value must survive a serialize/deserialize pair so
    /// the bridge decode path `roman: msg.roman` produces the same
    /// String the engine emitted.
    // 中文: Item 5 — roman 為非 optional;Swift wire round-trip 必須保留原值。
    func testCandidateMessage_RomanField_RoundTripsThroughWire() throws {
        var msg = Taigi_Engine_CandidateMessage()
        msg.roman = "tâi-uân"
        let data = try msg.serializedData()
        let decoded = try Taigi_Engine_CandidateMessage(serializedData: data)
        XCTAssertEqual(decoded.roman, "tâi-uân")
    }

    /// `optional string hanji = 9` distinguishes "field absent on the
    /// wire" (TAILO candidate — `hasHanji == false`) from "field set
    /// to empty string" (defective producer — `hasHanji == true`,
    /// `hanji == ""`). The bridge decode rule `msg.hasHanji ? msg.hanji
    /// : nil` relies on this presence accessor; if SwiftProtobuf ever
    /// stopped distinguishing absence from empty (e.g. due to a proto
    /// regen drift), bridge consumers would mis-classify TAILO
    /// candidates as `hanji == ""` and the dual-line render rule from
    /// `docs/engine/continuous-candidate-display.md` §5 would break.
    // 中文: Item 5 — hanji 為 proto3 optional;wire absent vs Some("") 必須由 hasHanji 區分。
    func testCandidateMessage_HanjiOptional_AbsentVsPresentEmpty() throws {
        // Default-constructed message has hanji absent.
        let absent = Taigi_Engine_CandidateMessage()
        XCTAssertFalse(absent.hasHanji, "default-constructed must have hanji absent")

        // Wire round-trip preserves absence.
        let absentData = try absent.serializedData()
        let decodedAbsent = try Taigi_Engine_CandidateMessage(serializedData: absentData)
        XCTAssertFalse(
            decodedAbsent.hasHanji,
            "absence survives wire round-trip — TAILO candidates must decode to hanji nil",
        )

        // Explicit empty-string set flips presence to true.
        var presentEmpty = Taigi_Engine_CandidateMessage()
        presentEmpty.hanji = ""
        XCTAssertTrue(
            presentEmpty.hasHanji,
            "explicit empty-string assignment flips presence — distinguishes 'producer set field' from 'absent'",
        )
        let presentData = try presentEmpty.serializedData()
        let decodedPresent = try Taigi_Engine_CandidateMessage(serializedData: presentData)
        XCTAssertTrue(decodedPresent.hasHanji)
        XCTAssertEqual(decodedPresent.hanji, "")

        // Non-empty content also wire-round-trips with presence.
        var presentHant = Taigi_Engine_CandidateMessage()
        presentHant.hanji = "臺灣"
        let hantData = try presentHant.serializedData()
        let decodedHant = try Taigi_Engine_CandidateMessage(serializedData: hantData)
        XCTAssertTrue(decodedHant.hasHanji)
        XCTAssertEqual(decodedHant.hanji, "臺灣")
    }
}
