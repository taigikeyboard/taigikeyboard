// 中文: iOS 端組字管理器 — Rust 組字引擎與 KeyboardKit / SwiftUI 之間的薄包裝。
// 中文: 引擎狀態(phase / rawInput / selectedCandidateIndex)由 Rust 端持有,這裡只做 mirror + effect dispatch。

import Foundation
import Observation
import SwiftProtobuf

/// Minimal write-only view of the composing-context state that the keyboard
/// extension needs updated when composing starts/stops. KeyboardKit's
/// `KeyboardContext` conforms via `KeyboardContext+Composing` so this
/// wrapper stays Foundation-only.
// 中文: 組字 context 的最小可寫 protocol — 只暴露 isComposingText 一個欄位。
// 中文: KeyboardContext 透過 KeyboardContext+Composing 來符合此 protocol。
protocol ComposingContextSink: AnyObject {
    var isComposingText: Bool { get set }
}

/// iOS platform wrapper around the Rust shared-core composing engine
/// (`engine/composing` crate, accessed through
/// `RustEngineBridge.composing*` methods).
///
/// Engine state (phase + raw input + selectedCandidateIndex) lives inside
/// the Rust singleton `EngineHandle`; this wrapper:
/// - mirrors the latest response into Observation-tracked properties for SwiftUI,
/// - dispatches the bridge-emitted `Effect[]` through `ComposingDelegate`
///   in proto-list order,
/// - notifies the `ComposingContextSink` once state is settled.
///
/// Lifecycle: `currentGeneration` ticks once per real input-context
/// change (per plan §4.2 + Codex P1.4). The bridge passes it on every call;
/// the engine compares against last-seen and silently drops state on
/// mismatch.
// 中文: iOS 端的組字管理器(@Observable)。負責三件事:
// 中文:   1) 把引擎回傳鏡射到 Observation-tracked 屬性給 SwiftUI;
// 中文:   2) 依 proto 順序派送 Effect 給 ComposingDelegate;
// 中文:   3) 結束後通知 ComposingContextSink。
// 中文: currentGeneration 每次 input-context 切換 +1,引擎會丟掉舊 generation 的 stale 請求。
@Observable
public class ComposingManager: ComposingStateProvider, ContinuousCandidateFetcher {
    // MARK: - Observable Mirror

    public private(set) var isComposing: Bool = false
    public private(set) var composingText: String = ""
    public private(set) var rawInput: String = ""
    public private(set) var selectedCandidateIndex: Int = -1

    // MARK: - Lifecycle Generation

    /// Bumped by `KeyboardViewController` lifecycle hooks (commit 11) when
    /// a real input-context change is detected. Engine-side generation
    /// mismatch then drops state silently before applying the next request.
    // 中文: input-context 切換時由 KeyboardViewController 生命週期 +1。
    // 中文: 引擎收到舊 generation 的請求會直接丟棄,避免 stale state 滲入。
    @ObservationIgnored
    private var currentGeneration: UInt64 = 1

    /// `true` while the platform is dispatching effects from a self-driven
    /// commit. Suppresses redundant generation bumps from `textWillChange`
    /// firing on candidate taps / self-commits. Platform suppression flag —
    /// not view state — so excluded from the observation graph.
    // 中文: 自我送出 commit 期間設為 true,壓掉 textWillChange 觸發的多餘 generation bump。
    // 中文: 此為平台抑制旗標(非 view state),刻意排除於 observation graph 外。
    @ObservationIgnored
    public internal(set) var selfCommitInProgress: Bool = false

    // MARK: - Collaborators

    @ObservationIgnored
    private weak var contextSink: ComposingContextSink?

    @ObservationIgnored
    weak var delegate: (any ComposingDelegate)?

    private let settingsProvider: EngineSettingsProvider
    private let userFrequencyService: UserFrequencyService
    // v3.5.8 Phase 9 Item 12 — `custom_dictionary.db` access for the
    // Continuous fetch (the sole custom-dict reader on the keyboard
    // candidate path after Item 13 retired the platform lexicon
    // fallback). `searchSync` is the eager-empty synchronous path
    // (returns `[]` until the DB is open) so the existing synchronous
    // `fetchContinuousCandidates` contract is unchanged. DB stays
    // native (`feedback_user_data_sqlite_stays_native`).
    // 中文: Item 12 — Continuous 路徑查 custom_dictionary.db,與 legacy lexicon path 共用同一 repository;
    // 中文: searchSync 為 eager-empty 同步查詢,DB 未開回 [],不破既有同步 fetch 契約。
    private let customDictionaryRepository: CustomDictionaryRepository
    private let logger = DebugLogger(category: "ComposingManager")

    // MARK: - Init

