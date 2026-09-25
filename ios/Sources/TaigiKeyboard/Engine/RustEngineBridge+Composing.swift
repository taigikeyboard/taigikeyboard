import Foundation
import SwiftProtobuf

// MARK: - RustEngineBridge Composing surface

/// Composing slice extension for `RustEngineBridge`. Holds 10 composing
/// ops + 4 continuous-input ops (v3.5.8 Phase 6) + their synthesized value
/// types (`ComposingTransition` / `CandidateMode` / `ContinuousCandidate` /
/// `ContinuousFetchResult`) + the composing-specific dispatch helpers
/// (`composingProtoRoundtrip` / `composingDispatch` / `composingFetchDispatch`
/// / `synthComposing` / `continuousAppConfig`). All five helpers stay
/// `private` within this file — they are file-local to the composing
/// surface.
public extension RustEngineBridge {
    // MARK: - Synthesized value types

    /// Bridge-synthesized companion to the proto `ComposingResponse`.
    /// Consumed by `ComposingManager` and its delegate.
    struct ComposingTransition: Equatable {
        public enum Effect: Equatable {
            case updatePreedit(String)
            case clearPreeditWithoutCommit
            case commitTextReplacingPreedit(String)
            case deleteBackwardFromDocument
            case resetAutocomplete
            case performAutocomplete
            case resetAutocompleteContext
            /// v3.5.8 Phase 4 — continuous-input mid-commit handshake. Maps to
            /// `NextWordRequest::UpdateLastSelectedWord(text, roman, now_ms)`.
            /// Platform delegate forwards to `NextWordController.updateLastSelectedWord`
            /// which injects `nowMs` + envelope generation.
            case nextWordUpdateLastSelectedWord(text: String, roman: String)
            /// v3.5.8 Phase 4 — continuous-input final-commit handshake. Maps to
            /// `NextWordRequest::WordSelected(text, roman, require_roman_mode=false,
            /// trigger_prediction, now_ms)`. Forward `triggerPrediction` exactly —
            /// hardcoding either value breaks the Phase 4 commit contract.
            case nextWordWordSelected(text: String, roman: String, triggerPrediction: Bool)
            /// v3.5.8 Phase 4 — continuous-input abort handshake. Maps to
            /// `NextWordRequest::ClearForNewComposing(now_ms)`. Platform delegate
            /// forwards to `NextWordController.clearDisplay()` (NOT
            /// `resetAndClearUI()` — that sends the structurally distinct
            /// `ResetFull` intent).
            case nextWordClearForNewComposing
            /// Learned phrases (§50) — the final continuous commit was a
            /// sequence of hanji picks; the platform upserts the
            /// `(hanji, canonicalTl)` pair into its learned store.
            case phraseLearned(hanji: String, canonicalTl: String)
        }

        public let rawInput: String
        public let displayText: String
        public let effects: [Effect]
        public let isComposing: Bool

        public static let noop = ComposingTransition(
            rawInput: "",
            displayText: "",
            effects: [],
            isComposing: false,
        )
    }

    /// MOE-aligned candidate-type discriminator. Wire mirror of
    /// `protos::engine::CandidateMode` (Phase 9.2). Engine derives in
    /// Rust from `DictionaryRecord.hanzi` presence + NFKD-normalized
    /// Latin-letter detection (`engine/lexicon/src/continuous.rs::
    /// derive_mode`); platforms read but never recompute (no
    /// display-text sniffing — that would parallel-implement the
    /// derive and violate `.claude/rules/cross-platform-alignment.md`).
    ///
    /// Metadata-only in v3.5.8 — does NOT enter the engine's `SortKey`
    /// tie-break (per `docs/releases/v3.5.8/plan.md` § Phase 9 R2 Q3.a). `.unspecified`
    /// is the proto3 default and means "unknown carrier — old engine or
    /// dropped field"; it is never emitted by the current Rust engine.
    /// Platforms must treat `.unspecified` as "ignore mode" rather than
    /// falling back to any local classification.
    enum CandidateMode: Equatable {
        case unspecified
        case hant
        case tailo
        case mixed

