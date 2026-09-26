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
    private let usageRecorder: any UsageRecorder
    private let nextWord: any NextWordPort
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
    /// `settingsProvider`, `usageRecorder` and `nextWord` have no
    /// defaults on purpose: the shipped ones read the user's real
    /// `UserDefaults` and count into the engine's stores of the user's data,
    /// and a defaulted parameter is how a test — or a second production path
    /// added later — would silently end up driving the engine from settings it
    /// never meant to read, or teaching the user's own store from a fixture.
    init(
        settingsProvider: EngineSettingsProvider,
        usageRecorder: any UsageRecorder,
        nextWord: any NextWordPort,
        startingGeneration: UInt64 = 1,
    ) {
        self.settingsProvider = settingsProvider
        self.usageRecorder = usageRecorder
        self.nextWord = nextWord
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
        // The next-word context is dropped along with the composition, and for
        // a sharper reason: a session change is usually a change of
        // application, and carrying the context across one would learn the last
        // word typed in a chat window as the predecessor of the first word
        // typed in a terminal. The engine's own state resets on the new
        // generation, but only when it next receives a request under it — this
        // is what makes that happen now rather than at the next commit.
        nextWord.forgetContext(
            settings: settingsProvider.current,
            generation: currentGeneration,
        )
    }

    /// A character reached the host without going through a composition.
    ///
    /// Forwarded so the engine can end the current context on sentence-end
    /// punctuation, which is what stops the last word of one sentence being
    /// learned as the predecessor of the first word of the next. Whether a
    /// given character does that — and whether it is noise that should change
    /// nothing at all — is the engine's call
    /// (`engine/nextword/src/decide.rs:115`), not this method's.
    ///
    /// Letters are excluded because a letter starts a composition rather than
    /// reaching the host on its own, so one arriving here is not a word and
    /// must not become the context. Whitespace is excluded because it can never
    /// be sentence-end punctuation, and forwarding it would put an engine
    /// round-trip on every space bar press outside a composition.
    func noteCharacterTypedOutsideComposition(_ character: String) {
        guard !character.isEmpty,
              !character.contains(where: { $0.isLetter || $0.isWhitespace })
        else { return }
        nextWord.wordSelected(
            text: character,
            roman: "",
            settings: settingsProvider.current,
            generation: currentGeneration,
        )
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

    /// Applies one Telex key — a tone letter, `z` or `f` — to the pending
    /// syllable. Shaped like `append` because it is the same step with the
    /// engine deciding what the key writes (`engine/composing/src/telex.rs`):
    /// an idle `z` starts a composition the way a letter does, and the
    /// promotion afterwards is what keeps a Telex-typed syllable on the same
    /// continuous phase an appended one reaches.
    func telexKey(_ key: String, executing executor: ComposingEffectExecutor) {
        Self.logger.debug("telexKey '\(key)'")
        let settings = settingsProvider.current
        apply(
            RustEngineBridge.composingTelexKey(
                key,
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

    /// Steps the caret inside the pending tail. Not a buffer change: no
    /// promotion, and the engine asks for no fetch — the candidates on screen
    /// still describe the same text.
    func moveCaret(_ direction: CaretDirection, executing executor: ComposingEffectExecutor) {
        Self.logger.debug("moveCaret \(String(describing: direction))")
        apply(
            RustEngineBridge.composingMoveCaret(
                direction,
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
    /// Answers the text the commit wrote, or nil for a commit that never
    /// reached the engine or wrote nothing — what the controller's auto-space
    /// earns its trailing space from. Discardable because the lifecycle
    /// commits have no use for it.
    @discardableResult
    func commitComposition(executing executor: ComposingEffectExecutor) -> String? {
        Self.logger.debug("commitComposition")
        let transition = RustEngineBridge.composingCommitRaw(
            settings: settingsProvider.current,
            generation: currentGeneration,
        )
        apply(transition, executing: executor)
        return Self.committedText(of: transition)
    }

    /// Commits the composition and appends `text` after it in the same engine
    /// step, so one keystroke reaches the host as one document mutation.
    @discardableResult
    func commitComposition(thenInsert text: String, executing executor: ComposingEffectExecutor) -> String? {
        Self.logger.debug("commitCompositionThenInsert '\(text)'")
        let settings = settingsProvider.current
        let transition = RustEngineBridge.composingCommitPreeditThenInsertExternal(
            text,
            settings: settings,
            generation: currentGeneration,
        )
        apply(transition, executing: executor)
        // This is the one commit path the engine does not describe to the
        // learner: it emits `NextWordClearForNewComposing` and no
        // `NextWordWordSelected` (`engine/composing/src/transition.rs:769-780`),
        // unlike Return, which does (`:483`). The word that just went into the
        // document therefore never becomes the context — and if the context
        // were left alone, the NEXT commit would pair itself with whatever was
        // committed BEFORE this one, learning a bigram that skips a word.
        //
        // Dropping the context is the safe half of that: it under-learns one
        // pair rather than learning a wrong one. Synthesizing the missing
        // handshake here is the alternative, and it is worse — the platform
        // would have to supply a canonical reading for the committed
        // composition, which only the engine knows, and a guessed one is
        // written into `prev_tl` for everything that follows.
        nextWord.forgetContext(settings: settings, generation: currentGeneration)
        return Self.committedText(of: transition)
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

    /// Reads the candidates for the composition as it currently stands, ranked
    /// against what the user has committed before.
    ///
    /// One fetch: the engine reads the user's own data itself — the counts,
    /// the custom dictionary (unless the setting turns it off) and the learned
    /// phrases — and ranks in the same call
    /// (`docs/architecture/user-data-engine-roadmap.md` P6).
    ///
    /// Sent under the composition's existing generation because the query is
    /// read-only: bumping the generation would reset the engine before the
    /// query ran (`engine/composing/src/handle.rs:61-66`).
    ///
    /// The answer carries the authoritative composition state, so it is
    /// mirrored whatever it says — an idle answer means the composition is
    /// gone, and the mirror must not go on claiming it.
    func fetchCandidates() -> CandidateFetchOutcome {
        let settings = settingsProvider.current
        guard let fetched = RustEngineBridge.composingFetchAtPos(
            settings: settings,
            generation: currentGeneration,
            nowMs: Int64(Date().timeIntervalSince1970 * 1000),
            enabledSourcesBitmask: RustEngineBridge
                .enabledSourcesBitmask(for: settings.dictionarySources),
        ) else { return .unavailable }

        mirror(fetched.transition)
        guard let candidates = fetched.candidates else { return .notComposing }
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

    /// How the candidate window presents `candidates` under the settings in
    /// force right now — the cells, and which candidate and script each one
    /// commits (`PresentedCandidate`) — together with whether the first cell
    /// is the §34 literal, which takes no slot key.
    ///
    /// Alongside `documentText` rather than derived from it: a cell shows one
    /// script or splits the two into columns while the document string may
    /// bracket them into one, so they share the settings snapshot, not the
    /// formatting. ONE snapshot for the whole list — it is a few dozen
    /// defaults reads, and a bar is rebuilt per keystroke — and one for BOTH
    /// answers, so the cells and the key row can never be resolved against
    /// two different instants.
    func presentation(
        for candidates: [ContinuousCandidate],
    ) -> (cells: [PresentedCandidate], leadsWithLiteralRoman: Bool) {
        let settings = settingsProvider.current
        return (
            PresentedCandidate.presentation(of: candidates, settings: settings),
            Self.leadsWithLiteralRomanCandidate(candidates, settings: settings),
        )
    }

    /// Whether `candidates` leads with the §34 literal — the WYSIWYG
    /// romanization the engine prepends at index 0 while Show Typed Text First is on
    /// (`engine/composing/src/dispatch.rs:260-268`). That cell takes no slot
    /// key: it is what the user is already typing, not an offer to pick
    /// (USER 2026-09-09), so the keys start on the cell after it
    /// (`CandidateIndexLabel.candidateIndex(forKeySlot:leadCellIsUnkeyed:indexForSlot:)`).
    ///
    /// Read off the setting plus the shape of the leading candidate rather
    /// than re-derived: the literal is roman-only by construction
    /// (`dispatch.rs:344` `hanji: None`), and the engine's other gate — the
    /// TPS buffer that suppresses the prepend (`dispatch.rs:331`) — cannot
    /// arise on the desktop, where the only modes are TL and POJ
    /// (`InputMode`). A hanji-bearing lead means the prepend did not happen,
    /// whatever the setting says, and every cell keeps its key.
    static func leadsWithLiteralRomanCandidate(
        _ candidates: [ContinuousCandidate],
        settings: EngineSettings,
    ) -> Bool {
        guard settings.isLiteralRomanCandidateEnabled, let first = candidates.first else {
            return false
        }
        return first.presentableHanji == nil
    }

    /// Commits `candidate`, which must come from the `fetchCandidates()` call
    /// that produced the list the user is looking at.
    ///
    /// The document string is derived here rather than taken from the caller:
    /// it depends on the same settings snapshot the engine call does, and
    /// letting a view layer supply it is how the two drift apart.
    ///
    /// The document text is not the identity the word is learnt under: that
    /// stays the `(display text, canonical TL)` pair (`CLAUDE.md` Core
    /// Principle #7), so a change of output script does not split a word's
    /// frequency or association rows.
    /// `script` picks WHICH of the candidate's two renderings the document
    /// gets. Still derived here rather than handed in as a string, for the
    /// reason above — the caller says which one it wants, never what it says.
    ///
    /// `.alternate` on a candidate that HAS no second script answers `.ignored`
    /// without reaching the engine, and that is the whole answer: the caller
    /// asked for something this candidate cannot give, so nothing is written
    /// and nothing is learnt. Deciding it here rather than behind a caller-side
    /// pre-check keeps one decision point — a guard at the call site plus a
    /// fallback here would be two rules for one case, free to disagree.
    /// The verdict comes back WITH the text because only the branch that
    /// picked the rendering knows it — the auto-space gate must not re-derive
    /// it from the output mode (`AutoSpacePolicy.isGateActive`). `nil` for a
    /// commit that wrote nothing, which earns nothing either way.
    func commitCandidate(
        _ candidate: ContinuousCandidate,
        script: CandidateScript = .primary,
        executing executor: ComposingEffectExecutor,
    ) -> (outcome: CandidateCommitOutcome, commit: ResolvedCommit?) {
        let settings = settingsProvider.current
        Self.logger.debug("commitCandidate consumedBytes=\(candidate.consumedSpanEnd)")
        let resolved: ResolvedCommit
        switch script {
        case .primary:
            resolved = CandidateDocumentText.resolved(for: candidate, settings: settings)
        case .alternate:
            guard let alternate = CandidateDocumentText.resolvedAlternate(
                for: candidate, settings: settings,
            ) else { return (.ignored, nil) }
            resolved = alternate
        }
        guard let transition = RustEngineBridge.composingCommitContinuous(
            documentText: resolved.text,
            canonicalText: candidate.displayText,
            associationTl: candidate.canonicalTl,
            hanji: candidate.hanji,
            consumedBytes: candidate.consumedSpanEnd,
            syllableCount: candidate.syllableCount,
            settings: settings,
            generation: currentGeneration,
        ) else { return (.unavailable, nil) }

        let outcome = CandidateCommitOutcome(transition)
        apply(transition, executing: executor)
        recordUsage(of: candidate, after: outcome, settings: settings)
        // The ENGINE's text with OUR verdict: the engine decides what actually
        // reached the document, this branch decided which script that is.
        return (outcome, Self.committedText(of: transition).map {
            ResolvedCommit(text: $0, wroteRomanization: resolved.wroteRomanization)
        })
    }

    /// The text `transition` wrote to the document, when it committed one.
    ///
    /// Read off the effects — the same signal `CandidateCommitOutcome` reads —
    /// because the mirror carries the display rendering, which the output
    /// settings can make differ from the document string.
    private static func committedText(of transition: ComposingTransition?) -> String? {
        transition?.effects.lazy.compactMap { effect in
            if case let .commitTextReplacingPreedit(text) = effect {
                return text
            }
            return nil
        }.last
    }

    /// Counts a candidate the engine confirmed it took.
    ///
    /// Gated on the effect-backed outcome rather than on the mirror: `.ignored`
    /// can follow a composition the engine reset out from under the commit, and
    /// counting there would promote a word the user never got. The identity
    /// written is the `(display text, canonical TL)` pair the ranker looks the
    /// candidate up by (`CLAUDE.md` Core Principle #7) — recording under the
    /// document rendering instead would key the row on a string that changes
    /// with the Hanji/romanization settings.
    private func recordUsage(
        of candidate: ContinuousCandidate,
        after outcome: CandidateCommitOutcome,
        settings: EngineSettings,
    ) {
        switch outcome {
        case .nailed, .finalized:
            // The setting gates the count only; the engine still touches a
            // learned phrase picked whole (§50 touch-on-use — learning is
            // always on).
            usageRecorder.record(Usage(
                displayText: candidate.displayText,
                canonicalTl: candidate.canonicalTl,
                hanji: candidate.hanji,
                isFrequencyRecordingEnabled: settings.isFrequencyRecordingEnabled,
            ))
        case .ignored, .unavailable:
            break
        }
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
        mirror(transition)

        let settings = settingsProvider.current
        for effect in transition.effects {
            switch effect {
            // The learning handshakes are not document effects, and the
            // executor writes into a client's document. Routing them here keeps
            // the executor's one job intact and keeps the generation — which
            // only this type knows — out of the effect path.
            case let .nextWordWordSelected(text, roman, _):
                nextWord.wordSelected(
                    text: text,
                    roman: roman,
                    settings: settings,
                    generation: currentGeneration,
                )
            case let .nextWordUpdateLastSelectedWord(text, roman):
                nextWord.segmentNailed(
                    text: text,
                    roman: roman,
                    settings: settings,
                    generation: currentGeneration,
                )
            case .nextWordClearForNewComposing:
                // Hides predictions while keeping the context. macOS shows no
                // predictions, so there is nothing to hide and the context is
                // exactly what must survive: forwarding it would spend a
                // round-trip to bump a generation nothing reads.
                break
            case .phraseLearned:
                // §50 — the engine decided the composition was a phrase and,
                // with the user data open, already wrote it to
                // `learned_phrases.db`; nothing is left for this side.
                break
            // Listed rather than defaulted: an effect added to the engine later
            // has to be classified here, and a `default` would quietly file it
            // under "write it into the user's document".
            case .updatePreedit,
                 .clearPreeditWithoutCommit,
                 .commitTextReplacingPreedit,
                 .deleteBackwardFromDocument,
                 .resetAutocomplete,
                 .performAutocomplete,
                 .resetAutocompleteContext:
                executor.execute(effect)
            }
        }
    }

    /// Updates the mirror from an engine answer, without performing anything.
    ///
    /// The read paths use this directly: a fetch is a query, and its response
    /// still carries the authoritative composition state, so ignoring it is how
    /// the mirror ends up claiming a composition the engine has already reset.
    private func mirror(_ transition: ComposingTransition) {
        isComposing = transition.isComposing
        rawInput = transition.rawInput
        displayText = transition.displayText
    }

    private func clearMirror() {
        isComposing = false
        rawInput = ""
        displayText = ""
    }
}