    init(
        settingsProvider: EngineSettingsProvider = SharedSettings.shared,
        userFrequencyService: UserFrequencyService = CompositionRoot.userFrequencyService,
        customDictionaryRepository: CustomDictionaryRepository = CompositionRoot
            .customDictionaryRepository,
    ) {
        self.settingsProvider = settingsProvider
        self.userFrequencyService = userFrequencyService
        self.customDictionaryRepository = customDictionaryRepository
    }

    func setContextSink(_ sink: ComposingContextSink) {
        contextSink = sink
    }

    /// Called by `KeyboardViewController` lifecycle hooks (per plan §4.2)
    /// when a NEW input context is detected. Subsequent bridge calls carry
    /// the bumped generation; engine drops stale state silently.
    // 中文: 當偵測到新的 input context 時,由 KeyboardViewController 生命週期呼叫,把 generation +1。
    public func bumpGeneration() {
        currentGeneration &+= 1
    }

    // MARK: - Composing Operations

    // 中文: 從外部給定的 text 啟動一段組字。
    public func startComposing(with text: String) {
        logger.debug("[COMPOSE] fn=startComposing text='\(text)'")
        let settings = settingsProvider.current
        apply(RustEngineBridge.composingStart(
            text,
            mode: settings.inputMode,
            toggles: settings.toneToggles,
            generation: currentGeneration,
        ))
        promoteToContinuousIfEligible(settings: settings)
    }

    // 中文: 把單一字元追加到 raw input。
    public func appendCharacter(_ char: String) {
        logger.debug("[COMPOSE] fn=appendCharacter char='\(char)'")
        let settings = settingsProvider.current
        apply(RustEngineBridge.composingAppend(
            char,
            mode: settings.inputMode,
            toggles: settings.toneToggles,
            generation: currentGeneration,
        ))
        promoteToContinuousIfEligible(settings: settings)
    }

    // 中文: 追加連字號 — POJ / TL 的音節分隔符。
    public func appendHyphen() {
        logger.debug("[COMPOSE] fn=appendHyphen")
        let settings = settingsProvider.current
        apply(RustEngineBridge.composingAppendHyphen(
            mode: settings.inputMode,
            toggles: settings.toneToggles,
            generation: currentGeneration,
        ))
        promoteToContinuousIfEligible(settings: settings)
    }

    /// TPS auto-correct — preserves `selectedCandidateIndex`.
    // 中文: TPS 自動更正用 — 用 replacement 取代 raw 最後一個字元;保留 selectedCandidateIndex。
    public func replaceLastCharacter(with replacement: String) {
        logger.debug("[COMPOSE] fn=replaceLastCharacter replacement='\(replacement)'")
        let settings = settingsProvider.current
        apply(RustEngineBridge.composingReplaceLast(
            replacement,
            mode: settings.inputMode,
            toggles: settings.toneToggles,
            generation: currentGeneration,
        ))
        promoteToContinuousIfEligible(settings: settings)
    }

    // MARK: - v3.5.8 Phase 7B — Continuous-input adapters

    /// The v3.5.8 §10.2 word-boundary-spacing flags the engine's
    /// `continuous_word_space` predicate needs, derived from live
    /// settings. Single source of the platform-side `effectiveSwapped`
    /// combine so all Continuous entry points agree (mis-set → silent
    /// hanji-first spurious spaces). `effectiveSwapped` folds TPS into
    /// the swap signal because the engine receives TPS as `"tl"`/`"poj"`
    /// `input_mode` (its own `input_mode == "tps"` branch never fires
    /// from the platform).
    // 中文: §10.2 字界空格旗標的唯一來源 — effectiveSwapped = 翻譯反轉 OR TPS。
    // 中文: TPS 在引擎端是 "tl"/"poj" input_mode,故 TPS 必須在平台端折進 swap 訊號。
    // CROSS-PLATFORM INVARIANT — mirrors android/app/src/main/java/com/siansiansu/taigikeyboard/ime/text/composing/ComposingManager.kt continuousSpacingFlags.
    // Drift causes silent divergence (hanji-first spurious word-boundary spaces).
    private static func continuousSpacingFlags(
        _ settings: EngineSettings,
    ) -> (effectiveSwapped: Bool, outputBothScripts: Bool) {
        (
            effectiveSwapped: settings.isTranslateSwapped || settings.inputMode == .tps,
            outputBothScripts: settings.isOutputBothScripts
        )
    }

