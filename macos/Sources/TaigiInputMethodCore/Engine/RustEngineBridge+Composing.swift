// Composing slice of the engine bridge: the intents macOS sends and the
// decoding of what comes back.

import Foundation

/// The composing intents macOS uses. This is a subset of the engine's sixteen:
///
/// - `AppendHyphen` is skipped because it is a pure alias for `Append("-")`
///   (`engine/composing/src/transition.rs:63-69`), and a hyphen is an ordinary
///   character on a Mac keyboard rather than a dedicated key as it is on iOS.
/// - `ReplaceLast` is skipped because it exists for TPS auto-correct, and macOS
///   ships TL and POJ only.
/// - `CommitDerived` and `ResetContinuous` have no caller here:
///   `Reset` already covers aborting a continuous composition
///   (`transition.rs:554`).
/// - `Start` is absent because `Append` enters `Phase::Composing` from Idle by
///   itself (`transition.rs:54`), so a separate "begin" op would be a second way
///   to do the same thing — and one that skips the per-character preprocessing.
/// - `SelectSuggestion` is absent because it is not what it looks like. Under
///   `Phase::Continuous` it REPLACES the pending tail and re-prepends the nailed
///   prefix (`transition.rs:724`), so handing it the composition as rendered
///   double-counts that prefix: `台北` nailed plus a marked `台北大學` commits
///   `台北台北大學`. Selecting a candidate is `CommitContinuous` (span-local),
///   and Return is `CommitRaw` (the whole marked region) — between them nothing
///   is left for it to do.
/// - Candidate navigation lives entirely in the platform's own candidate
///   model (`.claude/rules/cross-platform-alignment.md` §4.1); the engine's
///   `SetSelectedCandidateIndex` op had no caller on any platform and was
///   removed 2026-09-25.
///
/// Every op answers `nil` when the round-trip itself failed, which is a
/// different thing from the engine answering that it is idle. A failed call
/// leaves the engine exactly as it was, so there is no snapshot to mirror — and
/// making that an optional rather than a stand-in value is what stops a caller
/// from mirroring "not composing" over a composition that is still running.
extension RustEngineBridge {
    // MARK: - Composing

    /// Appends one typed character to the raw buffer.
    ///
    /// Carries the continuous config, as every op that re-renders the
    /// composition does: under `Phase::Continuous` the answer is the whole
    /// marked region, nailed prefix included, and the prefix's word-boundary
    /// spacing reads the two flags only that config sets. With the base
    /// config a nail rendered `台gi` and the next keystroke `台 gi` (found by
    /// the composing-caret round, 2026-09-09).
    static func composingAppend(
        _ character: String,
        settings: EngineSettings,
        generation: UInt64,
    ) -> ComposingTransition? {
        var append = Taigi_Engine_Append()
        append.char = character
        return dispatchComposing(
            .append(append),
            op: "composingAppend",
            generation: generation,
            config: continuousAppConfig(settings),
        )
    }

    /// Applies one Telex key to the pending syllable's tone — or, for `z`,
    /// types the affricate initial the input mode spells (`composing.proto`
    /// `TelexKey`, `engine/composing/src/telex.rs`). Carries the same config
    /// as `composingAppend` (`z` resolves by `input_mode`, the prefix by the
    /// spacing flags).
    static func composingTelexKey(
        _ key: String,
        settings: EngineSettings,
        generation: UInt64,
    ) -> ComposingTransition? {
        var telexKey = Taigi_Engine_TelexKey()
        telexKey.key = key
        return dispatchComposing(
            .telexKey(telexKey),
            op: "composingTelexKey",
            generation: generation,
            config: continuousAppConfig(settings),
        )
    }

    /// Drops the character before the caret. Same config as `composingAppend`.
    static func composingDeleteBackward(
        settings: EngineSettings,
        generation: UInt64,
    ) -> ComposingTransition? {
        dispatchComposing(
            .deleteBackward(Taigi_Engine_DeleteBackward()),
            op: "composingDeleteBackward",
            generation: generation,
            config: continuousAppConfig(settings),
        )
    }

    /// Steps the caret one character inside the pending tail (`composing.proto`
    /// `MoveCaret`). The buffer is untouched, so the engine answers with an
    /// `UpdatePreedit` carrying the new caret and nothing else — no fetch is
    /// requested. Same config as `composingAppend`: the answer re-renders the
    /// composition the way the last keystroke did, so a move never changes
    /// the text on screen.
    static func composingMoveCaret(
        _ direction: CaretDirection,
        settings: EngineSettings,
        generation: UInt64,
    ) -> ComposingTransition? {
        var moveCaret = Taigi_Engine_MoveCaret()
        moveCaret.direction = switch direction {
        case .left: .left
        case .right: .right
        }
        return dispatchComposing(
            .moveCaret(moveCaret),
            op: "composingMoveCaret",
            generation: generation,
            config: continuousAppConfig(settings),
        )
    }