        /// Decode the wire integer (`CandidateMessage.mode.rawValue`)
        /// produced by SwiftProtobuf. Unrecognized values (forward-compat
        /// from a newer engine) collapse to `.unspecified` so the
        /// platform never crashes on a binding mismatch.
        static func decode(_ wire: Int) -> CandidateMode {
            switch wire {
            case 1: .hant
            case 2: .tailo
            case 3: .mixed
            default: .unspecified
            }
        }
    }

    /// Single span-local continuous-input candidate. Wire mirror of
    /// `protos::engine::CandidateMessage` (Phase 6 + 9.2 `mode`).
    ///
    /// `consumedSpanStart` / `consumedSpanEnd` are byte offsets into the
    /// **original raw user input** stored in `Phase::Continuous { raw }` —
    /// TL/POJ users → ASCII bytes, TPS users → Bopomofo bytes. Platform UI
    /// slices `pending[start..<end]` on commit. `form` is currently always 1
    /// (FORM_NOTONE). `mode` is the Phase 9.2 carrier; metadata-only.
    struct ContinuousCandidate: Equatable {
        public let consumedSpanStart: UInt32
        public let consumedSpanEnd: UInt32
        public let syllableCount: UInt32
        public let displayText: String
        public let score: Float
        public let form: UInt32
        public let mode: CandidateMode
        /// v3.5.8 Phase 9 Item 5 — display-romanization sidechannel for
        /// dual-line cell render. Always non-empty for dictionary-
        /// sourced candidates; the engine renders it for the active
        /// input mode (TL, or POJ-display in POJ mode). UI reads
        /// `displayText` for commit / `user_frequency.db` writes and
        /// `roman` only for cell-title display.
        public let roman: String
        /// v3.5.8 Phase 9 Item 5 — hanji display sidechannel. `nil`
        /// iff the proto3 `optional string hanji` was absent on the
        /// wire (TAILO candidate). Present-empty is treated as
        /// present (engine never emits `Some("")` today; defensive).
        public let hanji: String?
        /// v3.6.1 R2 — canonical TL identity sidechannel
        /// (`CandidateMessage.canonical_tl`). Unlike `roman` (the
        /// POJ-rendered display form in POJ mode), this stays the
        /// canonical TL the `(hanji, canonical-TL)` word identity is keyed
        /// on. The tap path round-trips it into
        /// `commitContinuous(associationTl:)` so the NextWord association
        /// learns the same TL a normal candidate commit records. Empty
        /// only for TPS-OOV hanji-absent candidates with no dict TL.
        public let canonicalTl: String

        public init(
            consumedSpanStart: UInt32,
            consumedSpanEnd: UInt32,
            syllableCount: UInt32,
            displayText: String,
            score: Float,
            form: UInt32,
            mode: CandidateMode,
            roman: String,
            hanji: String?,
            canonicalTl: String,
        ) {
            self.consumedSpanStart = consumedSpanStart
            self.consumedSpanEnd = consumedSpanEnd
            self.syllableCount = syllableCount
            self.displayText = displayText
            self.score = score
            self.form = form
            self.mode = mode
            self.roman = roman
            self.hanji = hanji
            self.canonicalTl = canonicalTl
        }
    }

    /// Read-query result for `composingFetchAtPos`. The `candidates` tri-state
    /// is only authoritative when `isBridgeFailure == false`:
    /// - `nil` → engine reached `handle_fetch_at_pos` but `Phase::Continuous`
    ///   was not active (proto `continuous` field absent).
    /// - `[]` → continuous phase active but no candidates (no syllable
    ///   inventory installed, no FST hits, or `position != 0`).
    /// - non-empty → candidates returned in score-desc order.
    ///
    /// `transition` carries the engine snapshot (preedit / `isComposing`);
    /// FetchAtPos is read-only so its `effects` is empty.
    ///
    /// `isBridgeFailure` distinguishes "the engine returned Idle" (legit
    /// generation-mismatch reset; `transition` reflects the new Idle state,
    /// caller should `apply()` it) from "the FFI roundtrip itself failed"
    /// (encode / decode / non-OK engine response; `transition == .noop`
    /// is synthesized and `apply()`-ing it would clobber the mirror with
    /// false state). Set `true` only on the `composingFetchDispatch` early-
    /// return path via `ContinuousFetchResult.noop`; every successful
    /// dispatch sets `false`. Phase 9.3b plumb relies on this to fall back
    /// to phase-1 candidates on a transient phase-2 FFI failure rather
    /// than dropping suggestions and resetting state. Codex PR #265
    /// r3216857164.
    struct ContinuousFetchResult: Equatable {
        public let transition: ComposingTransition
        public let candidates: [ContinuousCandidate]?
        public let isBridgeFailure: Bool