    /// Synchronous Continuous-mode promotion fired immediately after each
    /// raw-input mutation (`startComposing` / `appendCharacter` / `appendHyphen`
    /// / `replaceLastCharacter`). Per Codex 2026-05-10 ANALYSIS-ONLY consult
    /// (Fork A modify): MUST run on the same thread frame as the triggering
    /// intent so the `EnterContinuous` request shares the caller's
    /// `currentGeneration` snapshot — the engine resets to Idle on any
    /// generation mismatch (`engine/composing/src/handle.rs:61-65`), so a
    /// delayed/async call could silently wipe newer composing state.
    /// Engine no-ops the request when `Phase::Composing { raw }` is empty or
    /// when already in `Phase::Continuous` (`engine/composing/src/transition.rs:496-502`),
    /// so unconditional issuance is safe and avoids platform-side eligibility
    /// heuristics.
    // 中文: 同步 Continuous 推進。同一執行緒 frame 內 fire,共享 caller generation
    // 中文: 快照,避免 stale 引擎重置抹掉新狀態。引擎在空 raw / 已是 Continuous 時 no-op,
    // 中文: 所以可無條件呼叫,不需要平台端啟發式判斷。
    private func promoteToContinuousIfEligible(settings: EngineSettings) {
        let transition = RustEngineBridge.composingEnterContinuous(
            mode: settings.inputMode,
            toggles: settings.toneToggles,
            generation: currentGeneration,
        )
        // EnterContinuous emits zero effects (transition.rs:514). The mirror
        // refresh keeps `composingText` in sync with the engine's preedit
        // even though no platform side-effects fire.
        apply(transition)
    }

