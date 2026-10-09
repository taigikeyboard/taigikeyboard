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
    }

    public func appendCharacter(_ char: String) {
        logger.debug("[COMPOSE] fn=appendCharacter char='\(char)'")
        let settings = settingsProvider.current
        apply(RustEngineBridge.composingAppend(
            char,
            settings: settings,
            generation: currentGeneration,
        ))
    }

    // The hyphen is the POJ / TL syllable separator.
    public func appendHyphen() {
        logger.debug("[COMPOSE] fn=appendHyphen")
        let settings = settingsProvider.current
        apply(RustEngineBridge.composingAppendHyphen(
            settings: settings,
            generation: currentGeneration,
        ))
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
    }

    // MARK: - v3.5.8 Phase 7B — Continuous-input adapters

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
    /// desktop-core `ComposingManager::fetch_candidates`.
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

    /// Commit one Continuous candidate. The pick MUST come verbatim from a
    /// `ContinuousCandidate` returned by an immediately preceding
    /// `fetchContinuousCandidates()` call — the engine collapses to noop on
    /// UTF-8/syllable boundary violations, so caller-side validation is
    /// unnecessary.
    ///
    /// The engine resolves the document text, counts the pick (R5), and
    /// answers what the pick did — the outcome callers gate the auto space on.
    /// Model B mid-commit (nail) emits `[UpdatePreedit(whole composition),
    /// NextWordUpdateLastSelectedWord, RefreshCandidates]` and stays
    /// Continuous; final-commit (`consumedBytes >= pending.utf8.count`)
    /// emits `[CommitTextReplacingPreedit(whole composition),
    /// ClearCandidates, ResetCandidateContext, NextWordWordSelected]`
    /// and exits Continuous; a stale generation or a rejected pick emits
    /// nothing and answers `.ignored`.
    public func commitContinuous(_ pick: RustEngineBridge.ContinuousPick) -> RustEngineBridge.ContinuousCommitOutcome {
        logger.debug(
            "[COMPOSE] fn=commitContinuous script=\(pick.script) "
                + "canonicalLen=\(pick.canonicalText.count) "
                + "consumedBytes=\(pick.consumedBytes) syllCount=\(pick.syllableCount)",
        )
        let result = RustEngineBridge.composingCommitContinuous(
            pick,
            settings: settingsProvider.current,
            generation: currentGeneration,
        )
        applyAsSelfCommit(result.transition)
        return result.outcome
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
        // string from engine state, so there is no prefix duplication.
        // Idle → `CommitRaw` is a no-op.
        let settings = settingsProvider.current
        applyAsSelfCommit(RustEngineBridge.composingCommitRaw(
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

    // `apply` as one of the IME's own writes (`performAsSelfCommit`).
    private func applyAsSelfCommit(_ transition: RustEngineBridge.ComposingTransition) {
        performAsSelfCommit { apply(transition) }
    }

    /// Runs `body` with `selfCommitInProgress` set, which keeps the IME's own
    /// document writes from reading as a field switch — including a commit
    /// the platform writes after the engine call returned (iOS
    /// `HostTextWriter.endEvent`).
    func performAsSelfCommit(_ body: () -> Void) {
        selfCommitInProgress = true
        defer { selfCommitInProgress = false }
        body()
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