        public static let noop = ContinuousFetchResult(
            transition: .noop,
            candidates: nil,
            isBridgeFailure: true,
        )
    }

    // MARK: Composing slice (10 ops)

    internal static func composingStart(
        _ text: String,
        settings: EngineSettings,
        generation: UInt64,
    ) -> ComposingTransition {
        var payload = Taigi_Engine_Start()
        payload.text = text
        return composingDispatch(
            method: .start(payload),
            op: "composingStart",
            generation: generation,
            config: continuousAppConfig(settings),
        )
    }

    internal static func composingAppend(
        _ char: String,
        settings: EngineSettings,
        generation: UInt64,
    ) -> ComposingTransition {
        var payload = Taigi_Engine_Append()
        payload.char = char
        return composingDispatch(
            method: .append(payload),
            op: "composingAppend",
            generation: generation,
            config: continuousAppConfig(settings),
        )
    }

    internal static func composingAppendHyphen(
        settings: EngineSettings,
        generation: UInt64,
    ) -> ComposingTransition {
        composingDispatch(
            method: .appendHyphen(Taigi_Engine_AppendHyphen()),
            op: "composingAppendHyphen",
            generation: generation,
            config: continuousAppConfig(settings),
        )
    }

    internal static func composingReplaceLast(
        _ replacement: String,
        settings: EngineSettings,
        generation: UInt64,
    ) -> ComposingTransition {
        var payload = Taigi_Engine_ReplaceLast()
        payload.replacement = replacement
        return composingDispatch(
            method: .replaceLast(payload),
            op: "composingReplaceLast",
            generation: generation,
            config: continuousAppConfig(settings),
        )
    }

    internal static func composingDeleteBackward(
        settings: EngineSettings,
        generation: UInt64,
    ) -> ComposingTransition {
        composingDispatch(
            method: .deleteBackward(Taigi_Engine_DeleteBackward()),
            op: "composingDeleteBackward",
            generation: generation,
            config: continuousAppConfig(settings),
        )
    }

    internal static func composingCommitDerived(
        settings: EngineSettings,
        generation: UInt64,
    ) -> ComposingTransition {
        composingDispatch(
            method: .commitDerived(Taigi_Engine_CommitDerived()),
            op: "composingCommitDerived",
            generation: generation,
            config: continuousAppConfig(settings),
        )
    }

    // Under `Phase::Continuous` the engine commits the whole composition
    // (`combined_display(nailed, pending, config)`), not the literal
    // keystrokes; the composing arm commits `raw` verbatim.
    internal static func composingCommitRaw(
        settings: EngineSettings,
        generation: UInt64,
    ) -> ComposingTransition {
        composingDispatch(
            method: .commitRaw(Taigi_Engine_CommitRaw()),
            op: "composingCommitRaw",
            generation: generation,
            config: continuousAppConfig(settings),
        )
    }

    // Under `Phase::Continuous` the engine prepends `nailed_prefix(nailed,
    // config)` to `text`; the composing arm commits `text` verbatim.
    internal static func composingSelectSuggestion(
        _ text: String,
        settings: EngineSettings,
        generation: UInt64,
    ) -> ComposingTransition {
        var payload = Taigi_Engine_SelectSuggestion()
        payload.text = text
        return composingDispatch(
            method: .selectSuggestion(payload),
            op: "composingSelectSuggestion",
            generation: generation,
            config: continuousAppConfig(settings),
        )
    }

