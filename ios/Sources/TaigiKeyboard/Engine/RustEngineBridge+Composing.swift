import Foundation
import SwiftProtobuf

// MARK: - RustEngineBridge Composing surface

/// Composing slice extension for `RustEngineBridge`. Holds the composing
/// and continuous-input ops (v3.5.8 Phase 6) + their synthesized value
/// types (`ComposingTransition` / `ContinuousCandidate` /
/// `ContinuousPick` / `ContinuousCommitResult` / `ContinuousFetchResult`) +
/// the composing-specific dispatch helpers
/// (`composingProtoRoundtrip` / `composingDispatch` / `composingFetchDispatch`
/// / `synthComposing` / `continuousAppConfig`). The four dispatch helpers
/// stay `private` to the composing surface; `continuousAppConfig` is
/// internal so the tests can pin the wire config.
public extension RustEngineBridge {
    // MARK: - Synthesized value types

    /// Bridge-synthesized companion to the proto `ComposingResponse`.
    /// Consumed by `ComposingManager` and its delegate.
    struct ComposingTransition: Equatable {
        public enum Effect: Equatable {
            case updatePreedit(String)
            case clearPreeditWithoutCommit
            case commitTextReplacingPreedit(String)
            case clearCandidates
            case refreshCandidates
            case resetCandidateContext
            /// v3.5.8 Phase 4 — continuous-input mid-commit handshake. Maps to
            /// `NextWordRequest::UpdateLastSelectedWord(text, roman, now_ms)`.
            /// Platform delegate forwards to `NextWordController.updateLastSelectedWord`
            /// which injects `nowMs` + envelope generation. NextWord learns nothing
            /// from it (§40); `ComposingManager` reads it as the nail signal.
            case nextWordUpdateLastSelectedWord(text: String, roman: String)
            /// v3.5.8 Phase 4 — continuous-input final-commit handshake. Maps to
            /// `NextWordRequest::WordSelected(text, roman, require_roman_mode=false,
            /// trigger_prediction, now_ms, preceding)`. Forward `triggerPrediction`
            /// and `preceding` exactly — `preceding` is the nailed segments committed
            /// before `text`, learned as one sequence (behavioral-invariants §40).
            case nextWordWordSelected(
                text: String,
                roman: String,
                triggerPrediction: Bool,
                preceding: [Taigi_Engine_CommittedWord],
            )
            /// v3.5.8 Phase 4 — continuous-input abort handshake. Maps to
            /// `NextWordRequest::ClearForNewComposing(now_ms)`. Platform delegate
            /// forwards to `NextWordController.clearDisplay()` (NOT
            /// `resetAndClearUI()` — that sends the structurally distinct
            /// `ResetAll` intent).
            case nextWordClearForNewComposing
        }

        public let rawInput: String
        public let displayText: String
        public let effects: [Effect]
        public let isComposing: Bool

        public static let noop = ComposingTransition(
            rawInput: "",
            displayText: "",
            effects: [],
            isComposing: false,
        )
    }

