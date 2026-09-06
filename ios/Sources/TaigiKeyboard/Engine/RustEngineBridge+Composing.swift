import Foundation
import SwiftProtobuf

// MARK: - RustEngineBridge Composing surface

/// Composing slice extension for `RustEngineBridge`. Holds 12 composing
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
        }

        public let rawInput: String
        public let displayText: String
        public let effects: [Effect]
        public let selectedCandidateIndex: Int
        public let isComposing: Bool

        public static let noop = ComposingTransition(
            rawInput: "",
            displayText: "",
            effects: [],
            selectedCandidateIndex: -1,
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
    /// `transition` carries the engine snapshot (preedit / `selectedCandidateIndex`
    /// / `isComposing`); FetchAtPos is read-only so its `effects` is empty.
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

    // MARK: Composing slice (12 ops)

    static func composingStart(
        _ text: String,
        mode: InputMode,
        toggles: ToneToggles,
        generation: UInt64,
    ) -> ComposingTransition {
        var payload = Taigi_Engine_Start()
        payload.text = text
        return composingDispatch(
            method: .start(payload),
            op: "composingStart",
            generation: generation,
            config: appConfig(mode: mode, toggles: toggles),
        )
    }

    static func composingAppend(
        _ char: String,
        mode: InputMode,
        toggles: ToneToggles,
        generation: UInt64,
    ) -> ComposingTransition {
        var payload = Taigi_Engine_Append()
        payload.char = char
        return composingDispatch(
            method: .append(payload),
            op: "composingAppend",
            generation: generation,
            config: appConfig(mode: mode, toggles: toggles),
        )
    }

    static func composingAppendHyphen(
        mode: InputMode,
        toggles: ToneToggles,
        generation: UInt64,
    ) -> ComposingTransition {
        composingDispatch(
            method: .appendHyphen(Taigi_Engine_AppendHyphen()),
            op: "composingAppendHyphen",
            generation: generation,
            config: appConfig(mode: mode, toggles: toggles),
        )
    }

    static func composingReplaceLast(
        _ replacement: String,
        mode: InputMode,
        toggles: ToneToggles,
        generation: UInt64,
    ) -> ComposingTransition {
        var payload = Taigi_Engine_ReplaceLast()
        payload.replacement = replacement
        return composingDispatch(
            method: .replaceLast(payload),
            op: "composingReplaceLast",
            generation: generation,
            config: appConfig(mode: mode, toggles: toggles),
        )
    }

    static func composingDeleteBackward(
        mode: InputMode,
        toggles: ToneToggles,
        generation: UInt64,
    ) -> ComposingTransition {
        composingDispatch(
            method: .deleteBackward(Taigi_Engine_DeleteBackward()),
            op: "composingDeleteBackward",
            generation: generation,
            config: appConfig(mode: mode, toggles: toggles),
        )
    }

    static func composingCommitDerived(
        mode: InputMode,
        toggles: ToneToggles,
        generation: UInt64,
    ) -> ComposingTransition {
        composingDispatch(
            method: .commitDerived(Taigi_Engine_CommitDerived()),
            op: "composingCommitDerived",
            generation: generation,
            config: appConfig(mode: mode, toggles: toggles),
        )
    }

    // v3.5.8 Phase 9 Item 3: `Intent::CommitRaw` under `Phase::Continuous`
    // commits `derived_display(pending, config)` rather than literal
    // keystrokes, so the engine needs the live `AppConfig` (input mode +
    // tone toggles) to render POJ doubletap / nasal-marker / tone marks
    // correctly. Composing-arm behavior is unchanged; the carrier is
    // ignored there.
    // v3.5.8 §10.2 platform pass: under `Phase::Continuous`, `CommitRaw`
    // routes to `commit_raw_continuous` which renders the whole
    // composition via `combined_display(nailed, raw, config)` — so the
    // continuous spacing flags ride here. Composing-arm `CommitRaw`
    // ignores them (base config behavior unchanged).
    // `effectiveSwapped` / `outputBothScripts` default to the v3.5.7
    // roman-first behavior (no swap, no both-scripts) so contract tests
    // and any non-continuous caller stay behavior-identical; EVERY
    // production Continuous call site MUST pass explicit live values via
    // `ComposingManager.continuousSpacingFlags` (the sole production
    // caller does — verified) or hanji-first silently regresses.
    static func composingCommitRaw(
        mode: InputMode,
        toggles: ToneToggles,
        effectiveSwapped: Bool = false,
        outputBothScripts: Bool = false,
        candidateDisplayMode: CandidateDisplayMode = .sideBySide,
        generation: UInt64,
    ) -> ComposingTransition {
        composingDispatch(
            method: .commitRaw(Taigi_Engine_CommitRaw()),
            op: "composingCommitRaw",
            generation: generation,
            config: continuousAppConfig(
                mode: mode,
                toggles: toggles,
                effectiveSwapped: effectiveSwapped,
                outputBothScripts: outputBothScripts,
                candidateDisplayMode: candidateDisplayMode,
            ),
        )
    }

    // v3.5.8 §10.2 platform pass: under `Phase::Continuous`,
    // `SelectSuggestion` routes to `select_suggestion_under_continuous`
    // which prepends `nailed_prefix(nailed, config)` — so the continuous
    // spacing flags must ride here (previously `config: nil` →
    // `AppConfig::default()` → spacing always ON → hanji-first spurious
    // spaces). The composing-arm `select_suggestion` ignores `config`
    // entirely (commits `text` verbatim), so this is a no-op there.
    // Defaults: v3.5.7 roman-first; production Continuous callers MUST
    // pass explicit `continuousSpacingFlags` values (see composingCommitRaw note).
    static func composingSelectSuggestion(
        _ text: String,
        mode: InputMode,
        toggles: ToneToggles,
        effectiveSwapped: Bool = false,
        outputBothScripts: Bool = false,
        candidateDisplayMode: CandidateDisplayMode = .sideBySide,
        generation: UInt64,
    ) -> ComposingTransition {
        var payload = Taigi_Engine_SelectSuggestion()
        payload.text = text
        return composingDispatch(
            method: .selectSuggestion(payload),
            op: "composingSelectSuggestion",
            generation: generation,
            config: continuousAppConfig(
                mode: mode,
                toggles: toggles,
                effectiveSwapped: effectiveSwapped,
                outputBothScripts: outputBothScripts,
                candidateDisplayMode: candidateDisplayMode,
            ),
        )
    }

    // v3.5.8 §10.2 platform pass: under `Phase::Continuous`,
    // `CommitPreeditThenInsertExternal` (e.g. emoji tap mid-continuous)
    // routes to `commit_preedit_then_insert_external_under_continuous`
    // which renders the nailed prefix via
    // `combined_display(nailed, raw, config)` — so the continuous spacing
    // flags ride here too. (Not in the 2026-05-18 enumerated 4 ops, but
    // the same class of Continuous nailed-rendering path: excluding it
    // would re-create the exact hanji-first spurious-space regression the
    // narrowed plumb minimizes — see continuous-input-ranking.md §10.2.)
    // The composing-arm path uses base spacing behavior as before.
    // Defaults: v3.5.7 roman-first; production Continuous callers MUST
    // pass explicit `continuousSpacingFlags` values (see composingCommitRaw note).
    static func composingCommitPreeditThenInsertExternal(
        _ text: String,
        mode: InputMode,
        toggles: ToneToggles,
        effectiveSwapped: Bool = false,
        outputBothScripts: Bool = false,
        candidateDisplayMode: CandidateDisplayMode = .sideBySide,
        generation: UInt64,
    ) -> ComposingTransition {
        var payload = Taigi_Engine_CommitPreeditThenInsertExternal()
        payload.text = text
        return composingDispatch(
            method: .commitPreeditThenInsertExternal(payload),
            op: "composingCommitPreeditThenInsertExternal",
            generation: generation,
            config: continuousAppConfig(
                mode: mode,
                toggles: toggles,
                effectiveSwapped: effectiveSwapped,
                outputBothScripts: outputBothScripts,
                candidateDisplayMode: candidateDisplayMode,
            ),
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

    static func composingSetSelectedCandidateIndex(
        _ index: Int,
        generation: UInt64,
    ) -> ComposingTransition {
        var payload = Taigi_Engine_SetSelectedCandidateIndex()
        payload.index = Int32(index)
        return composingDispatch(
            method: .setSelectedCandidateIndex(payload),
            op: "composingSetSelectedCandidateIndex",
            generation: generation,
            config: nil,
        )
    }

    static func composingQueryState(generation: UInt64) -> ComposingTransition {
        composingDispatch(
            method: .queryState(Taigi_Engine_QueryState()),
            op: "composingQueryState",
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
    static func composingEnterContinuous(
        mode: InputMode,
        toggles: ToneToggles,
        generation: UInt64,
    ) -> ComposingTransition {
        composingDispatch(
            method: .enterContinuous(Taigi_Engine_EnterContinuous()),
            op: "composingEnterContinuous",
            generation: generation,
            config: appConfig(mode: mode, toggles: toggles),
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
    // v3.5.8 §10.2 platform pass: the FetchAtPos snapshot renders the
    // combined marked region (`combined_display`) and per-segment recased
    // candidates, so it needs the continuous spacing flags to match the
    // commit-time rendering.
    // Defaults: v3.5.7 roman-first; production Continuous callers MUST
    // pass explicit `continuousSpacingFlags` values (see composingCommitRaw note).
    static func composingFetchAtPos(
        mode: InputMode,
        toggles: ToneToggles,
        effectiveSwapped: Bool = false,
        outputBothScripts: Bool = false,
        candidateDisplayMode: CandidateDisplayMode = .sideBySide,
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
    ) -> ContinuousFetchResult {
        var payload = Taigi_Engine_FetchAtPos()
        payload.position = 0
        payload.frequencyEntries = frequencyEntries
        payload.nowMs = nowMs
        payload.customEntries = customEntries
        payload.enabledSourcesBitmask = enabledSourcesBitmask
        payload.literalRomanCandidateDisabled = literalRomanCandidateDisabled
        return composingFetchDispatch(
            method: .fetchAtPos(payload),
            op: "composingFetchAtPos",
            generation: generation,
            config: continuousAppConfig(
                mode: mode,
                toggles: toggles,
                effectiveSwapped: effectiveSwapped,
                outputBothScripts: outputBothScripts,
                candidateDisplayMode: candidateDisplayMode,
            ),
        )
    }

    /// Commit a candidate segment in `Phase::Continuous`. `displayText` /
    /// `consumedBytes` / `syllableCount` MUST come from a `ContinuousCandidate`
    /// returned by an immediately preceding `composingFetchAtPos` call —
    /// sending mismatched values mis-aligns the committed segment.
    /// `consumedBytes >= pending.utf8.count` triggers a final commit (exit
    /// to Idle). Programmer-error inputs collapse to noop on the engine side.
    // v3.5.8 §10.2 platform pass: the repro path. Mid-commit renders
    // `combined_display(nailed, pending, config)`; final-commit renders
    // `nailed_prefix(nailed, config)` — both need the spacing flags so
    // segments join with the right (roman: space / hanji-first: none /
    // both-scripts: space) word boundary.
    static func composingCommitContinuous(
        displayText: String,
        canonicalText: String,
        associationTl: String,
        consumedBytes: UInt32,
        syllableCount: UInt32,
        mode: InputMode,
        toggles: ToneToggles,
        // Defaults: v3.5.7 roman-first; production Continuous callers MUST
        // pass explicit `continuousSpacingFlags` values (see composingCommitRaw note).
        effectiveSwapped: Bool = false,
        outputBothScripts: Bool = false,
        candidateDisplayMode: CandidateDisplayMode = .sideBySide,
        generation: UInt64,
    ) -> ComposingTransition {
        var payload = Taigi_Engine_CommitContinuous()
        payload.displayText = displayText
        payload.canonicalText = canonicalText
        // R2: canonical TL of the chosen candidate → NextWord `next_tl` /
        // `prev_tl`. Empty → engine falls back to the raw committed slice.
        payload.associationTl = associationTl
        payload.consumedBytes = consumedBytes
        payload.syllableCount = syllableCount
        return composingDispatch(
            method: .commitContinuous(payload),
            op: "composingCommitContinuous",
            generation: generation,
            config: continuousAppConfig(
                mode: mode,
                toggles: toggles,
                effectiveSwapped: effectiveSwapped,
                outputBothScripts: outputBothScripts,
                candidateDisplayMode: candidateDisplayMode,
            ),
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
    /// Applied ONLY at the Continuous-phase entry points that render the
    /// nailed prefix — `commit_continuous`, `commit_raw_continuous`,
    /// `select_suggestion_under_continuous`,
    /// `commit_preedit_then_insert_external_under_continuous`, and the
    /// FetchAtPos snapshot — so the hanji-first regression surface stays
    /// minimal (continuous-input-ranking.md §10.2; platform pass decided
    /// 2026-05-18). All other composing methods keep the flag-free base
    /// `appConfig`.
    // `candidateDisplayMode` (proto field 9) travels with the pair: under 羅馬字 the callers already
    // pass the DERIVED `(false, false)` pair, and FetchAtPos uses the mode to collapse same-roman rows.
    // CROSS-PLATFORM INVARIANT — mirrors android/app/src/main/java/com/siansiansu/taigikeyboard/engine/RustEngineBridge.kt continuousAppConfig.
    // Drift causes silent divergence (hanji-first spurious word-boundary spaces).
    private static func continuousAppConfig(
        mode: InputMode,
        toggles: ToneToggles,
        effectiveSwapped: Bool,
        outputBothScripts: Bool,
        candidateDisplayMode: CandidateDisplayMode,
    ) -> Taigi_Engine_AppConfig {
        var cfg = appConfig(mode: mode, toggles: toggles)
        cfg.candidateDisplayMode = candidateDisplayMode.engineValue
        cfg.isTranslateSwapped = effectiveSwapped
        cfg.outputBothScripts = outputBothScripts
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
        if let config { request.configSnapshot = config }

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
            }
        }
        return ComposingTransition(
            rawInput: proto.preedit.rawInput,
            displayText: proto.preedit.displayText,
            effects: effects,
            selectedCandidateIndex: Int(proto.selectedCandidateIndex),
            isComposing: proto.isComposing,
        )
    }
}
