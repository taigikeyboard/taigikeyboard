// RustEngineBridge 的 Composing 切片擴充 — v3.5.4 起,加上 v3.5.8 Phase 6 連續輸入 4 op。
// 含 12 composing op + 4 continuous op + 4 synthesized 型別 + composing 專屬 dispatch helper。

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
// Composing bridge 擴充入口。12+4 op + 4 個 synthesized 型別 + 5 個私有 composing 專屬 helper。
public extension RustEngineBridge {
    // MARK: - Synthesized value types

    /// Bridge-synthesized companion to the proto `ComposingResponse`.
    /// Consumed by `ComposingManager` and its delegate.
    // 對應 proto ComposingResponse 的 Swift 端 struct,由 bridge 解碼後組成。
    // ComposingManager 與其 delegate 用這個型別決定要對輸入框做什麼動作。
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
    // Phase 9.2 候選類型軸;Rust 端 derive_mode 推導,平台僅讀不算(禁 display_text sniff)。
    //   metadata-only,不入 SortKey。`.unspecified` = wire 上 mode 缺漏 → 視為「無 mode 資訊」。
    enum CandidateMode: Equatable {
        case unspecified
        case hant
        case tailo
        case mixed

        /// Decode the wire integer (`CandidateMessage.mode.rawValue`)
        /// produced by SwiftProtobuf. Unrecognized values (forward-compat
        /// from a newer engine) collapse to `.unspecified` so the
        /// platform never crashes on a binding mismatch.
        // 由 proto wire 整數解碼;未知值 fall back 到 .unspecified,避免 binding mismatch crash。
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
    // 連續輸入候選詞,對應 proto CandidateMessage。consumed span 是 raw
    // 緩衝區的 byte offset(TL/POJ = ASCII;TPS = Bopomofo)。form 目前固定 1;mode 為 Phase 9.2 metadata-only。
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
        // Item 5 — 顯示羅馬字 sidechannel(引擎依 input mode 渲染:TL 或 POJ),dual-line 候選列 render 用。
        public let roman: String
        /// v3.5.8 Phase 9 Item 5 — hanji display sidechannel. `nil`
        /// iff the proto3 `optional string hanji` was absent on the
        /// wire (TAILO candidate). Present-empty is treated as
        /// present (engine never emits `Some("")` today; defensive).
        // Item 5 — 漢字 sidechannel;TAILO 候選 wire 上 absent → nil。
        public let hanji: String?
        /// v3.6.1 R2 — canonical TL identity sidechannel
        /// (`CandidateMessage.canonical_tl`). Unlike `roman` (the
        /// POJ-rendered display form in POJ mode), this stays the
        /// canonical TL the `(hanji, canonical-TL)` word identity is keyed
        /// on. The tap path round-trips it into
        /// `commitContinuous(associationTl:)` so the NextWord association
        /// learns the same TL a normal candidate commit records. Empty
        /// only for TPS-OOV hanji-absent candidates with no dict TL.
        // R2 — canonical TL 身分 sidechannel;tap 時 round-trip 回 associationTl。
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
    // composingFetchAtPos 的查詢結果。candidates 三態只在 isBridgeFailure == false 時有意義。
    //   nil = 不在 Continuous phase;[] = 在但無候選;non-empty = 有候選。
    // transition 帶 engine 狀態(FetchAtPos 只讀,effects 必為空)。
    // isBridgeFailure 區分「引擎回 Idle」與「FFI 失敗」— 後者套用 transition 會清掉鏡射狀態。
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
    // Phase 9 Item 3 — Continuous 下 CommitRaw 走 derived_display 需 AppConfig;
    // Composing 路徑不受影響 (config 在 Composing 分支被忽略)。
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
    // 把 Composing 轉到 Continuous。Phase 6 規約 — 無 payload,raw 來自先前的
    // Start / Append。空 raw / 非 Composing 一律 noop。
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
    // 連續輸入候選查詢。position 固定為 0(Phase 6 dispatch 驗證)。
    // generation 必須沿用當前 composing session — 不可 bump,否則會在 fetch 前重置狀態。
    // frequencyEntries + nowMs 為 Phase 9.3a/9.3b 的 user_freq_boost / recency_rank 來源,
    // 預設空陣列 + 0 維持中性 boost,實際填充由 ComposingManager two-phase fetch 負責。
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
    // Item 12 — customEntries 帶平台 custom_dictionary.db 原始 (roman,hanji);預設空 = no-op,
    // 引擎合成 full-buffer 候選並對 (roman,hanji) 去重 (custom 必勝碰撞)。
    // B-4 — roman 為用戶 native 形(TL 或 POJ),引擎在合成 freq commit key 時折成 canonical TL,
    //   跨 mode freq 學習合一。
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
    // 連續輸入提交候選段。displayText / consumedBytes / syllableCount 必須與
    // 上一個 composingFetchAtPos 回傳的 ContinuousCandidate 對齊。
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
    // 連續輸入中止。pending 與 committed 一起丟,Phase 退回 Idle,發 abort 三 effects。
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
    // 連續輸入渲染用 AppConfig — base appConfig + §10.2 字界空格兩旗標。
    // effectiveSwapped(翻譯反轉 OR TPS,平台端合併,因雙平台 TPS→"tl" 故引擎 input_mode=="tps" 永不觸發)
    // 走 is_translate_swapped;outputBothScripts 區分漢字優先(無空格)vs 雙腳本(要空格)。
    // 只用在會渲染 nailed prefix 的 Continuous 進入點,縮小 hanji-first 退化面。
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
    // composing slice 的 FFI roundtrip,回傳原始 proto 供需要 continuous 載體的 caller(FetchAtPos)使用。
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

        let bytes: [UInt8]
        do {
            bytes = try Array(request.serializedData())
        } catch {
            recordFailure(op: op, message: "encode failed: \(error)")
            return nil
        }

        logger.debug("[FFI->] fn=composingDispatch op=\(op) id=\(request.id) generation=\(generation)")
        let responseBytes = bytes.withUnsafeBufferPointer { buf in
            process_request_bytes(buf).toArray()
        }
        guard let response = try? Taigi_Engine_Response(
            serializedBytes: Data(responseBytes),
        ) else {
            recordFailure(op: op, message: "response decode failed")
            return nil
        }
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
    // Phase 6 FetchAtPos 專用分派 — 同時產生 ComposingTransition 與
    // ContinuousFetchResult.candidates(從 proto.continuous 三態解碼)。
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
                // Item 5 — hanji 為 proto3 optional;wire absent → Swift nil。
                // roman 防禦性 fallback — wire skew 時 displayText 兜底,避免空 title。
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