    /// Single span-local continuous-input candidate. Wire mirror of
    /// `protos::engine::CandidateMessage` (Phase 6).
    ///
    /// `consumedSpanEnd` is a byte offset into the **original raw user
    /// input** stored in `Phase::Continuous { raw }` — TL/POJ users → ASCII
    /// bytes, TPS users → Bopomofo bytes. The pick sends it back as
    /// `consumedBytes` so the engine knows how much of the pending buffer
    /// the commit consumes.
    struct ContinuousCandidate: Equatable {
        public let consumedSpanEnd: UInt32
        public let syllableCount: UInt32
        public let displayText: String
        /// v3.5.8 Phase 9 Item 5 — display-romanization sidechannel for
        /// dual-line cell render. Always non-empty for dictionary-
        /// sourced candidates; the engine renders it for the active
        /// input mode (TL, or POJ-display in POJ mode). UI reads
        /// `displayText` for commit / `user_frequency.db` writes and
        /// `roman` only for cell-title display.
        public let roman: String
        /// v3.5.8 Phase 9 Item 5 — hanji display sidechannel. `nil`
        /// iff the proto3 `optional string hanji` was absent on the
        /// wire (roman-only candidate). Present-empty is treated as
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
            consumedSpanEnd: UInt32,
            syllableCount: UInt32,
            displayText: String,
            roman: String,
            hanji: String?,
            canonicalTl: String,
        ) {
            self.consumedSpanEnd = consumedSpanEnd
            self.syllableCount = syllableCount
            self.displayText = displayText
            self.roman = roman
            self.hanji = hanji
            self.canonicalTl = canonicalTl
        }
    }

    /// One continuous-candidate pick (R5): which of its scripts the document
    /// gets, and the candidate metadata the engine resolves the text from
    /// (`engine/composing/src/commit_text.rs`) — never the view-rewritten
    /// suggestion. Every field but `script` round-trips verbatim from the
    /// `ContinuousCandidate` the user picked; `consumedBytes` is its
    /// `consumedSpanEnd`, an absolute offset into the pending raw buffer.
    struct ContinuousPick: Equatable {
        public let script: Taigi_Engine_CommitScript
        /// The candidate's display romanization (`ContinuousCandidate.roman`).
        public let roman: String
        /// Identity keys the engine counts the pick under — the canonical
        /// `hanji ?? roman` and TL (Core Principle #6), never the rendering.
        public let canonicalText: String
        public let associationTl: String
        /// §50 — the pick's hanji, `nil` for a hanji-less candidate.
        public let hanji: String?
        public let consumedBytes: UInt32
        public let syllableCount: UInt32

        /// The wire request.
        var request: Taigi_Engine_CommitContinuous {
            var request = Taigi_Engine_CommitContinuous()
            request.script = script
            request.roman = roman
            request.canonicalText = canonicalText
            // R2: canonical TL of the chosen candidate → NextWord `next_tl` /
            // `prev_tl`. Empty → engine falls back to the raw committed slice.
            request.associationTl = associationTl
            if let hanji, !hanji.isEmpty {
                request.hanji = hanji
            }
            request.consumedBytes = consumedBytes
            request.syllableCount = syllableCount
            return request
        }
    }

    /// What a continuous pick did, as the engine answered it
    /// (`ComposingResponse.commit`) — never read off the composing mirror: a
    /// generation mismatch resets the engine to Idle before the intent runs
    /// (`engine/composing/src/handle.rs`), so a mirror read would report a
    /// commit that never happened.
    enum ContinuousCommitOutcome: Equatable {
        /// Nothing changed: a stale generation, a rejected pick, or a failed
        /// round-trip.
        case ignored
        /// The segment was nailed and the composition continues (Model B: no
        /// document write).
        case nailed
        /// The whole composition was written and the engine is Idle.
        /// `earnsAutoSpace` is the engine's §23 verdict on what the pick wrote;
        /// the live Auto-Space setting is the caller's.
        case finalized(earnsAutoSpace: Bool)

        // CROSS-PLATFORM INVARIANT — mirrors desktop-core `CandidateCommitOutcome::from_resolution`
        // and android `RustEngineBridge.ContinuousCommitOutcome.from`.
        init(_ resolution: Taigi_Engine_CommitResolution) {
            switch resolution.outcome {
            case .nailed: self = .nailed
            case .finalized: self = .finalized(earnsAutoSpace: resolution.earnsAutoSpace)
            // UNSPECIFIED: an answer without a resolution changed nothing this
            // side can tell.
            case .ignored, .unspecified, .UNRECOGNIZED: self = .ignored
            }
        }
    }

    /// Result of `composingCommitContinuous`: the transition to replay and what
    /// the pick did.
    struct ContinuousCommitResult: Equatable {
        public let transition: ComposingTransition
        public let outcome: ContinuousCommitOutcome

        static let failed = ContinuousCommitResult(transition: .noop, outcome: .ignored)
    }

    /// Read-query result for `composingFetchAtPos`. The `candidates` tri-state
    /// is only authoritative when `isBridgeFailure == false`:
    /// - `nil` → engine reached `handle_fetch_at_pos` but `Phase::Continuous`
    ///   was not active (proto `continuous` field absent).
    /// - `[]` → continuous phase active but no candidates (no syllable
    ///   inventory installed, no FST hits, or a hanji-contaminated buffer).
    /// - non-empty → candidates returned in score-desc order.
    ///
    /// `transition` carries the engine snapshot (preedit / `isComposing`);
    /// FetchAtPos is read-only so its `effects` is empty.
    ///
    /// `isBridgeFailure` distinguishes "the engine returned Idle" (legit
    /// generation-mismatch reset; `transition` reflects the new Idle state,
    /// caller should `apply()` it) from "the FFI roundtrip itself failed"
    /// (encode / decode / non-OK engine response; `transition == .noop`
    /// is synthesized and `apply()`-ing it would clobber the mirror with
    /// false state). Set `true` only on the `composingFetchDispatch` early-
    /// return path via `ContinuousFetchResult.noop`; every successful
    /// dispatch sets `false`. `ComposingManager.fetchContinuousCandidates`
    /// relies on this to leave the mirror untouched on a failed round-trip.
    /// Codex PR #265
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

    // MARK: Composing slice (10 ops)

    internal static func composingStart(
        _ text: String,
        settings: EngineSettings,
        generation: UInt64,
    ) -> ComposingTransition {
        var payload = Taigi_Engine_Start()
        payload.text = text
        return composingDispatch(
            method: .start(payload),
            op: "composingStart",
            generation: generation,
            config: continuousAppConfig(settings),
        )
    }

    internal static func composingAppend(
        _ char: String,
        settings: EngineSettings,
        generation: UInt64,
    ) -> ComposingTransition {
        var payload = Taigi_Engine_Append()
        payload.char = char
        return composingDispatch(
            method: .append(payload),
            op: "composingAppend",
            generation: generation,
            config: continuousAppConfig(settings),
        )
    }

    internal static func composingAppendHyphen(
        settings: EngineSettings,
        generation: UInt64,
    ) -> ComposingTransition {
        composingDispatch(
            method: .appendHyphen(Taigi_Engine_AppendHyphen()),
            op: "composingAppendHyphen",
            generation: generation,
            config: continuousAppConfig(settings),
        )
    }

    internal static func composingReplaceLast(
        _ replacement: String,
        settings: EngineSettings,
        generation: UInt64,
    ) -> ComposingTransition {
        var payload = Taigi_Engine_ReplaceLast()
        payload.replacement = replacement
        return composingDispatch(
            method: .replaceLast(payload),
            op: "composingReplaceLast",
            generation: generation,
            config: continuousAppConfig(settings),
        )
    }

    internal static func composingDeleteBackward(
        settings: EngineSettings,
        generation: UInt64,
    ) -> ComposingTransition {
        composingDispatch(
            method: .deleteBackward(Taigi_Engine_DeleteBackward()),
            op: "composingDeleteBackward",
            generation: generation,
            config: continuousAppConfig(settings),
        )
    }

    // The engine commits the whole composition
    // (`combined_display(nailed, pending, config)`), not the literal keystrokes.
    internal static func composingCommitRaw(
        settings: EngineSettings,
        generation: UInt64,
    ) -> ComposingTransition {
        composingDispatch(
            method: .commitRaw(Taigi_Engine_CommitRaw()),
            op: "composingCommitRaw",
            generation: generation,
            config: continuousAppConfig(settings),
        )
    }

    // E.g. an emoji tap mid-composition: the engine commits the rendered
    // composition first, then inserts `text`.
    internal static func composingCommitPreeditThenInsertExternal(
        _ text: String,
        settings: EngineSettings,
        generation: UInt64,
    ) -> ComposingTransition {
        var payload = Taigi_Engine_CommitPreeditThenInsertExternal()
        payload.text = text
        return composingDispatch(
            method: .commitPreeditThenInsertExternal(payload),
            op: "composingCommitPreeditThenInsertExternal",
            generation: generation,
            config: continuousAppConfig(settings),
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

    // MARK: Continuous-input (2 ops) — v3.5.8 Phase 6

    /// Read-only candidate query for the current `Phase::Continuous { raw }`.
    /// Caller MUST share the active composing-session generation — FetchAtPos
    /// is read-only and bumping generation would reset engine state before
    /// the fetch (`engine/composing/src/requests.rs::query`).
    ///
    /// The user's own data is not among the arguments: the engine reads its
    /// stores itself and ranks in the same call
    /// (`docs/architecture/user-data-engine-roadmap.md` P7b). `nowMs` is the
    /// clock its recency ranking reads. Mirrors desktop-core
    /// `engine::composing::fetch_at_pos`.
    internal static func composingFetchAtPos(
        settings: EngineSettings,
        generation: UInt64,
        nowMs: Int64,
        // §34/S22 — invert of the Show Typed Text First setting. Default `false` = show
        // (proto3-absent sentinel → engine prepends the literal-roman
        // candidate, the pre-toggle always-on behaviour for callers/tests).
        literalRomanCandidateDisabled: Bool = false,
        // Invert of the Enable Custom Dictionary setting: the engine reads the
        // user's dictionary only with it on. Default `false` = read it.
        customDictionaryDisabled: Bool = false,
    ) -> ContinuousFetchResult {
        var payload = Taigi_Engine_FetchAtPos()
        payload.nowMs = nowMs
        // The dictionary toggles the Tab3 browse path resolves too; the engine
        // turns them into its source filter (every dictionary off offers no
        // dictionary candidates, §57).
        payload.toggles = dictionaryTogglesProto(DictionaryToggles(from: settings))
        payload.literalRomanCandidateDisabled = literalRomanCandidateDisabled
        payload.customDictionaryDisabled = customDictionaryDisabled
        return composingFetchDispatch(
            method: .fetchAtPos(payload),
            op: "composingFetchAtPos",
            generation: generation,
            config: continuousAppConfig(settings),
        )
    }

    /// Commit a candidate segment in `Phase::Continuous`. The pick MUST come
    /// from a `ContinuousCandidate` returned by an immediately preceding
    /// `composingFetchAtPos` call — mismatched values mis-align the committed
    /// segment. `consumedBytes >= pending.utf8.count` makes a final commit
    /// (exit to Idle); anything less nails the segment, writing nothing
    /// (Model B). The engine resolves the document text from the pick's
    /// scripts under `settings` and — with the user data open — counts the
    /// pick itself (R5). Mirrors desktop-core
    /// `engine::composing::commit_continuous`.
    internal static func composingCommitContinuous(
        _ pick: ContinuousPick,
        settings: EngineSettings,
        generation: UInt64,
    ) -> ContinuousCommitResult {
        guard let payload = composingProtoRoundtrip(
            method: .commitContinuous(pick.request),
            op: "composingCommitContinuous",
            generation: generation,
            config: continuousAppConfig(settings),
        ) else {
            return .failed
        }
        // Always set on this path; an absent one reads as UNSPECIFIED → ignored.
        return ContinuousCommitResult(
            transition: synthComposing(payload),
            outcome: ContinuousCommitOutcome(payload.commit),
        )
    }

    // MARK: - Private dispatch (composing envelope)

    /// The composing `AppConfig`: the live settings through `appConfig`,
    /// including the two v3.5.8 §10.2 word-boundary-spacing flags the
    /// engine's `continuous_word_space` predicate consumes.
    ///
    /// The swap is the Candidate-Display-projected setting; a TPS layout is
    /// sent as `"tps"` and the engine reads it as Hanji-first itself
    /// (`AppConfig::renders_hanji_first`). `outputBothScripts`
    /// distinguishes hanji-first (no inter-segment space) from
    /// both-scripts (`hit (彼)` — space wanted); `is_hanji_first`
    /// is `true` for both, so the second flag is required.
    ///
    /// Every composing op that renders the composition sends it — under
    /// Model B that is every mutation and every snapshot, not only the
    /// commits: `Append` / `DeleteBackward` after a nail re-render the
    /// nailed prefix through `combined_display(nailed, raw, config)` too,
    /// so a nail and the keystroke after it must agree on the prefix
    /// (the 2026-05-18 "commit entry points only" split left Hanji-first
    /// showing `台 gi` while typing after `台`; desktop closed the same
    /// drift in #31, S37). Only `Reset`, which carries no config, stays
    /// outside.
    // `candidateDisplayMode` (proto field 9) travels with the pair: under Romanization Only the callers already
    // pass the DERIVED `(false, false)` pair, and FetchAtPos uses the mode to collapse same-roman rows.
    // CROSS-PLATFORM INVARIANT — mirrors android/app/src/main/java/com/siansiansu/taigikeyboard/engine/EngineAppConfig.kt continuousAppConfig.
    // Drift causes silent divergence (hanji-first spurious word-boundary spaces).
    internal static func continuousAppConfig(_ settings: EngineSettings) -> Taigi_Engine_AppConfig {
        appConfig(
            mode: settings.inputMode,
            pojMarkers: settings.pojMarkerOptions,
            isHanjiFirst: settings.isHanjiFirst,
            isOutputBothScripts: settings.isOutputBothScripts,
            candidateDisplayMode: settings.candidateDisplayMode,
            syllableSeparator: settings.syllableSeparator,
            isTpsOrMappedToER: settings.isTpsOrMappedToER,
        )
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
        if let config {
            request.configSnapshot = config
        }

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
                // distinction faithfully (roman-only candidate). `roman` is
                // never empty: dictionary rows carry it, a custom entry cannot
                // be saved without one, the literal is the typed text.
                ContinuousCandidate(
                    consumedSpanEnd: msg.consumedSpanEnd,
                    syllableCount: msg.syllableCount,
                    displayText: msg.displayText,
                    roman: msg.roman,
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
            case .clearCandidates_p: return .clearCandidates
            case .refreshCandidates: return .refreshCandidates
            case .resetCandidateContext: return .resetCandidateContext
            case let .nextWordUpdateLastSelectedWord(m):
                return .nextWordUpdateLastSelectedWord(text: m.text, roman: m.roman)
            case let .nextWordWordSelected(m):
                return .nextWordWordSelected(
                    text: m.text,
                    roman: m.roman,
                    triggerPrediction: m.triggerPrediction,
                    preceding: m.preceding,
                )
            case .nextWordClearForNewComposing:
                return .nextWordClearForNewComposing
            }
        }
        return ComposingTransition(
            rawInput: proto.preedit.rawInput,
            displayText: proto.preedit.displayText,
            effects: effects,
            isComposing: proto.isComposing,
        )
    }
}