    /// Synchronous span-local candidate query. Read-only; engine returns the
    /// current `Phase::Continuous { raw }` candidate set in score-desc order.
    /// Returns `[]` when not in Continuous phase, when no syllable inventory
    /// is installed, or when the FST returns no hits — the caller cannot
    /// distinguish these cases (Codex Risk 4: graceful degrade is OK because
    /// the lexicon path then handles the same input via its own search).
    ///
    /// v3.5.8 Phase 9.3b — two-phase fetch closes Gap B (`docs/engine/
    /// continuous-input-ranking.md` §3.2) by feeding the engine's
    /// `user_freq_boost` + `SortKey.recency_rank` axes:
    /// 1. Neutral fetch (empty `frequencyEntries`, `nowMs = 0`) discovers
    ///    candidate `displayText` keys — iOS cannot know them up-front.
    /// 2. Batch query `user_frequency.db WHERE word IN (...)` for those keys.
    /// 3. Populated fetch on the same `currentGeneration` snapshot re-ranks
    ///    the candidate set with `user_freq_boost(count)` saturated at
    ///    `MAX_BOOST = 5.0` per `engine/ranking/src/score.rs`.
    ///
    /// `currentGeneration` is captured once so a `bumpGeneration()` between
    /// the two FFI calls cannot corrupt the populated fetch — engine resets
    /// to Idle on generation mismatch (`engine/composing/src/handle.rs:61-66`)
    /// and we surface that as the documented "no candidates this frame"
    /// degrade rather than an inconsistent boost. The phase-2 `transition`
    /// already reflects the Idle reset; returning the phase-1 list would
    /// render stale candidates against the new context, so we return `[]`
    /// instead. Codex pre/post-impl Q5/R2.
    ///
    /// Bridge-failure handling distinguishes "engine returned Idle" (legit
    /// reset; apply Idle transition + return `[]`) from "FFI roundtrip
    /// failed" (transient encode/decode/non-OK; engine state unchanged —
    /// apply phase-1 transition + return phase-1 candidates). Without the
    /// `isBridgeFailure` flag both scenarios collapse to a `.noop`
    /// transition + `nil` candidates, and applying `.noop` clobbers the
    /// mirror with false Idle state. Phase-1 FFI failure short-circuits
    /// the whole frame; phase-2 FFI failure degrades to neutral-ranked
    /// phase-1 results. Codex PR #265 r3216857164.
    ///
    /// Cold-start: when `user_frequency.db` has not yet been opened (covers
    /// the brief window between `setupCoreServices` firing its best-effort
    /// `ensureInitialized` Task and that Task completing — the very first
    /// composition may legitimately fall here), skip phase 2 and return
    /// the neutral list. Matches
    /// `CustomDictionaryRepository.searchSync`'s eager-empty pattern.
    /// Codex pre-impl Q6.
    ///
    /// `isConnected()` only proves the SQLite connection is open — schema
    /// creation may still be in flight inside `ensureInitialized`. If a
    /// fetch slips through that race, `frequencyDataBatch` returns `[:]`
    /// when the `SELECT` fails to prepare against an absent table; the
    /// engine then sees empty `frequencyEntries` and applies neutral boost
    /// everywhere — same observable outcome as the cold-start branch but
    /// via one extra phase-2 fetch. Documented degrade, not a bug.
    /// Codex PR #265 r3216760651 (P6).
    // 中文: 同步擷取 Continuous 候選詞。只讀,non-Continuous / 無 inventory / 無命中
    // 中文: 都回傳空 []。呼叫端無需區分,fall-through 到既有 lexicon path 即可。
    // 中文: Phase 9.3b two-phase fetch — 中性查 → SQLite 查 freq → 帶 freq 重查 + 重排。
    // 中文: generation 一次取樣,中途 bump 會讓 phase 2 回空,等同無 candidate 這 frame。
    public func fetchContinuousCandidates() -> [RustEngineBridge.ContinuousCandidate] {
        let settings = settingsProvider.current
        let generation = currentGeneration

        // v3.5.8 Phase 9 Item 12 — query `custom_dictionary.db` once
        // for the current raw buffer and pass the same `customEntries`
        // to both fetch phases (the result depends only on `rawInput`,
        // which is stable for this synchronous fetch). The engine
        // synthesizes a full-buffer candidate per entry and dedupes
        // `(roman, hanji)` against the FST hits.
        // 中文: Item 12 — 用當前 rawInput 查 custom_dictionary.db 一次,兩個 phase 共用同一 customEntries。
        let customEntries = buildCustomEntries(rawInput: rawInput, settings: settings)
        let spacing = Self.continuousSpacingFlags(settings)

        // PR-9.6 — compute the dictionary source-toggle bitmask from the
        // SAME settings snapshot + SAME `compute_filters` bridge the Tab3
        // browse path uses (`DictionarySearchService.search`), so keyboard
        // candidates honour the same 12 source toggles + kautian
        // subcollection (腔調/姓名) toggles. Computed once and shared by
        // both fetch phases (the result depends only on `settings`, stable
        // for this synchronous fetch — mirrors `customEntries`).
        let enabledSourcesBitmask = RustEngineBridge.lexiconDictionaryFilters(
            toggles: RustEngineBridge.DictionaryToggles(from: settings),
        ).dictionaryFilterBitmask

        // Phase 1: neutral fetch to learn candidate displayText keys.
        let neutral = RustEngineBridge.composingFetchAtPos(
            mode: settings.inputMode,
            toggles: settings.toneToggles,
            effectiveSwapped: spacing.effectiveSwapped,
            outputBothScripts: spacing.outputBothScripts,
            generation: generation,
            customEntries: customEntries,
            enabledSourcesBitmask: enabledSourcesBitmask,
        )
        // Phase-1 FFI failure: do NOT apply the synthesized `.noop` — that
        // would clobber the mirror with false Idle state. Surface as "no
        // candidates this frame"; the mirror keeps reflecting the most
        // recent successful transition (typically the keystroke's
        // append/promote that brought us into Continuous), so the next
        // keystroke's fetch finds the right engine state. Codex PR #265
        // r3216857164 pre-impl S5 + post-impl T2.
        if neutral.isBridgeFailure {
            return []
        }
        guard let neutralCandidates = neutral.candidates, !neutralCandidates.isEmpty else {
            apply(neutral.transition)
            return neutral.candidates ?? []
        }

        // Cold-start: user_frequency.db not yet open. Skip phase 2 — engine
        // already produced neutral-boost ranking on the phase-1 response.
        guard userFrequencyService.isConnected() else {
            apply(neutral.transition)
            return neutralCandidates
        }

        // Phase 2: populated fetch with the user-frequency snapshot.
        let entries = Self.buildFrequencyEntries(
            for: neutralCandidates,
            via: userFrequencyService,
        )
        let nowMs = Int64(Date().timeIntervalSince1970 * 1000)
        let boosted = RustEngineBridge.composingFetchAtPos(
            mode: settings.inputMode,
            toggles: settings.toneToggles,
            effectiveSwapped: spacing.effectiveSwapped,
            outputBothScripts: spacing.outputBothScripts,
            generation: generation,
            frequencyEntries: entries,
            nowMs: nowMs,
            customEntries: customEntries,
            enabledSourcesBitmask: enabledSourcesBitmask,
        )
        // Phase-2 FFI failure: engine state did NOT change since phase-1
        // (the request never reached the engine). Apply phase-1's transition
        // (the real engine snapshot from the moment phase-1 succeeded) and
        // return phase-1 candidates — degrade to neutral-ranked instead of
        // dropping the frame. Codex PR #265 r3216857164.
        if boosted.isBridgeFailure {
            apply(neutral.transition)
            return neutralCandidates
        }
        apply(boosted.transition)
        // Engine determinism: same `Phase::Continuous { raw }` returns the
        // same candidate set. A `nil` phase-2 carrier with `isBridgeFailure
        // == false` means a `bumpGeneration` raced in between and engine
        // reset to Idle BEFORE this fetch — the applied transition already
        // mirrors that Idle state, so returning phase-1 candidates would
        // render stale suggestions against the new context. Surface as
        // "no candidates this frame" instead. Codex pre/post-impl Q5/R2.
        return boosted.candidates ?? []
    }