    /// Commits the whole composition exactly as the marked region renders it —
    /// `Σ nailed.display_text + derived(pending)` under the continuous phase
    /// (`transition.rs:443`), which is what the snapshot reports as
    /// `display_text` (`transition.rs:585`). This is the Return key.
    ///
    /// Not `composingSelectSuggestion`, despite what an earlier note in this
    /// file claimed: under `Phase::Continuous` that op prepends the nailed
    /// prefix to whatever text it is handed (`transition.rs:724`), so passing
    /// it the marked-region string double-counts — a composition reading
    /// `台北大學` with `台北` already nailed would commit `台北台北大學`.
    static func composingCommitRaw(
        settings: EngineSettings,
        generation: UInt64,
    ) -> ComposingTransition? {
        dispatchComposing(
            .commitRaw(Taigi_Engine_CommitRaw()),
            op: "composingCommitRaw",
            generation: generation,
            config: continuousAppConfig(settings),
        )
    }

    /// Finalizes the composition and appends `text` after it, as one engine
    /// step. Used for the space bar: committing and then inserting separately
    /// would be two host mutations for one keystroke.
    static func composingCommitPreeditThenInsertExternal(
        _ text: String,
        settings: EngineSettings,
        generation: UInt64,
    ) -> ComposingTransition? {
        var insert = Taigi_Engine_CommitPreeditThenInsertExternal()
        insert.text = text
        return dispatchComposing(
            .commitPreeditThenInsertExternal(insert),
            op: "composingCommitPreeditThenInsertExternal",
            generation: generation,
            config: continuousAppConfig(settings),
        )
    }

    /// Abandons the composition without writing anything to the document.
    static func composingReset(generation: UInt64) -> ComposingTransition? {
        dispatchComposing(
            .reset(Taigi_Engine_Reset()),
            op: "composingReset",
            generation: generation,
            config: nil,
        )
    }

    // MARK: - Continuous input

    /// Promotes an active composition into the continuous phase, where the
    /// engine segments the whole buffer instead of one syllable. Safe to send
    /// unconditionally: the engine no-ops on an empty buffer and on a
    /// composition that is already continuous (`transition.rs:496-502`), so the
    /// platform needs no eligibility rule of its own.
    static func composingEnterContinuous(
        settings: EngineSettings,
        generation: UInt64,
    ) -> ComposingTransition? {
        dispatchComposing(
            .enterContinuous(Taigi_Engine_EnterContinuous()),
            op: "composingEnterContinuous",
            generation: generation,
            // Same config as `composingAppend`: already under Continuous the
            // answer is a snapshot whose `displayText` the manager mirrors,
            // and a snapshot rendered with the base config would put the
            // space back.
            config: continuousAppConfig(settings),
        )
    }

    /// Reads the candidates for the current continuous composition.
    ///
    /// Read-only, so it must be sent under the composition's existing
    /// generation: a bumped generation resets the engine before the query runs
    /// (`engine/composing/src/handle.rs:61-66`).
    ///
    /// `frequencyRows` is what the user has committed before, which the engine
    /// turns into a per-candidate boost. Leaving it empty is not an error and
    /// not a degraded mode: the engine reads the proto3 zero values as "rank
    /// these without any usage history", which is the correct answer for a
    /// fresh install and for the first fetch of any composition, before the
    /// candidate keys to look up are even known.
    ///
    /// `enabledSourcesBitmask` is what the user's dictionary toggles resolve to
    /// (`lexiconDictionaryFilters`). `0` is not "no sources": the engine reads
    /// it as "platform did not wire this" and searches all of them
    /// (`composing.proto:176-183`), which is the degrade a failed resolve
    /// takes.
    ///
    /// `customEntries` is what the user's own dictionary matched for the
    /// current raw buffer. The columns go out exactly as stored — the engine
    /// dedupes `(roman, hanji)` against the FST hits and folds the roman to
    /// canonical TL itself when it learns from a commit, so massaging either
    /// here would break a key it owns.
    ///
    /// Both are computed once per keystroke by the caller and passed to both
    /// fetch phases, so the neutral and boosted answers describe one
    /// composition under one set of rules.
    ///
    /// `nowMs` is the clock the engine's recency ranking reads. It belongs with
    /// the rows rather than being read inside the engine, so that both fetches
    /// of one keystroke rank against a single instant.
    static func composingFetchAtPos(
        settings: EngineSettings,
        generation: UInt64,
        frequencyRows: [FrequencyRow] = [],
        nowMs: Int64 = 0,
        enabledSourcesBitmask: UInt32 = 0,
        customEntries: [CustomDictionaryRow] = [],
        // §50 — learned phrases whose whole-buffer key equals the raw buffer
        // (`LearnedPhraseStore.rows(matching:)`); competitors of the
        // dictionary rows, never the override `customEntries` are.
        learnedEntries: [LearnedPhraseRow] = [],
    ) -> ContinuousFetchResult? {
        var fetch = Taigi_Engine_FetchAtPos()
        // §34/S22 — positive platform setting → inverted proto disable gate
        // (the field's own comment carries why), so Show Typed Text First ON leaves the
        // preedit literal leading the list and Return commits what was typed.
        // CROSS-PLATFORM INVARIANT — mirrors desktop/crates/taigi-desktop-core/src/engine/composing.rs
        // `fetch_at_pos`, which inverts the same setting onto the same field.
        fetch.literalRomanCandidateDisabled = !settings.isLiteralRomanCandidateEnabled
        fetch.frequencyEntries = frequencyRows.map(frequencyEntry)
        fetch.nowMs = nowMs
        fetch.enabledSourcesBitmask = enabledSourcesBitmask
        fetch.customEntries = customEntries.map(customDictEntry)
        fetch.learnedEntries = learnedEntries.map(learnedEntry)

        guard let response = composingResponse(
            .fetchAtPos(fetch),
            op: "composingFetchAtPos",
            generation: generation,
            config: continuousAppConfig(settings),
        ) else {
            return nil
        }
        let candidates: [ContinuousCandidate]? = response.hasContinuous
            ? response.continuous.candidates.map(decodeCandidate)
            : nil
        return ContinuousFetchResult(
            transition: decodeTransition(response),
            candidates: candidates,
        )
    }

