// Platform wrapper around the Rust composing engine: one intent in, mirror
// updated and effects dispatched out.

import Foundation

/// Turns a user intent into an engine round-trip, mirrors what came back, and
/// hands the engine's effects to the executor for the client that asked.
///
/// The engine owns the composition (phase, raw buffer, candidate index); this
/// type owns only a mirror of the last answer, which the controller reads to
/// decide whether a key belongs to the composition or to the host.
///
/// One instance per process, held by `ComposingSessionCoordinator`, which is
/// where the process-wide engine state it stands for is explained — a second
/// instance would hand out generations the first one knows nothing about, and
/// the engine drops its state whenever the generation moves.
@MainActor
final class ComposingManager {
    // MARK: - Engine mirror

    private(set) var isComposing = false
    /// What the user typed, with numeric tones. This is the engine's search key.
    ///
    /// Not the length of the composition on screen: once a candidate has been
    /// nailed this is only the pending tail, while the marked region holds the
    /// nailed prefix as well (`engine/composing/src/transition.rs:585`). Anything
    /// measuring the rendered composition must read `displayText`.
    private(set) var rawInput = ""

    /// The composition as the marked region renders it — the whole thing,
    /// nailed prefix included. Mirrored rather than derived because the engine
    /// is the only thing that knows how segments join (the word-boundary
    /// spacing rules differ per output mode), and because the candidate window
    /// has to anchor itself to the end of what is actually on screen.
    private(set) var displayText = ""

    private let settingsProvider: EngineSettingsProvider
    private static let logger = DebugLogger(category: "ComposingManager")

    /// Generations must be unique across everything that talks to the engine,
    /// which is why the starting value is a parameter: the engine keeps one
    /// composition per process and drops it whenever the generation it is
    /// handed changes, so two managers counting from the same value would
    /// silently wipe each other's state.
    private var currentGeneration: UInt64

    /// The default starts at 1 because 0 is the generation an unset proto field
    /// carries; keeping them apart means a request that forgot to set one
    /// cannot be mistaken for a request from the first session.
    /// `settingsProvider` has no default on purpose: the shipped one reads the
    /// user's real `UserDefaults`, and a defaulted parameter is how a test — or
    /// a second production path added later — would silently end up driving the
    /// engine from settings it never meant to read.
    init(
        settingsProvider: EngineSettingsProvider,
        startingGeneration: UInt64 = 1,
    ) {
        self.settingsProvider = settingsProvider
        currentGeneration = startingGeneration
    }

    // MARK: - Session lifecycle

    /// Abandons any composition without touching a document, so the next
    /// session starts from an idle engine.
    ///
    /// Deliberately sends nothing: the engine drops its state as soon as it
    /// sees the new generation (`handle.rs:61-66`), and a `Reset` sent here
    /// would find the state already gone and no-op on Idle. The mirror is
    /// cleared locally for the same reason — there is no response to mirror.
    ///
    /// This does NOT clear marked text: the region belongs to a client this
    /// manager has no reference to. Clearing it is the outgoing controller's
    /// job, on its way out, while it still has its client.
    func startNewSession() {
        currentGeneration &+= 1
        clearMirror()
    }

    /// Appends one typed character. The engine starts a composition when it is
    /// idle (`engine/composing/src/transition.rs:55`), so there is no separate
    /// "begin" call and no platform-side phase check that could disagree with
    /// the engine's.
    func append(_ character: String, executing executor: ComposingEffectExecutor) {
        Self.logger.debug("append '\(character)'")
        let settings = settingsProvider.current
        apply(
            RustEngineBridge.composingAppend(
                character,
                settings: settings,
                generation: currentGeneration,
            ),
            executing: executor,
        )
        promoteToContinuous(settings: settings, executing: executor)
    }

    /// Drops the last character of the raw buffer. Ends the composition when
    /// that empties it.
    func deleteBackward(executing executor: ComposingEffectExecutor) {
        Self.logger.debug("deleteBackward")
        apply(
            RustEngineBridge.composingDeleteBackward(
                settings: settingsProvider.current,
                generation: currentGeneration,
            ),
            executing: executor,
        )
    }

    /// Commits the composition as rendered. Under the continuous phase the
    /// engine builds the text from its own state — `Σ nailed.display_text +
    /// derived(pending)` (`transition.rs:443`) — so passing the mirrored
    /// display text back in would double-count the nailed prefix.
    func commitComposition(executing executor: ComposingEffectExecutor) {
        Self.logger.debug("commitComposition")
        apply(
            RustEngineBridge.composingCommitRaw(
                settings: settingsProvider.current,
                generation: currentGeneration,
            ),
            executing: executor,
        )
    }

