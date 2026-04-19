import Foundation

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.

/// Pure decision + filtering layer for NextWord prediction.
///
/// Split from `NextWordController` (G5-impl): the controller remains the
/// iOS platform executor (Timer, @MainActor, settings read, generation
/// bookkeeping, UI fan-out); this engine owns validation, state
/// transitions, association-window math, compound-word splitting, and
/// prediction filtering.
///
/// **Invariants** (see `docs/architecture/nextword-engine-boundary.md`):
/// - No clock reads. Callers supply `nowMs` via `NextWordDecisionInput`.
/// - No settings provider. Callers pass a `NextWordEngineSettings` value.
/// - No Timer / DispatchQueue / Task. Scheduling is described as effects
///   the executor interprets.
/// - Any state-invalidating intent bumps `currentGeneration` so late
///   async prediction results can be discarded by the executor.
enum NextWordEngine {
    // MARK: - Constants

    /// Strict-less-than association window. A gap of exactly 10_000 ms does
    /// NOT record; negative deltas (clock skew) also do not record.
    static let associationTimeoutMs: Int64 = 10000

    /// Context timeout scheduled via `.rescheduleContextTimeout(after:)`.
    static let contextTimeoutSeconds: TimeInterval = 30.0

    /// Punctuation that ends a sentence and resets NextWord state.
    static let sentenceEndPunctuation: Set<Character> = ["。", "！", "？", ".", "!", "?"]

    /// Superset of sentence-end: any char in this set aborts `wordSelected`
    /// without recording or predicting (punctuation/whitespace/number paths).
    private static let noisePunctuation: Set<Character> = Set(
        "。！？.!?，,、；;：:「」『』\"\"\u{2018}\u{2019}（）()【】[]{}—–-～~…·",
    )

    // MARK: - Entry Points

    /// Decide what the platform executor should do in response to a single
    /// lifecycle event. Pure function: same inputs → same outputs.
    static func decide(
        intent: NextWordIntent,
        state: NextWordPersistedState,
        input: NextWordDecisionInput,
    ) -> NextWordOutcome {
        switch intent {
        case let .wordSelected(text, roman, requireRomanMode, triggerPrediction):
            decideWordSelected(
                text: text,
                roman: roman,
                requireRomanMode: requireRomanMode,
                triggerPrediction: triggerPrediction,
                state: state,
                input: input,
            )
        case let .backspace(lastChar):
            decideBackspace(lastChar: lastChar, state: state, input: input)
        case .contextTimeoutFired:
            decideContextTimeoutFired(state: state)
        case .clearForNewComposing:
            decideClearForNewComposing(state: state)
        case .resetFull:
            decideResetFull(state: state)
        }
    }

    /// Filter + shape raw service predictions into UI-ready values.
    /// Pure function mirroring today's `makePredictions(from:)` but typed
    /// on the shared DTO so it can move to shared-core without depending
    /// on `NextWordService`.
    static func filterPredictions(
        _ raw: [RawNextWordPrediction],
        settings: NextWordEngineSettings,
    ) -> [EnginePrediction] {
        raw.compactMap { prediction in
            if !settings.isTranslateSwapped, prediction.tl.isEmpty {
                return nil
            }

            let roman = settings.inputMode == .poj
                ? RomanizationConverter.tlToPOJ(prediction.tl)
                : prediction.tl
            let text = roman.isEmpty ? prediction.hanzi : roman
            let subtitle: String? = roman.isEmpty ? nil : prediction.hanzi

            return EnginePrediction(
                text: text,
                subtitle: subtitle,
                hanzi: prediction.hanzi,
                tl: prediction.tl,
            )
        }
    }

    // MARK: - Pure Helpers (exposed for testing)

    /// Association window check. Strict `<` at 10s; negative deltas (clock
    /// skew / wrapping) return false — a deliberate fix vs the pre-split
    /// code, which would have returned true for a future-relative state.
    static func shouldRecordAssociation(
        state: NextWordPersistedState,
        nowMs: Int64,
    ) -> Bool {
        guard state.lastSelectedWord != nil else { return false }
        let delta = nowMs - state.lastSelectionTimeMs
        guard delta >= 0 else { return false }
        return delta < associationTimeoutMs
    }

    /// Split a compound word on `-` (e.g. "tshit-niû" → ["tshit", "niû"]).
    /// Empty parts are dropped so `"-a-"` yields `["a"]`, not `["", "a", ""]`.
    static func splitCompound(_ word: String) -> [String] {
        guard !word.isEmpty else { return [] }
        return word.split(separator: "-").map(String.init).filter { !$0.isEmpty }
    }

    /// Build sequential bigram pairs from a compound word. Order preserved
    /// so the executor can feed them to `NextWordService.recordAssociation`
    /// sequentially (parallel writes would race on the SQLite UNIQUE index).
    static func compoundAssociationPairs(
        displayText: String,
        roman: String,
    ) -> [NextWordAssociationPair] {
        let parts = splitCompound(displayText)
        let romanParts = splitCompound(roman)
        guard parts.count > 1 else { return [] }

        var pairs: [NextWordAssociationPair] = []
        pairs.reserveCapacity(parts.count - 1)
        for i in 0 ..< (parts.count - 1) {
            let prevTl = romanParts.indices.contains(i) ? romanParts[i] : ""
            let nextTl = romanParts.indices.contains(i + 1) ? romanParts[i + 1] : ""
            pairs.append(NextWordAssociationPair(
                prev: parts[i],
                prevTl: prevTl,
                next: parts[i + 1],
                nextTl: nextTl,
            ))
        }
        return pairs
    }