    /// Commits one candidate returned by `composingFetchAtPos`.
    ///
    /// Every argument except `documentText` must be round-tripped verbatim from
    /// the `ContinuousCandidate` the user picked — in particular `consumedBytes`
    /// is the candidate's `consumedSpanEnd`, an absolute offset into the pending
    /// raw buffer, NOT the span's length (`engine/protos/proto/composing.proto:228`).
    /// The engine collapses to a noop on a mismatched or unaligned offset
    /// (`transition.rs:812-817`), so there is nothing for the caller to validate.
    ///
    /// `documentText` is the platform's rendering of the candidate for the
    /// document; `canonicalText` and `associationTl` are the identity keys the
    /// engine learns from, which is why they are separate arguments rather than
    /// derived from the rendering (Core Principle #7 keys a word on the
    /// `(Hanji, canonical TL)` pair).
    ///
    /// Consuming the whole pending buffer makes this a final commit — the
    /// engine writes the composition to the document and exits to Idle. Anything
    /// less nails the segment and stays continuous, writing nothing
    /// (`transition.rs:842-889`, Model B).
    static func composingCommitContinuous(
        documentText: String,
        canonicalText: String,
        associationTl: String,
        // §50 — the picked candidate's hanji, `nil` for a hanji-less pick; the
        // engine learns a composition only when every segment carried one.
        hanji: String? = nil,
        consumedBytes: UInt32,
        syllableCount: UInt32,
        settings: EngineSettings,
        generation: UInt64,
    ) -> ComposingTransition? {
        var commit = Taigi_Engine_CommitContinuous()
        commit.displayText = documentText
        commit.canonicalText = canonicalText
        commit.associationTl = associationTl
        if let hanji, !hanji.isEmpty {
            commit.hanji = hanji
        }
        commit.consumedBytes = consumedBytes
        commit.syllableCount = syllableCount
        return dispatchComposing(
            .commitContinuous(commit),
            op: "composingCommitContinuous",
            generation: generation,
            config: continuousAppConfig(settings),
        )
    }

    // MARK: - Dispatch

    private static func dispatchComposing(
        _ method: Taigi_Engine_ComposingRequest.OneOf_Method,
        op: String,
        generation: UInt64,
        config: Taigi_Engine_AppConfig?,
    ) -> ComposingTransition? {
        guard let response = composingResponse(
            method,
            op: op,
            generation: generation,
            config: config,
        ) else {
            return nil
        }
        return decodeTransition(response)
    }

    private static func composingResponse(
        _ method: Taigi_Engine_ComposingRequest.OneOf_Method,
        op: String,
        generation: UInt64,
        config: Taigi_Engine_AppConfig?,
    ) -> Taigi_Engine_ComposingResponse? {
        var composing = Taigi_Engine_ComposingRequest()
        composing.method = method
        guard let payload = roundtrip(
            payload: .composing(composing),
            op: op,
            generation: generation,
            config: config,
        ) else {
            return nil
        }
        guard case let .composing(response) = payload else {
            recordFailure(op: op, message: "expected a composing payload, got \(payload)")
            return nil
        }
        return response
    }