    // E.g. an emoji tap mid-composition: the engine commits the rendered
    // composition first, then inserts `text`.
    internal static func composingCommitPreeditThenInsertExternal(
        _ text: String,
        settings: EngineSettings,
        generation: UInt64,
    ) -> ComposingTransition {
        var payload = Taigi_Engine_CommitPreeditThenInsertExternal()
        payload.text = text
        return composingDispatch(
            method: .commitPreeditThenInsertExternal(payload),
            op: "composingCommitPreeditThenInsertExternal",
            generation: generation,
            config: continuousAppConfig(settings),
        )
    }

    static func composingReset(generation: UInt64) -> ComposingTransition {
        composingDispatch(
            method: .reset(Taigi_Engine_Reset()),
            op: "composingReset",
            generation: generation,
            config: nil,
        )
    }

    // MARK: Continuous-input (4 ops) — v3.5.8 Phase 6

    /// `Phase::Composing { raw }` → `Phase::Continuous { raw, committed: [] }`.
    /// Phase 6 contract: no payload — buffer is whatever earlier `Start` /
    /// `Append` populated. Engine no-ops on Idle / already-Continuous / empty
    /// `Composing.raw`. AppConfig is required because the snapshot's preedit
    /// display goes through `derived_display(raw, config)`.
    internal static func composingEnterContinuous(
        settings: EngineSettings,
        generation: UInt64,
    ) -> ComposingTransition {
        composingDispatch(
            method: .enterContinuous(Taigi_Engine_EnterContinuous()),
            op: "composingEnterContinuous",
            generation: generation,
            config: continuousAppConfig(settings),
        )
    }

    /// Read-only candidate query for the current `Phase::Continuous { raw }`.
    /// `position` is reserved as `0` in v3.5.8 (Phase 6 dispatch validates).
    /// Caller MUST share the active composing-session generation — FetchAtPos
    /// is read-only and bumping generation would reset engine state before
    /// the fetch (`engine/composing/src/dispatch.rs:103-160`).
    ///
    /// `frequencyEntries` + `nowMs` are the v3.5.8 Phase 9.3a/9.3b plumb for
    /// `user_freq_boost` + `SortKey.recency_rank`. Caller pre-filters entries
    /// to candidate-relevant `displayTextKey`s (`hanji ?? roman`) — see
    /// `engine/protos/proto/composing.proto:144-148`. Defaults `[]` + `0`
    /// reproduce the PR-9.2 neutral-boost behaviour (`user_freq_boost = 1.0`,
    /// `recency_rank = 1` everywhere); the platform plumb is responsible for
    /// populating real values via a two-phase fetch (`ComposingManager
    /// .fetchContinuousCandidates`).
    //
    /// v3.5.8 Phase 9 Item 12 — `customEntries` carries the platform's
    /// `custom_dictionary.db` matches (raw stored `(roman, hanji)`
    /// columns; DB stays native). Default `[]` = no custom matches /
    /// feature off — backward-compatible no-op. The engine synthesizes
    /// a full-buffer candidate per entry and dedupes `(roman, hanji)`
    /// against the FST hits (custom wins the collision).
    ///
    /// v3.5.9 B-4 — `roman` may be either TL or POJ display form
    /// (whichever the user typed when storing). The engine treats it
    /// raw on the lattice / dedupe axis and folds it to canonical TL
    /// only when synthesizing the `user_frequency.db` commit key,
    /// keeping that key mode-invariant across TL/POJ.
    internal static func composingFetchAtPos(
        settings: EngineSettings,
        generation: UInt64,
        frequencyEntries: [Taigi_Engine_FrequencyEntry] = [],
        nowMs: Int64 = 0,
        customEntries: [Taigi_Engine_CustomDictEntry] = [],
        // PR-9.6 — dictionary source-toggle bitmask (same one Tab3 browse
        // sends). Default `0` = proto3-absent sentinel → engine all-on,
        // preserving pre-PR-9.6 behaviour for callers (incl. tests).
        enabledSourcesBitmask: UInt32 = 0,
        // §34/S22 — invert of the 顯示當咧拍的字 setting. Default `false` = show
        // (proto3-absent sentinel → engine prepends the literal-roman
        // candidate, the pre-toggle always-on behaviour for callers/tests).
        literalRomanCandidateDisabled: Bool = false,
        // §50 — learned phrases whose whole-buffer key equals the raw buffer
        // (`LearnedPhraseRepository.matchesSync`). Default `[]` =
        // feature off / nothing learned.
        learnedEntries: [Taigi_Engine_LearnedEntry] = [],
    ) -> ContinuousFetchResult {
        var payload = Taigi_Engine_FetchAtPos()
        payload.position = 0
        payload.frequencyEntries = frequencyEntries
        payload.nowMs = nowMs
        payload.customEntries = customEntries
        payload.enabledSourcesBitmask = enabledSourcesBitmask
        payload.literalRomanCandidateDisabled = literalRomanCandidateDisabled
        payload.learnedEntries = learnedEntries
        return composingFetchDispatch(
            method: .fetchAtPos(payload),
            op: "composingFetchAtPos",
            generation: generation,
            config: continuousAppConfig(settings),
        )
    }

