import SwiftProtobuf
@testable import TaigiKeyboard
import XCTest

/// v3.5.8 Phase 7A — iOS bridge surface tests for the continuous-input slice.
///
/// Scope: Swift-side decode the engine cannot see — the `candidates` nil
/// branch of `ContinuousFetchResult`, the partial-consume NextWord effect,
/// the engine-resolved commit (`earnsAutoSpace`, `ContinuousPick.request`),
/// spacing flags, the bridge-failure flag. Phase transitions, fetch, reset
/// and stale-generation behaviour are
/// engine tests (`engine/composing/tests/continuous_phase.rs`,
/// `dispatch_continuous.rs`, `continuous_commit_resolution.rs`); the
/// manager-level flow is `ComposingManagerContinuousTests`.
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
    private let settings = StubEngineSettings()
    private let hanjiFirst = StubEngineSettings(isHanjiFirst: true)

    /// The 台 / `tâi` pick over the leading `tai` of the pending buffer.
    private let taiPick = RustEngineBridge.ContinuousPick(
        script: .lead,
        roman: "tâi",
        canonicalText: "台",
        associationTl: "tâi",
        hanji: "台",
        consumedBytes: UInt32("tai".utf8.count),
        syllableCount: 1,
    )

    override func setUp() {
        super.setUp()
        Self.nextEnvelopeGen &+= 1
        envelopeGen = Self.nextEnvelopeGen
    }

    // MARK: - FetchAtPos outside Continuous (nil carrier)

    func testFetchAtPos_FromIdle_CandidatesIsNil() {
        // Phase::Idle → engine returns snapshot without `continuous` carrier.
        let result = RustEngineBridge.composingFetchAtPos(
            settings: settings, generation: envelopeGen,
            nowMs: 0,
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

    // MARK: - CommitContinuous mid-commit emits NextWordUpdateLastSelectedWord

    func testCommitContinuous_PartialConsume_EmitsNextWordUpdateLastSelectedWord() {
        // Multi-syllable buffer; consume only the leading syllable so the
        // engine stays in Continuous (mid-commit branch). Phase 4 contract
        // emits NextWordUpdateLastSelectedWord (NOT WordSelected) in this
        // path because the user is composing a sentence, not finalizing.
        _ = RustEngineBridge.composingStart(
            "taibak", settings: settings, generation: envelopeGen,
        )
        let commit = RustEngineBridge.composingCommitContinuous(
            taiPick,
            settings: settings,
            generation: envelopeGen,
        ).transition
        XCTAssertTrue(commit.isComposing, "Mid-commit must stay in Continuous phase")
        XCTAssertEqual(commit.rawInput, "bak", "Pending tail should remain after mid-commit")
        let hasUpdate = commit.effects.contains { effect in
            if case .nextWordUpdateLastSelectedWord = effect {
                return true
            }
            return false
        }
        XCTAssertTrue(
            hasUpdate,
            "Phase 4 mid-commit contract: emits NextWordUpdateLastSelectedWord; got \(commit.effects)",
        )
    }

    // MARK: - Spacing flags ride every rendering op

    // INVARIANT_EVERY_COMPOSING_OP_CARRIES_THE_RENDERING_CONFIG (behavioral-invariants.md §54)
    /// The keystroke after a nail re-renders the nailed prefix from the
    /// request's own settings, so every op builds its config from the same
    /// snapshot. Under Hanji-first (`isHanjiFirst`, no both-scripts) the
    /// prefix has no word-boundary space: `台` + `bak` reads `台bak`, never
    /// `台 bak` (desktop #31 / S37).
    func testAppendAfterNail_HanjiFirst_KeepsPrefixUnspaced() {
        _ = RustEngineBridge.composingStart(
            "taibak", settings: hanjiFirst, generation: envelopeGen,
        )
        _ = RustEngineBridge.composingCommitContinuous(
            taiPick,
            settings: hanjiFirst,
            generation: envelopeGen,
        )
        let append = RustEngineBridge.composingAppend(
            "k", settings: hanjiFirst, generation: envelopeGen,
        )
        XCTAssertEqual(append.displayText, "台bakk", "hanji-first Append must not insert a word-boundary space")
        let delete = RustEngineBridge.composingDeleteBackward(
            settings: hanjiFirst, generation: envelopeGen,
        )
        XCTAssertEqual(delete.displayText, "台bak", "hanji-first DeleteBackward must not insert a word-boundary space")
    }

    // INVARIANT_EVERY_COMPOSING_OP_CARRIES_THE_RENDERING_CONFIG (behavioral-invariants.md §54)
    /// The TPS layout with the swap stored OFF: the bridge sends `"tps"` and the stored swap, and
    /// the engine reads the layout as Hanji-first itself — the prefix after a nail stays unspaced.
    func testAppendAfterNail_TpsLayoutSwapStoredOff_KeepsPrefixUnspaced() {
        let tps = StubEngineSettings(inputMode: .tps, isHanjiFirst: false)
        _ = RustEngineBridge.composingStart(
            "taibak", settings: tps, generation: envelopeGen,
        )
        _ = RustEngineBridge.composingCommitContinuous(
            taiPick,
            settings: tps,
            generation: envelopeGen,
        )
        let append = RustEngineBridge.composingAppend(
            "k", settings: tps, generation: envelopeGen,
        )
        XCTAssertEqual(append.displayText, "台bakk", "TPS Append must not insert a word-boundary space")
        let delete = RustEngineBridge.composingDeleteBackward(
            settings: tps, generation: envelopeGen,
        )
        XCTAssertEqual(delete.displayText, "台bak", "TPS DeleteBackward must not insert a word-boundary space")
    }

    // MARK: - Engine-resolved commits (R5)

    /// The engine writes what the pick resolves to and answers the auto-space
    /// verdict: roman-led TL writes the romanization and earns the space.
    func testCommitContinuous_RomanLedFinalPick_WritesRomanization_EarnsAutoSpace() {
        _ = RustEngineBridge.composingStart("tai", settings: settings, generation: envelopeGen)
        let result = RustEngineBridge.composingCommitContinuous(taiPick, settings: settings, generation: envelopeGen)
        XCTAssertEqual(result.outcome, .finalized(earnsAutoSpace: true))
        XCTAssertTrue(result.transition.effects.contains(.commitTextReplacingPreedit("tâi")), "\(result.transition.effects)")
    }

    /// R5 P2 (parity, USER 2026-09-30): on the TPS layout a hanji-less pick writes the Bopomofo
    /// its cell shows (`tlDisplayToTPS(roman)`), which earns no space. Was: the view-rewritten
    /// `tlNumericToTPS` of the display romanization. Android pins the same row engine-side
    /// (`commit_text.rs` `tps_truth_table`).
    func testCommitContinuous_TpsHanjilessPick_WritesItsBopomofo_EarnsNoSpace() {
        let tps = StubEngineSettings(inputMode: .tps)
        _ = RustEngineBridge.composingStart("tai", settings: tps, generation: envelopeGen)
        let hanjiless = RustEngineBridge.ContinuousPick(
            script: .lead,
            roman: "tâi",
            canonicalText: "tâi",
            associationTl: "tâi",
            hanji: nil,
            consumedBytes: UInt32("tai".utf8.count),
            syllableCount: 1,
        )
        let result = RustEngineBridge.composingCommitContinuous(hanjiless, settings: tps, generation: envelopeGen)
        let bopomofo = RustEngineBridge.tlDisplayToTPS("tâi", orMapsToER: tps.isTpsOrMappedToER)
        XCTAssertEqual(bopomofo, "ㄉㄞˊ")
        XCTAssertEqual(result.outcome, .finalized(earnsAutoSpace: false))
        XCTAssertTrue(result.transition.effects.contains(.commitTextReplacingPreedit(bopomofo)), "\(result.transition.effects)")
    }

    // MARK: - Dictionary toggles

    /// The fetch carries the toggles from `settings` and the engine filters by
    /// them; the user's custom dictionary is left out so only `dictionary.bin`
    /// rows carry hanji. Mirrors engine `golden_fetch_at_pos.rs`
    /// `fetch_at_pos_resolves_the_dictionary_toggles_it_carries` and desktop-core
    /// `engine_roundtrip.rs` `all_sources_off_fetches_no_dictionary_candidates`.
    // INVARIANT_DICTIONARIES_ALL_OFF_OFFERS_NO_DICTIONARY_CANDIDATES (behavioral-invariants.md §57)
    func testFetchAtPos_EveryDictionaryOff_OffersNoDictionaryCandidates() {
        var allOff = settings
        allOff.isMoeDictEnabled = false
        allOff.isNewwordDictEnabled = false
        allOff.isITaigiDictEnabled = false
        allOff.isTaiwanPlantDictEnabled = false
        allOff.isTaiHuaDictEnabled = false
        allOff.isTaiwanJapanDictEnabled = false
        allOff.isKunggeDictEnabled = false
        allOff.isSttiDictEnabled = false
        allOff.isKhpooDictEnabled = false
        allOff.isVariantEnabled = false
        allOff.isKhiinEnabled = false
        allOff.isLkkDictEnabled = false
        allOff.isDevDictEnabled = false

        XCTAssertFalse(fetchedHanji("taigi", settings: settings).isEmpty, "the default toggles offer dictionary hanji")
        // Draw from the shared counter so the next test's `setUp` still gets
        // an unused generation (and the engine resets for it).
        Self.nextEnvelopeGen &+= 1
        envelopeGen = Self.nextEnvelopeGen
        XCTAssertEqual(fetchedHanji("taigi", settings: allOff), [])
    }

    private func fetchedHanji(_ raw: String, settings: StubEngineSettings) -> [String] {
        _ = RustEngineBridge.composingStart(raw, settings: settings, generation: envelopeGen)
        let result = RustEngineBridge.composingFetchAtPos(
            settings: settings, generation: envelopeGen,
            nowMs: 0,
            customDictionaryDisabled: true,
        )
        return (result.candidates ?? []).compactMap(\.hanji)
    }

    // MARK: - isBridgeFailure flag

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
    func testCandidateMessage_RomanField_RoundTripsThroughWire() throws {
        var msg = Taigi_Engine_CandidateMessage()
        msg.roman = "tâi-uân"
        let data = try msg.serializedData()
        let decoded = try Taigi_Engine_CandidateMessage(serializedBytes: data)
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
    func testCandidateMessage_HanjiOptional_AbsentVsPresentEmpty() throws {
        // Default-constructed message has hanji absent.
        let absent = Taigi_Engine_CandidateMessage()
        XCTAssertFalse(absent.hasHanji, "default-constructed must have hanji absent")

        // Wire round-trip preserves absence.
        let absentData = try absent.serializedData()
        let decodedAbsent = try Taigi_Engine_CandidateMessage(serializedBytes: absentData)
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
        let decodedPresent = try Taigi_Engine_CandidateMessage(serializedBytes: presentData)
        XCTAssertTrue(decodedPresent.hasHanji)
        XCTAssertEqual(decodedPresent.hanji, "")

        // Non-empty content also wire-round-trips with presence.
        var presentHant = Taigi_Engine_CandidateMessage()
        presentHant.hanji = "臺灣"
        let hantData = try presentHant.serializedData()
        let decodedHant = try Taigi_Engine_CandidateMessage(serializedBytes: hantData)
        XCTAssertTrue(decodedHant.hasHanji)
        XCTAssertEqual(decodedHant.hanji, "臺灣")
    }
}