    /// Marshal the per-candidate `user_frequency.db` snapshot into the proto
    /// `FrequencyEntry[]` shape required by `FetchAtPos`. Dedupes by
    /// `displayText` (engine's `display_text_key` = `hanji ?? roman`) so a
    /// candidate list with the same hanji twice (different roman) issues
    /// only one SQL placeholder; the engine's `build_frequency_map` is
    /// last-write-wins on duplicates either way (`engine/ranking/src/
    /// score.rs::build_frequency_map`). Only entries present in the DB are
    /// marshalled — missing rows mean "no user usage yet" and the engine
    /// applies `user_freq_boost(0) = 1.0` neutral. Codex pre-impl Q4 / Q8.
    // 中文: 把候選詞的 user_frequency.db 快照打包成 proto FrequencyEntry。
    // 中文: 以 displayText 去重壓 SQL placeholder;DB 沒有的 row 不送 → 引擎自動 neutral。
    private static func buildFrequencyEntries(
        for candidates: [RustEngineBridge.ContinuousCandidate],
        via service: UserFrequencyService,
    ) -> [Taigi_Engine_FrequencyEntry] {
        var seen = Set<String>()
        var uniqueKeys: [String] = []
        uniqueKeys.reserveCapacity(candidates.count)
        for candidate in candidates where seen.insert(candidate.displayText).inserted {
            uniqueKeys.append(candidate.displayText)
        }
        let snapshot = service.frequencyDataBatch(for: uniqueKeys)
        return snapshot.map { word, data in
            var entry = Taigi_Engine_FrequencyEntry()
            entry.displayTextKey = word
            entry.count = UInt32(max(0, data.count))
            entry.lastUsedMs = data.lastUsedMillis
            return entry
        }
    }

    /// v3.5.8 Phase 9 Item 12 — query `custom_dictionary.db` for the
    /// current raw buffer and marshal matches into the proto
    /// `CustomDictEntry[]` carried by `FetchAtPos`. The engine owns the
    /// merge + `(roman, hanji)` dedupe + ranking (spec G3 — platform
    /// never re-ranks); this method only fetches + marshals.
    ///
    /// Query path: `CustomDictionaryDerivation.searchPrefix` →
    /// `CustomDictionaryRepository.searchSync` (parameterized SQL).
    /// **Marshals the RAW stored `(roman, hanzi)` columns** — NOT a
    /// capitalization-massaged form — so the engine's
    /// `(roman, hanji)` dedupe key collides correctly against
    /// `dict.bin`'s `DictionaryRecord.tl` / `.hanzi` (Codex pre-impl
    /// 2026-05-15); capitalization is the engine's concern. The
    /// stored roman may be either TL or POJ display form (whichever
    /// the user typed) — v3.5.9 B-4 keeps it raw on the lattice axis
    /// and only folds it to canonical TL inside the engine when
    /// synthesizing the `user_frequency.db` commit key, so this
    /// marshaler stays form-agnostic. An empty stored hanzi maps to
    /// proto-absent `hanji` (romanization-only entry → engine derives
    /// `CandidateMode::Tailo`), mirroring `record_to_candidate`.
    ///
    /// `searchSync` is the eager-empty synchronous path (returns `[]`
    /// until the DB connection is open), so the synchronous
    /// `fetchContinuousCandidates` contract is preserved with no extra
    /// await — same cold-start tolerance as
    /// `userFrequencyService.isConnected()`.
    // 中文: Item 12 — 查 custom_dictionary.db 並 marshal 成 proto CustomDictEntry[];
    // 中文: 與 legacy lexicon path 共用同一 derivation+query,但送「原始儲存的 (roman,hanzi)」,
    // 中文: 不送大寫化後的形式,讓引擎 (roman,hanji) 去重鍵能與 dict.bin 正確碰撞。
    // 中文: 空 hanzi → proto-absent hanji (純羅馬字 → 引擎判 TAILO);searchSync eager-empty 保同步契約。
    private func buildCustomEntries(
        rawInput: String,
        settings: EngineSettings,
    ) -> [Taigi_Engine_CustomDictEntry] {
        guard settings.isCustomDictEnabled, !rawInput.isEmpty else { return [] }
        let (searchPrefix, isToneAware) = CustomDictionaryDerivation.searchPrefix(for: rawInput)
        let rows = customDictionaryRepository.searchSync(
            prefix: searchPrefix,
            isToneAware: isToneAware,
            limit: 20,
        )
        return rows.map { row in
            var entry = Taigi_Engine_CustomDictEntry()
            entry.roman = row.roman
            if !row.hanzi.isEmpty {
                entry.hanji = row.hanzi
            }
            return entry
        }
    }