    /// Commit a candidate segment in `Phase::Continuous`. `displayText` /
    /// `consumedBytes` / `syllableCount` MUST come from a `ContinuousCandidate`
    /// returned by an immediately preceding `composingFetchAtPos` call —
    /// sending mismatched values mis-aligns the committed segment.
    /// `consumedBytes >= pending.utf8.count` triggers a final commit (exit
    /// to Idle). Programmer-error inputs collapse to noop on the engine side.
    internal static func composingCommitContinuous(
        displayText: String,
        canonicalText: String,
        associationTl: String,
        // §50 — the picked candidate's hanji (`ContinuousCandidate.hanji`),
        // `nil` for a hanji-less pick; the engine learns a composition only
        // when every segment carried one.
        hanji: String? = nil,
        consumedBytes: UInt32,
        syllableCount: UInt32,
        settings: EngineSettings,
        generation: UInt64,
    ) -> ComposingTransition {
        var payload = Taigi_Engine_CommitContinuous()
        payload.displayText = displayText
        payload.canonicalText = canonicalText
        // R2: canonical TL of the chosen candidate → NextWord `next_tl` /
        // `prev_tl`. Empty → engine falls back to the raw committed slice.
        payload.associationTl = associationTl
        if let hanji, !hanji.isEmpty {
            payload.hanji = hanji
        }
        payload.consumedBytes = consumedBytes
        payload.syllableCount = syllableCount
        return composingDispatch(
            method: .commitContinuous(payload),
            op: "composingCommitContinuous",
            generation: generation,
            config: continuousAppConfig(settings),
        )
    }

    /// Abort continuous-input. Drops `Phase::Continuous` committed list +
    /// pending raw, exits to Idle, emits the standard abort effect trio
    /// (`ClearPreeditWithoutCommit` + `ResetAutocomplete` +
    /// `NextWordClearForNewComposing`). Committed segments stay in the
    /// document — earlier `CommitTextReplacingPreedit` effects already wrote
    /// them.
    static func composingResetContinuous(generation: UInt64) -> ComposingTransition {
        composingDispatch(
            method: .resetContinuous(Taigi_Engine_ResetContinuous()),
            op: "composingResetContinuous",
            generation: generation,
            config: nil,
        )
    }

    // MARK: - Private dispatch (composing envelope)

