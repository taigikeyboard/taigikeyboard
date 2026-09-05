// iOS composing manager — thin wrapper between the Rust composing engine and KeyboardKit / SwiftUI.
// Engine state (phase / rawInput / selectedCandidateIndex) lives in Rust; this only mirrors it and dispatches effects.

import Foundation
import Observation
import SwiftProtobuf

/// Minimal write-only view of the composing-context state that the keyboard
/// extension needs updated when composing starts/stops. KeyboardKit's
/// `KeyboardContext` conforms via `KeyboardContext+Composing` so this
/// wrapper stays Foundation-only.
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
    @ObservationIgnored
    private var currentGeneration: UInt64 = 1

    /// `true` while the platform is dispatching effects from a self-driven
    /// commit. Suppresses redundant generation bumps from `textWillChange`
    /// firing on candidate taps / self-commits. Platform suppression flag —
    /// not view state — so excluded from the observation graph.
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
    public func bumpGeneration() {
        currentGeneration &+= 1
    }

    // MARK: - Composing Operations

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

    // The hyphen is the POJ / TL syllable separator.
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
    public func fetchContinuousCandidates() -> [RustEngineBridge.ContinuousCandidate] {
        let settings = settingsProvider.current
        let generation = currentGeneration

        // v3.5.8 Phase 9 Item 12 — query `custom_dictionary.db` once
        // for the current raw buffer and pass the same `customEntries`
        // to both fetch phases (the result depends only on `rawInput`,
        // which is stable for this synchronous fetch). The engine
        // synthesizes a full-buffer candidate per entry and dedupes
        // `(roman, hanji)` against the FST hits.
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

        // §34/S22 — invert the 顯示當咧拍的字 setting into the engine's
        // `disabled` wire flag. Computed once from the same snapshot and
        // shared by both fetch phases so a mid-fetch settings change cannot
        // make the two phases disagree (mirrors `enabledSourcesBitmask`).
        let literalRomanCandidateDisabled = !settings.isLiteralRomanCandidateEnabled

        // Phase 1: neutral fetch to learn candidate displayText keys.
        let neutral = RustEngineBridge.composingFetchAtPos(
            mode: settings.inputMode,
            toggles: settings.toneToggles,
            effectiveSwapped: spacing.effectiveSwapped,
            outputBothScripts: spacing.outputBothScripts,
            candidateDisplayMode: settings.candidateDisplayMode,
            generation: generation,
            customEntries: customEntries,
            enabledSourcesBitmask: enabledSourcesBitmask,
            literalRomanCandidateDisabled: literalRomanCandidateDisabled,
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
            candidateDisplayMode: settings.candidateDisplayMode,
            generation: generation,
            frequencyEntries: entries,
            nowMs: nowMs,
            customEntries: customEntries,
            enabledSourcesBitmask: enabledSourcesBitmask,
            literalRomanCandidateDisabled: literalRomanCandidateDisabled,
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
        // R5 pair-key (#7): one `FrequencyEntry` per `(word, tl)` ROW so the
        // engine can build a `(display_text, canonical_tl)`-keyed map. A
        // word may yield several rows (each learned reading + the legacy
        // `tl == ""` bucket); the engine's tolerant `get` resolves them.
        let rows = service.frequencyDataBatch(for: uniqueKeys)
        return rows.map { row in
            var entry = Taigi_Engine_FrequencyEntry()
            entry.displayTextKey = row.word
            entry.canonicalTl = row.tl
            entry.count = UInt32(max(0, row.data.count))
            entry.lastUsedMs = row.data.lastUsedMillis
            return entry
        }
    }

    /// v3.5.8 Phase 9 Item 12 — query `custom_dictionary.db` for the
    /// current raw buffer and marshal matches into the proto
    /// `CustomDictEntry[]` carried by `FetchAtPos`. The engine owns the
    /// merge + `(roman, hanji)` dedupe + ranking (spec G3 — platform
    /// never re-ranks); this method only fetches + marshals.
    ///
    /// Query path: `CustomDictionaryDerivation.queryKey(for:mode:)` →
    /// `CustomDictionaryRepository.searchSync(family:form:key:)` (cross-mode
    /// side-table join, parameterized SQL).
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
    private func buildCustomEntries(
        rawInput: String,
        settings: EngineSettings,
    ) -> [Taigi_Engine_CustomDictEntry] {
        guard settings.isCustomDictEnabled, !rawInput.isEmpty,
              let q = CustomDictionaryDerivation.queryKey(for: rawInput, mode: settings.inputMode)
        else { return [] }
        let rows = customDictionaryRepository.searchSync(
            family: q.family,
            form: q.form,
            key: q.key,
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
            candidateDisplayMode: settings.candidateDisplayMode,
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
    public func resetContinuous() {
        logger.debug("[COMPOSE] fn=resetContinuous")
        applyAsSelfCommit(RustEngineBridge.composingResetContinuous(generation: currentGeneration))
    }

    public func deleteBackward() {
        logger.debug("[COMPOSE] fn=deleteBackward")
        let settings = settingsProvider.current
        apply(RustEngineBridge.composingDeleteBackward(
            mode: settings.inputMode,
            toggles: settings.toneToggles,
            generation: currentGeneration,
        ))
    }

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
            candidateDisplayMode: settings.candidateDisplayMode,
            generation: currentGeneration,
        ))
    }

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
        let settings = settingsProvider.current
        let spacing = Self.continuousSpacingFlags(settings)
        applyAsSelfCommit(RustEngineBridge.composingCommitRaw(
            mode: settings.inputMode,
            toggles: settings.toneToggles,
            effectiveSwapped: spacing.effectiveSwapped,
            outputBothScripts: spacing.outputBothScripts,
            candidateDisplayMode: settings.candidateDisplayMode,
            generation: currentGeneration,
        ))
    }

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
            candidateDisplayMode: settings.candidateDisplayMode,
            generation: currentGeneration,
        ))
    }

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
            candidateDisplayMode: settings.candidateDisplayMode,
            generation: currentGeneration,
        ))
    }

    /// Commit the currently-selected candidate, given the visible candidate
    /// strings. KK-side callers pass `suggestions.map(\.text)`.
    public func confirmSelectedCandidate(availableTexts: [String]) -> Bool {
        logger.debug("[COMPOSE] fn=confirmSelectedCandidate index=\(selectedCandidateIndex) count=\(availableTexts.count)")
        guard isComposing,
              selectedCandidateIndex >= 0,
              selectedCandidateIndex < availableTexts.count
        else { return false }
        selectSuggestion(text: availableTexts[selectedCandidateIndex])
        return true
    }

    // Clears the composition without committing it.
    public func reset() {
        logger.debug("[COMPOSE] fn=reset")
        applyAsSelfCommit(RustEngineBridge.composingReset(generation: currentGeneration))
    }

    public func setSelectedCandidateIndex(_ index: Int) {
        logger.debug("[COMPOSE] fn=setSelectedCandidateIndex index=\(index)")
        apply(RustEngineBridge.composingSetSelectedCandidateIndex(index, generation: currentGeneration))
    }

    // MARK: - Apply Transition (three-phase, see boundary doc §2.4)

    // Self-commit variant of `apply`: the `selfCommitInProgress` flag suppresses a redundant generation bump.
    private func applyAsSelfCommit(_ transition: RustEngineBridge.ComposingTransition) {
        selfCommitInProgress = true
        defer { selfCommitInProgress = false }
        apply(transition)
    }

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