    /// Commit one Continuous candidate. `displayText` / `consumedBytes` /
    /// `syllableCount` MUST come verbatim from a `ContinuousCandidate`
    /// returned by an immediately preceding `fetchContinuousCandidates()`
    /// call — the engine collapses to noop on UTF-8/syllable boundary
    /// violations, so caller-side validation is unnecessary
    /// (`engine/composing/src/transition.rs:613-679`).
    ///
    /// Returns an effect-backed signal so callers can gate side effects
    /// (frequency recording, auto-space) on actual commit success rather
    /// than coarse `isComposing` mirror state. Codex PR #257 r3214932308:
    /// generation mismatch can silently reset the engine to Idle in
    /// `engine/composing/src/handle.rs:61-65` BEFORE the intent runs, in
    /// which case `Intent::CommitContinuous` becomes a phase-mismatch noop
    /// — the post-call mirror flips to `isComposing=false` (engine is
    /// Idle) but no `CommitTextReplacingPreedit` effect is emitted.
    /// Without an effect-backed gate, callers would record frequency for
    /// uncommitted text and append a stray space.
    ///
    /// - Returns (**Model B**, §10):
    ///   - `didCommit`: a continuous selection succeeded — a **nail**
    ///     (mid-commit) OR a final commit. Under Model B a mid-commit nail
    ///     does NOT write the document (emits no `.commitTextReplacingPreedit`)
    ///     so it is detected by `.nextWordUpdateLastSelectedWord` (the
    ///     per-segment learning signal emitted exactly on a successful nail
    ///     in this path); a final commit is detected by
    ///     `.commitTextReplacingPreedit`. A noop emits neither.
    ///   - `didFinalCommit`: `.commitTextReplacingPreedit` present AND
    ///     `transition` exited Continuous — only the hard finalize writes
    ///     literal text. Implies `didCommit`.
    ///
    /// Model B mid-commit (nail) emits `[UpdatePreedit(whole composition),
    /// NextWordUpdateLastSelectedWord, PerformAutocomplete]` and stays
    /// Continuous; final-commit (`consumedBytes >= pending.utf8.count`)
    /// emits `[CommitTextReplacingPreedit(whole composition),
    /// ResetAutocomplete, ResetAutocompleteContext, NextWordWordSelected]`
    /// and exits Continuous.
    // 中文: 送出一個 Continuous 候選詞段;回傳 effect-backed (didCommit, didFinalCommit) 旗標,
    // 中文: 讓 caller 用真實 commit signal 過濾 frequency / auto-space side-effects,
    // 中文: 而不是 isComposing mirror — 後者在 generation 不對齊 silent reset 時會誤報。
    // v3.5.8 Phase 9 Bug 1 (Option A): `displayText` is the swap/TPS/both-
    // scripts-formatted DOCUMENT string (caller mirrors the legacy lexicon
    // formatter); `canonicalText` is the canonical key (`hanji ?? roman`)
    // routed to NextWord so association learning stays mode-independent.
    public func commitContinuous(
        displayText: String,
        canonicalText: String,
        associationTl: String,
        consumedBytes: UInt32,
        syllableCount: UInt32,
    ) -> (didCommit: Bool, didFinalCommit: Bool) {
        logger.debug(
            "[COMPOSE] fn=commitContinuous displayLen=\(displayText.count) "
                + "canonicalLen=\(canonicalText.count) "
                + "consumedBytes=\(consumedBytes) syllCount=\(syllableCount)",
        )
        let settings = settingsProvider.current
        let spacing = Self.continuousSpacingFlags(settings)
        let transition = RustEngineBridge.composingCommitContinuous(
            displayText: displayText,
            canonicalText: canonicalText,
            associationTl: associationTl,
            consumedBytes: consumedBytes,
            syllableCount: syllableCount,
            mode: settings.inputMode,
            toggles: settings.toneToggles,
            effectiveSwapped: spacing.effectiveSwapped,
            outputBothScripts: spacing.outputBothScripts,
            generation: currentGeneration,
        )
        // Inspect transition BEFORE dispatching effects so we can return an
        // effect-backed signal. `applyAsSelfCommit` body inlined (3 lines)
        // for the same reason — semantics identical to the helper.
        let hasCommitText = transition.effects.contains { effect in
            if case .commitTextReplacingPreedit = effect { return true }
            return false
        }
        let hasNail = transition.effects.contains { effect in
            if case .nextWordUpdateLastSelectedWord = effect { return true }
            return false
        }
        // Model B: nail (mid-commit) emits no commit-text; the learning
        // effect is its success signal. Final-commit emits commit-text +
        // exits. noop emits neither → both flags false (closes the
        // generation-mismatch race, unchanged guarantee).
        let didCommit = hasCommitText || hasNail
        let didFinalCommit = hasCommitText && !transition.isComposing
        selfCommitInProgress = true
        defer { selfCommitInProgress = false }
        apply(transition)
        return (didCommit: didCommit, didFinalCommit: didFinalCommit)
    }