    /// Continuous-rendering `AppConfig`: base `appConfig(mode:toggles:)`
    /// plus the two v3.5.8 §10.2 word-boundary-spacing flags the engine's
    /// `continuous_word_space` predicate consumes.
    ///
    /// `effectiveSwapped` (= translate-swap OR TPS layout, combined
    /// platform-side because both platforms map TPS → `"tl"`/`"poj"`
    /// `input_mode`, so the engine's own `input_mode == "tps"` branch
    /// never fires) rides `is_translate_swapped`. `outputBothScripts`
    /// distinguishes hanji-first (no inter-segment space) from
    /// both-scripts (`hit (彼)` — space wanted); `is_translate_swapped`
    /// is `true` for both, so the second flag is required.
    ///
    /// Every composing op that renders the composition sends it — under
    /// Model B that is every mutation and every snapshot, not only the
    /// commits: `Append` / `DeleteBackward` after a nail re-render the
    /// nailed prefix through `combined_display(nailed, raw, config)` too,
    /// so a nail and the keystroke after it must agree on the prefix
    /// (the 2026-05-18 "commit entry points only" split left 漢字優先
    /// showing `台 gi` while typing after `台`; desktop closed the same
    /// drift in #31, S37). Only `Reset`, which carries no config, stays
    /// outside.
    // `candidateDisplayMode` (proto field 9) travels with the pair: under 羅馬字 the callers already
    // pass the DERIVED `(false, false)` pair, and FetchAtPos uses the mode to collapse same-roman rows.
    // CROSS-PLATFORM INVARIANT — mirrors android/app/src/main/java/com/siansiansu/taigikeyboard/engine/RustEngineBridge.kt continuousAppConfig.
    // Drift causes silent divergence (hanji-first spurious word-boundary spaces).
    private static func continuousAppConfig(_ settings: EngineSettings) -> Taigi_Engine_AppConfig {
        var cfg = appConfig(mode: settings.inputMode, toggles: settings.pojMarkerOptions)
        cfg.candidateDisplayMode = settings.candidateDisplayMode.engineValue
        // Already TPS-folded by `SharedSettings.isHyphenlessRomanEnabled` (§49).
        cfg.hyphenlessRoman = settings.isHyphenlessRomanEnabled
        // TPS is a layout, not an engine mode: the engine sees `"tl"` /
        // `"poj"`, so its own `input_mode == "tps"` branch never fires and the
        // swap is folded here, once, at the settings seam.
        cfg.isTranslateSwapped = settings.isTranslateSwapped || settings.inputMode == .tps
        cfg.outputBothScripts = settings.isOutputBothScripts
        return cfg
    }

    /// Encode → FFI roundtrip → decode for the composing slice. Returns the
    /// raw `ComposingResponse` proto so callers that need access to the
    /// `continuous` carrier (FetchAtPos) can reach it without a second
    /// dispatch. Generation is passed through verbatim — composing-slice
    /// generation bumping is owned by `ComposingManager.bumpGeneration()`,
    /// not this layer.
    private static func composingProtoRoundtrip(
        method: Taigi_Engine_ComposingRequest.OneOf_Method,
        op: String,
        generation: UInt64,
        config: Taigi_Engine_AppConfig?,
    ) -> Taigi_Engine_ComposingResponse? {
        let logger = LoggerFactory.make(category: "RustEngineBridge")
        var composing = Taigi_Engine_ComposingRequest()
        composing.method = method

        var request = Taigi_Engine_Request()
        request.id = nextRequestID()
        request.generation = generation
        request.payload = .composing(composing)
        if let config {
            request.configSnapshot = config
        }

        guard let bytes = encodeRequest(request, op: op) else { return nil }
        logger.debug("[FFI->] fn=composingDispatch op=\(op) id=\(request.id) generation=\(generation)")
        guard let response = dispatch(bytes, op: op) else { return nil }
        guard response.error == .ok else {
            recordFailure(op: op, message: "engine returned \(response.error)", code: Int32(response.error.rawValue))
            return nil
        }
        guard case let .composing(payload) = response.payload else {
            recordFailure(op: op, message: "missing composing payload")
            return nil
        }
        return payload
    }

    private static func composingDispatch(
        method: Taigi_Engine_ComposingRequest.OneOf_Method,
        op: String,
        generation: UInt64,
        config: Taigi_Engine_AppConfig?,
    ) -> ComposingTransition {
        guard let payload = composingProtoRoundtrip(
            method: method,
            op: op,
            generation: generation,
            config: config,
        ) else {
            return .noop
        }
        let transition = synthComposing(payload)
        let logger = LoggerFactory.make(category: "RustEngineBridge")
        logger.debug("[FFI<-] fn=composingDispatch op=\(op) effects=\(transition.effects.count) composing=\(transition.isComposing)")
        return transition
    }