    /// Commits the composition and appends `text` after it in the same engine
    /// step, so one keystroke reaches the host as one document mutation.
    func commitComposition(thenInsert text: String, executing executor: ComposingEffectExecutor) {
        Self.logger.debug("commitCompositionThenInsert '\(text)'")
        apply(
            RustEngineBridge.composingCommitPreeditThenInsertExternal(
                text,
                settings: settingsProvider.current,
                generation: currentGeneration,
            ),
            executing: executor,
        )
    }

    /// Abandons the composition. Nothing reaches the document: under the
    /// engine's model the preedit was never document text.
    func cancelComposition(executing executor: ComposingEffectExecutor) {
        Self.logger.debug("cancelComposition")
        apply(
            RustEngineBridge.composingReset(generation: currentGeneration),
            executing: executor,
        )
    }

    // MARK: - Candidates

    /// Reads the candidates for the composition as it currently stands.
    ///
    /// Sent under the composition's existing generation because the query is
    /// read-only: bumping the generation would reset the engine before the
    /// query ran (`engine/composing/src/handle.rs:61-66`).
    func fetchCandidates() -> CandidateFetchOutcome {
        guard let result = RustEngineBridge.composingFetchAtPos(
            settings: settingsProvider.current,
            generation: currentGeneration,
        ) else { return .unavailable }

        guard let candidates = result.candidates else { return .notComposing }
        return .found(candidates)
    }

    /// What committing `candidate` would write into the document, under the
    /// settings in force right now.
    ///
    /// Exposed so the candidate bar can label a cell with the string that cell
    /// produces. Rendering it in the view instead would be a second copy of the
    /// output-mode rules, and the two would disagree the moment one of them
    /// read a different settings snapshot than the commit did.
    func documentText(for candidate: ContinuousCandidate) -> String {
        CandidateDocumentText.text(for: candidate, settings: settingsProvider.current)
    }

    /// Commits `candidate`, which must come from the `fetchCandidates()` call
    /// that produced the list the user is looking at.
    ///
    /// The document rendering is derived here rather than taken from the
    /// caller: it depends on the same settings snapshot the engine call does,
    /// and letting a view layer supply it is how the two drift apart.
    func commitCandidate(
        _ candidate: ContinuousCandidate,
        executing executor: ComposingEffectExecutor,
    ) -> CandidateCommitOutcome {
        let settings = settingsProvider.current
        Self.logger.debug("commitCandidate consumedBytes=\(candidate.consumedSpanEnd)")
        guard let transition = RustEngineBridge.composingCommitContinuous(
            documentText: CandidateDocumentText.text(for: candidate, settings: settings),
            canonicalText: candidate.displayText,
            associationTl: candidate.canonicalTl,
            consumedBytes: candidate.consumedSpanEnd,
            syllableCount: candidate.syllableCount,
            settings: settings,
            generation: currentGeneration,
        ) else { return .unavailable }

        let outcome = CandidateCommitOutcome(transition)
        apply(transition, executing: executor)
        return outcome
    }

    /// Promotes the composition into the continuous phase, where the engine
    /// segments the whole buffer instead of one syllable.
    ///
    /// Sent unconditionally on the same call stack as the character that
    /// triggered it, sharing its generation: the engine no-ops on an empty
    /// buffer and on an already-continuous composition (`transition.rs:496-502`),
    /// so a platform-side eligibility rule would only be a second, drifting
    /// copy of that decision — and deferring the call to a later turn would let
    /// a generation bump land in between and wipe the composition it promotes.
    private func promoteToContinuous(
        settings: EngineSettings,
        executing executor: ComposingEffectExecutor,
    ) {
        apply(
            RustEngineBridge.composingEnterContinuous(
                settings: settings,
                generation: currentGeneration,
            ),
            executing: executor,
        )
    }

    /// Mirror first, then run the effects in the order the engine listed them.
    /// The order is the engine's instruction, not an implementation detail: a
    /// commit that ran before the preedit update it replaces would leave the
    /// old composition on screen.
    ///
    /// `nil` is a round-trip that never reached the engine, which left it
    /// exactly as it was: there is nothing to mirror and nothing to perform,
    /// and claiming otherwise would leave the mirror describing a composition
    /// the engine is still holding.
    private func apply(
        _ transition: ComposingTransition?,
        executing executor: ComposingEffectExecutor,
    ) {
        guard let transition else { return }

        isComposing = transition.isComposing
        rawInput = transition.rawInput
        displayText = transition.displayText

        for effect in transition.effects {
            executor.execute(effect)
        }
    }

    private func clearMirror() {
        isComposing = false
        rawInput = ""
        displayText = ""
    }
}