    /// Abort Continuous-input. Drops `Phase::Continuous`'s pending + nailed
    /// list, exits to Idle, emits the standard abort effect trio
    /// (`ClearPreeditWithoutCommit` + `ResetAutocomplete` +
    /// `NextWordClearForNewComposing`). **Model B** (§10.6): nailed segments
    /// were never literal document text — `ClearPreeditWithoutCommit` clears
    /// the WHOLE marked composition; abort discards it entirely (no document
    /// write, no `DeleteBackwardFromDocument`).
    /// Used by `KeyboardViewController+Setup.syncSettings` on input-mode swap
    /// (TL ↔ POJ ↔ TPS) so stale Continuous state can't leak across modes.
    // 中文: 中止 Continuous;committed segments 不回退(已在 document)。Settings inputMode
    // 中文: 切換時呼叫,確保跨模式無殘留狀態。
    public func resetContinuous() {
        logger.debug("[COMPOSE] fn=resetContinuous")
        applyAsSelfCommit(RustEngineBridge.composingResetContinuous(generation: currentGeneration))
    }

    // 中文: 退格 — 刪掉 raw input 最後一個字元。
    public func deleteBackward() {
        logger.debug("[COMPOSE] fn=deleteBackward")
        let settings = settingsProvider.current
        apply(RustEngineBridge.composingDeleteBackward(
            mode: settings.inputMode,
            toggles: settings.toneToggles,
            generation: currentGeneration,
        ))
    }

    // 中文: 把目前 derived 顯示文字送出(commit derived) — 結束組字。
    public func commitComposition() {
        logger.debug("[COMPOSE] fn=commitComposition")
        // Model B (§10.3 + v3.5.8 Phase 9 Finding 2): finalize the WHOLE
        // current composition. Route through `CommitRaw` — under Continuous
        // the engine commits `Σ nailed.display_text + derived(pending)` (the
        // whole composition) and fires the terminal NextWord; it builds the
        // string from engine state, so there is no prefix duplication. The
        // old `SelectSuggestion(composingText)` reroute double-counted the
        // nailed prefix once the composing buffer became the whole
        // composition (`select_suggestion_under_continuous` prepends
        // `nailed_prefix`). `selectSuggestion(candidate)` still uses
        // SelectSuggestion (bare candidate → engine prepends the nailed
        // prefix correctly). Empty preedit → CommitDerived (a no-op on
        // Idle). Bare `Phase::Composing` reaching here would violate the
        // Phase 7B invariant (every active composition is auto-promoted to
        // Continuous first); we deliberately do NOT branch on phase — the
        // binding has no safe phase signal (Codex pre-impl point 3).
        // 中文: Model B — commitComposition 走 CommitRaw 提交整段組字,無前綴重複;
        // 中文: 空 preedit 走 CommitDerived(Idle noop);不在 binding branch phase。
        let settings = settingsProvider.current
        guard !composingText.isEmpty else {
            applyAsSelfCommit(RustEngineBridge.composingCommitDerived(
                mode: settings.inputMode,
                toggles: settings.toneToggles,
                generation: currentGeneration,
            ))
            return
        }
        let spacing = Self.continuousSpacingFlags(settings)
        applyAsSelfCommit(RustEngineBridge.composingCommitRaw(
            mode: settings.inputMode,
            toggles: settings.toneToggles,
            effectiveSwapped: spacing.effectiveSwapped,
            outputBothScripts: spacing.outputBothScripts,
            generation: currentGeneration,
        ))
    }

    // 中文: 把 raw input 直接送出。Composing 階段送字面 keystrokes;Continuous 階段送
    // 中文: 整段組字 (Σ nailed.display_text + derived(pending)),由引擎依 phase 自動分派。
    public func commitRawInput() {
        logger.debug("[COMPOSE] fn=commitRawInput")
        // v3.5.8 Phase 9 Item 3 + Model B (§10.3):
        // `Intent::CommitRaw` handles `Phase::Continuous` natively in the
        // engine — under Model B it commits the WHOLE composition
        // (`Σ nailed.display_text + derived(pending)`) and fires the same
        // terminal NextWord effect as a final-commit candidate tap (see
        // `engine/composing/tests/continuous_phase.rs::commit_raw_under_continuous_*`).
        // The Phase 7B SelectSuggestion bypass is no longer needed; the
        // engine owns the per-phase routing.
        // 中文: Phase 9 Item 3 + Model B — engine 在 Continuous 下提交整段組字 + 終端 NextWord;
        // 中文: 平台不再 SelectSuggestion 繞路,直接送 CommitRaw 由引擎決定行為。
        let settings = settingsProvider.current
        let spacing = Self.continuousSpacingFlags(settings)
        applyAsSelfCommit(RustEngineBridge.composingCommitRaw(
            mode: settings.inputMode,
            toggles: settings.toneToggles,
            effectiveSwapped: spacing.effectiveSwapped,
            outputBothScripts: spacing.outputBothScripts,
            generation: currentGeneration,
        ))
    }