    /// Phase 6 FetchAtPos dispatcher. Synthesizes both the standard
    /// `ComposingTransition` (for engine snapshot mirroring) and the
    /// `ContinuousFetchResult.candidates` tri-state read off
    /// `ComposingResponse.continuous`.
    private static func composingFetchDispatch(
        method: Taigi_Engine_ComposingRequest.OneOf_Method,
        op: String,
        generation: UInt64,
        config: Taigi_Engine_AppConfig?,
    ) -> ContinuousFetchResult {
        guard let payload = composingProtoRoundtrip(
            method: method,
            op: op,
            generation: generation,
            config: config,
        ) else {
            return .noop
        }
        let transition = synthComposing(payload)
        let candidates: [ContinuousCandidate]? = payload.hasContinuous
            ? payload.continuous.candidates.map { msg in
                // v3.5.8 Phase 9 Item 5 — `hanji` is proto3 `optional`;
                // SwiftProtobuf exposes presence via `hasHanji`. Map
                // absent → `nil` (NOT empty string) so the bridge
                // struct's `hanji: String?` carries the wire-absent
                // distinction faithfully (TAILO candidate).
                //
                // Defensive `roman` fallback per
                // `docs/engine/continuous-candidate-display.md` §7 +
                // Codex pre-impl F4 verdict A: if `msg.roman` is
                // empty (old-Rust-new-platform wire skew, or proto
                // regen skipped), fall back to `displayText` so the
                // Item 6 dual-line render does not show a blank title
                // row. Bundled releases never hit this branch.
                let roman = msg.roman.isEmpty ? msg.displayText : msg.roman
                return ContinuousCandidate(
                    consumedSpanStart: msg.consumedSpanStart,
                    consumedSpanEnd: msg.consumedSpanEnd,
                    syllableCount: msg.syllableCount,
                    displayText: msg.displayText,
                    score: msg.score,
                    form: msg.form,
                    mode: CandidateMode.decode(msg.mode.rawValue),
                    roman: roman,
                    hanji: msg.hasHanji ? msg.hanji : nil,
                    canonicalTl: msg.canonicalTl,
                )
            }
            : nil
        let logger = LoggerFactory.make(category: "RustEngineBridge")
        let candidateCount = candidates?.count ?? -1
        logger.debug("[FFI<-] fn=composingFetchDispatch op=\(op) effects=\(transition.effects.count) candidates=\(candidateCount)")
        return ContinuousFetchResult(
            transition: transition,
            candidates: candidates,
            isBridgeFailure: false,
        )
    }

    private static func synthComposing(_ proto: Taigi_Engine_ComposingResponse) -> ComposingTransition {
        // Phase 6 effect contract is exhaustive — every emitted Effect.kind
        // maps to a Swift case. The platform delegate
        // (`KeyboardViewController+TextInput`) decides how to dispatch each.
        let effects: [ComposingTransition.Effect] = proto.effect.compactMap { eff -> ComposingTransition.Effect? in
            guard let kind = eff.kind else { return nil }
            switch kind {
            case let .updatePreedit(m): return .updatePreedit(m.display)
            case .clearPreeditWithoutCommit_p: return .clearPreeditWithoutCommit
            case let .commitTextReplacingPreedit(m): return .commitTextReplacingPreedit(m.text)
            case .deleteBackwardFromDocument: return .deleteBackwardFromDocument
            case .resetAutocomplete: return .resetAutocomplete
            case .performAutocomplete: return .performAutocomplete
            case .resetAutocompleteContext: return .resetAutocompleteContext
            case let .nextWordUpdateLastSelectedWord(m):
                return .nextWordUpdateLastSelectedWord(text: m.text, roman: m.roman)
            case let .nextWordWordSelected(m):
                return .nextWordWordSelected(
                    text: m.text,
                    roman: m.roman,
                    triggerPrediction: m.triggerPrediction,
                )
            case .nextWordClearForNewComposing:
                return .nextWordClearForNewComposing
            case let .phraseLearned(m):
                return .phraseLearned(hanji: m.hanji, canonicalTl: m.canonicalTl)
            }
        }
        return ComposingTransition(
            rawInput: proto.preedit.rawInput,
            displayText: proto.preedit.displayText,
            effects: effects,
            isComposing: proto.isComposing,
        )
    }
}
