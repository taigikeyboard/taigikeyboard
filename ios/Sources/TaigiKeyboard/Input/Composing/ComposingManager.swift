// iOS composing manager — thin wrapper between the Rust composing engine and KeyboardKit / SwiftUI.
// Engine state (phase / rawInput) lives in Rust; this only mirrors it and dispatches effects.

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
/// Engine state (phase + raw input) lives inside
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
    private let logger = DebugLogger(category: "ComposingManager")

    // MARK: - Init

    init(settingsProvider: EngineSettingsProvider = SharedSettings.shared) {
        self.settingsProvider = settingsProvider
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
            settings: settings,
            generation: currentGeneration,
        ))
        promoteToContinuousIfEligible(settings: settings)
    }

    public func appendCharacter(_ char: String) {
        logger.debug("[COMPOSE] fn=appendCharacter char='\(char)'")
        let settings = settingsProvider.current
        apply(RustEngineBridge.composingAppend(
            char,
            settings: settings,
            generation: currentGeneration,
        ))
        promoteToContinuousIfEligible(settings: settings)
    }

    // The hyphen is the POJ / TL syllable separator.
    public func appendHyphen() {
        logger.debug("[COMPOSE] fn=appendHyphen")
        let settings = settingsProvider.current
        apply(RustEngineBridge.composingAppendHyphen(
            settings: settings,
            generation: currentGeneration,
        ))
        promoteToContinuousIfEligible(settings: settings)
    }

    /// TPS auto-correct — swaps the last raw-input character in place.
    public func replaceLastCharacter(with replacement: String) {
        logger.debug("[COMPOSE] fn=replaceLastCharacter replacement='\(replacement)'")
        let settings = settingsProvider.current
        apply(RustEngineBridge.composingReplaceLast(
            replacement,
            settings: settings,
            generation: currentGeneration,
        ))
        promoteToContinuousIfEligible(settings: settings)
    }

    // MARK: - v3.5.8 Phase 7B — Continuous-input adapters

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
            settings: settings,
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
    /// One fetch: the engine reads the user's own data itself — the counts,
    /// the custom dictionary (unless the setting turns it off) and the learned
    /// phrases — and ranks in the same call
    /// (`docs/architecture/user-data-engine-roadmap.md` P7b). `nowMs` is the
    /// clock its recency ranking reads. CROSS-PLATFORM INVARIANT — mirrors
    /// macOS `ComposingManager.fetchCandidates`.
    ///
    /// Bridge-failure handling distinguishes "engine returned Idle" (legit
    /// reset — a `bumpGeneration` the engine saw first; apply the Idle
    /// transition, return `[]`) from "FFI roundtrip failed" (engine state
    /// unchanged — apply nothing, return `[]`): applying the synthesized
    /// `.noop` would clobber the mirror with false Idle state. Codex PR #265
    /// r3216857164.
    public func fetchContinuousCandidates() -> [RustEngineBridge.ContinuousCandidate] {
        let settings = settingsProvider.current
        let fetched = RustEngineBridge.composingFetchAtPos(
            settings: settings,
            generation: currentGeneration,
            nowMs: Int64(Date().timeIntervalSince1970 * 1000),
            // PR-9.6 — the dictionary source-toggle bitmask the Tab3 browse
            // path sends too (`DictionarySearchService.search`).
            enabledSourcesBitmask: RustEngineBridge.lexiconDictionaryFilters(
                toggles: RustEngineBridge.DictionaryToggles(from: settings),
            ).dictionaryFilterBitmask,
            // §34/S22 — invert of the Show Typed Text First setting.
            literalRomanCandidateDisabled: !settings.isLiteralRomanCandidateEnabled,
            customDictionaryDisabled: !settings.isCustomDictEnabled,
        )
        if fetched.isBridgeFailure {
            return []
        }
        apply(fetched.transition)
        return fetched.candidates ?? []
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
        hanji: String? = nil,
        consumedBytes: UInt32,
        syllableCount: UInt32,
    ) -> (didCommit: Bool, didFinalCommit: Bool) {
        logger.debug(
            "[COMPOSE] fn=commitContinuous displayLen=\(displayText.count) "
                + "canonicalLen=\(canonicalText.count) "
                + "consumedBytes=\(consumedBytes) syllCount=\(syllableCount)",
        )
        let settings = settingsProvider.current
        let transition = RustEngineBridge.composingCommitContinuous(
            displayText: displayText,
            canonicalText: canonicalText,
            associationTl: associationTl,
            hanji: hanji,
            consumedBytes: consumedBytes,
            syllableCount: syllableCount,
            settings: settings,
            generation: currentGeneration,
        )
        // Inspect transition BEFORE dispatching effects so we can return an
        // effect-backed signal. `applyAsSelfCommit` body inlined (3 lines)
        // for the same reason — semantics identical to the helper.
        let hasCommitText = transition.effects.contains { effect in
            if case .commitTextReplacingPreedit = effect {
                return true
            }
            return false
        }
        let hasNail = transition.effects.contains { effect in
            if case .nextWordUpdateLastSelectedWord = effect {
                return true
            }
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
            settings: settings,
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
                settings: settings,
                generation: currentGeneration,
            ))
            return
        }
        applyAsSelfCommit(RustEngineBridge.composingCommitRaw(
            settings: settings,
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
        applyAsSelfCommit(RustEngineBridge.composingCommitRaw(
            settings: settings,
            generation: currentGeneration,
        ))
    }

    public func selectSuggestion(text: String) {
        logger.debug("[COMPOSE] fn=selectSuggestion len=\(text.count)")
        let settings = settingsProvider.current
        applyAsSelfCommit(RustEngineBridge.composingSelectSuggestion(
            text,
            settings: settings,
            generation: currentGeneration,
        ))
    }

    public func commitPreeditThenInsertExternal(_ text: String) {
        logger.debug("[COMPOSE] fn=commitPreeditThenInsertExternal len=\(text.count)")
        let settings = settingsProvider.current
        applyAsSelfCommit(RustEngineBridge.composingCommitPreeditThenInsertExternal(
            text,
            settings: settings,
            generation: currentGeneration,
        ))
    }

    // Clears the composition without committing it.
    public func reset() {
        logger.debug("[COMPOSE] fn=reset")
        applyAsSelfCommit(RustEngineBridge.composingReset(generation: currentGeneration))
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
        if isComposing != transition.isComposing {
            isComposing = transition.isComposing
        }
        if rawInput != transition.rawInput {
            rawInput = transition.rawInput
        }
        if !transition.effects.isEmpty, composingText != transition.displayText {
            composingText = transition.displayText
        }

        // Phase 2 — execute platform effects in proto-list order.
        for effect in transition.effects {
            delegate?.execute(effect)
        }

        // Phase 3 — notify composing-context sink once state is settled.
        contextSink?.isComposingText = transition.isComposing
    }
}