    // 中文: 使用者點選候選詞時呼叫,送出 text 並結束組字。
    public func selectSuggestion(text: String) {
        logger.debug("[COMPOSE] fn=selectSuggestion len=\(text.count)")
        // §10.2 platform pass: under Continuous this routes to
        // `select_suggestion_under_continuous` (prepends `nailed_prefix`),
        // so pass the live spacing flags instead of the old nil config.
        let settings = settingsProvider.current
        let spacing = Self.continuousSpacingFlags(settings)
        applyAsSelfCommit(RustEngineBridge.composingSelectSuggestion(
            text,
            mode: settings.inputMode,
            toggles: settings.toneToggles,
            effectiveSwapped: spacing.effectiveSwapped,
            outputBothScripts: spacing.outputBothScripts,
            generation: currentGeneration,
        ))
    }

    // 中文: 先把 preedit 送出再插入外部 text — 例如剪貼或 NextWord 觸發時用。
    public func commitPreeditThenInsertExternal(_ text: String) {
        logger.debug("[COMPOSE] fn=commitPreeditThenInsertExternal len=\(text.count)")
        let settings = settingsProvider.current
        let spacing = Self.continuousSpacingFlags(settings)
        applyAsSelfCommit(RustEngineBridge.composingCommitPreeditThenInsertExternal(
            text,
            mode: settings.inputMode,
            toggles: settings.toneToggles,
            effectiveSwapped: spacing.effectiveSwapped,
            outputBothScripts: spacing.outputBothScripts,
            generation: currentGeneration,
        ))
    }

    /// Commit the currently-selected candidate, given the visible candidate
    /// strings. KK-side callers pass `suggestions.map(\.text)`.
    // 中文: 把目前選中的候選詞送出。呼叫端傳入目前可見候選文字列表(KK 是 suggestions.map(\.text))。
    public func confirmSelectedCandidate(availableTexts: [String]) -> Bool {
        logger.debug("[COMPOSE] fn=confirmSelectedCandidate index=\(selectedCandidateIndex) count=\(availableTexts.count)")
        guard isComposing,
              selectedCandidateIndex >= 0,
              selectedCandidateIndex < availableTexts.count
        else { return false }
        selectSuggestion(text: availableTexts[selectedCandidateIndex])
        return true
    }

    // 中文: 重置組字狀態 — 不送出,僅清空。
    public func reset() {
        logger.debug("[COMPOSE] fn=reset")
        applyAsSelfCommit(RustEngineBridge.composingReset(generation: currentGeneration))
    }

    // 中文: 更新目前選中的候選詞 index,給鍵盤方向鍵 / 候選列點擊使用。
    public func setSelectedCandidateIndex(_ index: Int) {
        logger.debug("[COMPOSE] fn=setSelectedCandidateIndex index=\(index)")
        apply(RustEngineBridge.composingSetSelectedCandidateIndex(index, generation: currentGeneration))
    }

    // MARK: - Apply Transition (three-phase, see boundary doc §2.4)

    // 中文: apply 的自我送出版本 — 設好 selfCommitInProgress 旗標壓掉多餘 generation bump。
    private func applyAsSelfCommit(_ transition: RustEngineBridge.ComposingTransition) {
        selfCommitInProgress = true
        defer { selfCommitInProgress = false }
        apply(transition)
    }

    // 中文: 套用一次 ComposingTransition,三階段:鏡射 → 派送 effect → 通知 sink。
    private func apply(_ transition: RustEngineBridge.ComposingTransition) {
        // Phase 1 — mutate observable mirror (guarded-inequality writes keep
        // idle→idle silent and avoid redundant SwiftUI invalidation).
        if isComposing != transition.isComposing { isComposing = transition.isComposing }
        if rawInput != transition.rawInput { rawInput = transition.rawInput }
        if !transition.effects.isEmpty, composingText != transition.displayText {
            composingText = transition.displayText
        }
        if selectedCandidateIndex != transition.selectedCandidateIndex {
            selectedCandidateIndex = transition.selectedCandidateIndex
        }

        // Phase 2 — execute platform effects in proto-list order.
        for effect in transition.effects {
            delegate?.execute(effect)
        }

        // Phase 3 — notify composing-context sink once state is settled.
        contextSink?.isComposingText = transition.isComposing
    }
}