    // MARK: - Intent Handlers

    private static func decideWordSelected(
        text: String,
        roman: String,
        requireRomanMode: Bool,
        triggerPrediction: Bool,
        state: NextWordPersistedState,
        input: NextWordDecisionInput,
    ) -> NextWordOutcome {
        // Enter commits raw romanization only; skip entirely in Hanji mode.
        if requireRomanMode, input.settings.isTranslateSwapped {
            return NextWordOutcome(newState: state, effects: [])
        }

        // Noise text (punctuation / whitespace / digits) never records or
        // predicts. Sentence-end punctuation is a *subset* of noise: it
        // additionally resets state + clears UI, so we branch on it inside
        // the noise guard before returning.
        guard !text.isEmpty, !isNoiseText(text) else {
            if isSentenceEndPunctuation(text) {
                return resetAndClearPredictions(state: state)
            }
            return NextWordOutcome(newState: state, effects: [])
        }

        // pojToTL is idempotent on TL input — safe for POJ and TPS alike.
        let textTl = RomanizationConverter.pojToTL(roman)
        let prevTl = RomanizationConverter.pojToTL(state.lastSelectedRoman ?? "")

        var effects: [NextWordOutcome.Effect] = []

        if input.settings.isAssociationRecordingEnabled {
            if shouldRecordAssociation(state: state, nowMs: input.nowMs),
               let prevWord = state.lastSelectedWord
            {
                effects.append(.recordAssociation(NextWordAssociationPair(
                    prev: prevWord,
                    prevTl: prevTl,
                    next: text,
                    nextTl: textTl,
                )))
            }
            let compound = compoundAssociationPairs(displayText: text, roman: textTl)
            if !compound.isEmpty {
                effects.append(.recordCompoundAssociations(compound))
            }
        }

        effects.append(.rescheduleContextTimeout(after: contextTimeoutSeconds))

        var newState = state
        newState.lastSelectedWord = text
        newState.lastSelectedRoman = textTl
        newState.lastSelectionTimeMs = input.nowMs
        newState.currentGeneration &+= 1

        if triggerPrediction {
            effects.append(.queryPredictions(
                word: text,
                roman: textTl,
                generation: newState.currentGeneration,
            ))
        }

        return NextWordOutcome(newState: newState, effects: effects)
    }

    private static func decideBackspace(
        lastChar: String,
        state: NextWordPersistedState,
        input: NextWordDecisionInput,
    ) -> NextWordOutcome {
        var newState = state
        newState.lastSelectedWord = lastChar
        newState.lastSelectedRoman = nil
        newState.lastSelectionTimeMs = input.nowMs
        newState.currentGeneration &+= 1

        return NextWordOutcome(
            newState: newState,
            effects: [
                .queryPredictions(word: lastChar, roman: "", generation: newState.currentGeneration),
            ],
        )
    }

    private static func decideContextTimeoutFired(
        state: NextWordPersistedState,
    ) -> NextWordOutcome {
        resetAndClearPredictions(state: state)
    }

    private static func decideClearForNewComposing(
        state: NextWordPersistedState,
    ) -> NextWordOutcome {
        var newState = state
        newState.isShowing = false
        newState.currentGeneration &+= 1

        let effects: [NextWordOutcome.Effect] = state.isShowing
            ? [.clearPredictionsUI(generation: newState.currentGeneration)]
            : []
        return NextWordOutcome(newState: newState, effects: effects)
    }

    private static func decideResetFull(
        state: NextWordPersistedState,
    ) -> NextWordOutcome {
        resetAndClearPredictions(state: state)
    }

    /// Shared reset path used by sentence-end punctuation, context timeout,
    /// and `resetFull` intents. Cancels the Timer, optionally clears the
    /// UI, zeroes state, bumps generation.
    private static func resetAndClearPredictions(
        state: NextWordPersistedState,
    ) -> NextWordOutcome {
        var newState = NextWordPersistedState.initial
        newState.currentGeneration = state.currentGeneration &+ 1

        var effects: [NextWordOutcome.Effect] = [.cancelContextTimeout]
        if state.isShowing {
            effects.append(.clearPredictionsUI(generation: newState.currentGeneration))
        }
        return NextWordOutcome(newState: newState, effects: effects)
    }

    // MARK: - Classification

    /// Punctuation / whitespace / pure-digit text never triggers NextWord.
    static func isNoiseText(_ text: String) -> Bool {
        guard let firstChar = text.first else { return true }
        if noisePunctuation.contains(firstChar) { return true }
        if firstChar.isWhitespace { return true }
        if text.allSatisfy({ $0.isASCII && $0.isNumber }) { return true }
        return false
    }

    static func isSentenceEndPunctuation(_ text: String) -> Bool {
        guard let firstChar = text.first else { return false }
        return sentenceEndPunctuation.contains(firstChar)
    }
}