    // MARK: - Decoding

    /// Not `private` so the decoding can be driven directly by tests: the two
    /// next-word effects that carry a payload are emitted only by intents the
    /// candidate window owns, and their field mapping would otherwise go
    /// unchecked until that slice lands and mislearns the user's associations.
    static func decodeTransition(
        _ response: Taigi_Engine_ComposingResponse,
    ) -> ComposingTransition {
        ComposingTransition(
            rawInput: response.preedit.rawInput,
            displayText: response.preedit.displayText,
            effects: response.effect.compactMap(decodeEffect),
            isComposing: response.isComposing,
        )
    }

    /// `nil` only for an effect whose `kind` the wire left unset, which the
    /// current engine never emits. Every kind it *does* emit has a case here —
    /// the exhaustive `switch` is what makes a newly added engine effect a
    /// compile error rather than a silently dropped instruction.
    static func decodeEffect(
        _ effect: Taigi_Engine_Effect,
    ) -> ComposingTransition.Effect? {
        guard let kind = effect.kind else { return nil }
        switch kind {
        case let .updatePreedit(payload):
            return .updatePreedit(payload.display, caretUTF16: Int(payload.caretUtf16))
        case .clearPreeditWithoutCommit_p:
            return .clearPreeditWithoutCommit
        case let .commitTextReplacingPreedit(payload):
            return .commitTextReplacingPreedit(payload.text)
        case .deleteBackwardFromDocument:
            return .deleteBackwardFromDocument
        case .resetAutocomplete:
            return .resetAutocomplete
        case .performAutocomplete:
            return .performAutocomplete
        case .resetAutocompleteContext:
            return .resetAutocompleteContext
        case let .nextWordUpdateLastSelectedWord(payload):
            return .nextWordUpdateLastSelectedWord(text: payload.text, roman: payload.roman)
        case let .nextWordWordSelected(payload):
            return .nextWordWordSelected(
                text: payload.text,
                roman: payload.roman,
                triggerPrediction: payload.triggerPrediction,
            )
        case .nextWordClearForNewComposing:
            return .nextWordClearForNewComposing
        case let .phraseLearned(payload):
            return .phraseLearned(hanji: payload.hanji, canonicalTl: payload.canonicalTl)
        }
    }

    /// One learned row on the wire. `count` is clamped rather than trusted to
    /// fit: the column is a 64-bit SQLite integer and the field is 32-bit, and
    /// a saturating conversion is a wrong boost where a trapping one is a
    /// crash in the middle of a keystroke.
    private static func frequencyEntry(_ row: FrequencyRow) -> Taigi_Engine_FrequencyEntry {
        var entry = Taigi_Engine_FrequencyEntry()
        entry.displayTextKey = row.word
        entry.canonicalTl = row.tl
        entry.count = UInt32(clamping: row.count)
        entry.lastUsedMs = row.lastUsedMillis
        return entry
    }

    /// One learned row (§50) on the wire — always a hanji, the engine only
    /// ever learns hanji picks.
    private static func learnedEntry(_ row: LearnedPhraseRow) -> Taigi_Engine_LearnedEntry {
        var entry = Taigi_Engine_LearnedEntry()
        entry.hanji = row.hanzi
        entry.canonicalTl = row.canonicalTl
        return entry
    }

    /// One custom-dictionary row on the wire.
    ///
    /// An empty stored hanji maps to an ABSENT `hanji` rather than an empty
    /// string: the field is proto3-optional, and the engine reads absence as
    /// "romanization-only entry" while an empty string would be a hanji that
    /// renders as nothing (`composing.proto:215-218`).
    private static func customDictEntry(_ row: CustomDictionaryRow) -> Taigi_Engine_CustomDictEntry {
        var entry = Taigi_Engine_CustomDictEntry()
        entry.roman = row.roman
        if !row.hanzi.isEmpty {
            entry.hanji = row.hanzi
        }
        return entry
    }

    private static func decodeCandidate(
        _ message: Taigi_Engine_CandidateMessage,
    ) -> ContinuousCandidate {
        ContinuousCandidate(
            consumedSpanStart: message.consumedSpanStart,
            consumedSpanEnd: message.consumedSpanEnd,
            syllableCount: message.syllableCount,
            displayText: message.displayText,
            score: message.score,
            form: message.form,
            mode: CandidateMode.decode(message.mode.rawValue),
            roman: message.roman,
            // Absent means "romanization-only candidate", which an empty string
            // would not distinguish from a present-but-blank hanji.
            hanji: message.hasHanji ? message.hanji : nil,
            canonicalTl: message.canonicalTl,
        )
    }
}
