package com.siansiansu.taigikeyboard.engine

import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.engine.proto.AppConfig
import com.siansiansu.taigikeyboard.engine.proto.CustomDictEntry
import com.siansiansu.taigikeyboard.engine.proto.FrequencyEntry
import com.siansiansu.taigikeyboard.engine.proto.Response
import com.siansiansu.taigikeyboard.ime.core.logging.LoggerBackend
import com.siansiansu.taigikeyboard.ime.core.logging.NullLoggerBackend
import com.siansiansu.taigikeyboard.ime.core.settings.CandidateDisplayMode
import com.siansiansu.taigikeyboard.ime.core.settings.InputMode
import com.siansiansu.taigikeyboard.ime.dictionary.FrequencyData
import com.siansiansu.taigikeyboard.ime.dictionary.FrequencyRow
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord
import java.util.ArrayDeque
import java.util.concurrent.atomic.AtomicInteger
import com.siansiansu.taigikeyboard.engine.proto.CandidateDisplayMode as ProtoCandidateDisplayMode

/**
 * Thin Kotlin wrapper around the Rust shared-core FFI exposed by
 * `engine/android-jni/src/lib.rs`.
 *
 * Facade only. Per-slice implementation lives in sibling impl objects in
 * the same package:
 * - [PhoneticsBridge] — phonetics core (8) + derivation (2) + TPS (5) + tone-variations cache
 * - [ComposingBridge] — composing slice (12) + continuous-input (4)
 * - [NextWordBridge] — NextWord slice (9)
 * - [LexiconBridge] — lexicon read path + ranking pipeline
 * - [CaseTransformBridge] — per-char/per-word case operations
 *
 * Each sibling re-uses [nextRequestIdInternal] for unique request IDs
 * and [recordFailure] for centralised diagnostics. JNI hop choice differs
 * by slice and is load-bearing — do NOT "clean up" into a uniform helper
 * without re-running the byte-for-byte parity audit:
 * - [PhoneticsBridge] / [ComposingBridge] / [NextWordBridge] +
 *   [LexiconBridge.rankingDispatch] use [sendRawBytes] (catches
 *   `Response.parseFrom` failure as `null`, lets `processRequestBytes`
 *   JNI exceptions propagate — mirrors pre-split facade dispatchers).
 * - Legacy [LexiconBridge.dispatch] + [CaseTransformBridge] use
 *   [dispatchRaw] (catches both JNI throw + parse failure via
 *   try/Throwable, emits `backend.w` only, no op-name `recordFailure`).
 *   Pre-existing; swap would lose `backend.w` log scope.
 *
 * Nested DTO types (e.g. [ComposingTransition], [NextWordDecideResult],
 * [ScoreBreakdown]) stay declared here so existing call-site import paths
 * (`RustEngineBridge.ComposingTransition`, etc.) keep working — facade
 * methods on this object delegate to the siblings.
 *
 * Per Codex v2 §7: `normalizeTone` requires `ToneToggles` mandatory
 * parameter — no `ToneToggles(true, true)` silent default.
 *
 * Per Codex v2 §8 + v3 §7 + v4 §5: error visibility is hardened. Failures
 * increment a counter and append a structured [DiagnosticsEntry] to a
 * 32-entry bounded queue (synchronized). DEBUG additionally surfaces the
 * failure via `installedBackend.e` for logcat traceability — NEVER throws
 * (would kill IME mid-keystroke).
 */
object RustEngineBridge {
    init {
        System.loadLibrary("rust_taigi")
    }

    @Volatile
    private var installedBackend: LoggerBackend = NullLoggerBackend

    /** Sibling-bridge accessor for `LexiconBridge` / `CaseTransformBridge` —
     *  same backend the JNI layer routes through. Read-only; mutation goes
     *  through [install]. */
    internal val backend: LoggerBackend
        get() = installedBackend

    @Volatile
    private var installed: Boolean = false

    private val installLock = Any()
    private val nextId = AtomicInteger(0)

    /**
     * Idempotent. Stash the backend then ask Rust to install the JNI bridge.
     *
     * In `BuildConfig.DEBUG` builds, also bumps Rust `log::max_level` to
     * `Debug` so dogfood traces are visible. Release stays at default `Warn`
     * so `log::debug!` / `log::info!` short-circuit before format — no JNI
     * cost for the no-op render path.
     */
    @JvmStatic
    fun install(backend: LoggerBackend) {
        synchronized(installLock) {
            installedBackend = backend
            if (!installed) {
                registerLogger()
                if (BuildConfig.DEBUG) {
                    setLogLevel(4) // 4 = Debug
                }
                installed = true
            }
        }
    }

    // region Phonetics facade — delegates to [PhoneticsBridge]

    /**
     * `Method::NormalizeTone` — input + AppConfig.input_mode + ToneToggles →
     * tone-marked string. `mode` and `toggles` are mandatory (no default)
     * to enforce the live-read invariant per Codex v2 §7.
     */
    fun normalizeTone(
        input: String,
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
    ): String = PhoneticsBridge.normalizeTone(input, mode, toggles)

    // Strips the syllable's tone combining mark; tone is "" when the syllable has none.
    fun stripTone(input: String): StripToneOutcome = PhoneticsBridge.stripTone(input)

    fun pojToTl(input: String): String = PhoneticsBridge.pojToTl(input)

    fun tlToPoj(input: String): String = PhoneticsBridge.tlToPoj(input)

    fun normalizeToTl(input: String): String = PhoneticsBridge.normalizeToTl(input)

    // Full NormalizeInput pipeline down to a trie-query key: TPS preprocess, lowercase, syllable split,
    // nasal / o͘ prep, checked-ending inference.
    fun normalizeInput(input: String): String = PhoneticsBridge.normalizeInput(input)

    /**
     * Replaces platform `TaigiUnicode.nfdPreprocessed(...)`. Lookup-side
     * NFD prep used by `ExternalLookupURLBuilder` before tone stripping.
     * Distinct semantics from [normalizeInput] — this preserves tone
     * diacritics; only nasal markers (ⁿ / ᴺ → "nn") and standalone
     * `\u{0358}` → `o` are rewritten.
     */
    fun nfdPreprocessForLookup(input: String): String = PhoneticsBridge.nfdPreprocessForLookup(input)

    // Backspace path: drops the last NFD tone mark and recomposes; null when there is no mark.
    fun restoreTone(text: String): String? = PhoneticsBridge.restoreTone(text)

    /** Lazy-init cache for `Method::GetToneVariations`. See [PhoneticsBridge.toneVariations]. */
    val toneVariations: ToneVariationsCache
        get() = PhoneticsBridge.toneVariations

    // Custom-dictionary search key: toneless form used for toneless prefix search.
    fun deriveNotone(roman: String): String = PhoneticsBridge.deriveNotone(roman)

    // Custom-dictionary search key: per-syllable initials (split on hyphen/space); "" for a single syllable.
    fun deriveAbbrev(roman: String): String = PhoneticsBridge.deriveAbbrev(roman)

    // Write side: expands roman into the {tl,poj,tps} x {num,notone,abbrev} key bundle.
    fun deriveCustomSearchKeys(roman: String): List<CustomSearchKey> = PhoneticsBridge.deriveCustomSearchKeys(roman)

    // Query side: one family key from input + mode. TPS collapses to "tl" through InputMode, so the engine
    // upgrades to the tps family via contains_tps(raw) instead.
    fun deriveCustomQueryKey(
        input: String,
        mode: com.siansiansu.taigikeyboard.ime.core.settings.InputMode,
    ): CustomSearchKey? = PhoneticsBridge.deriveCustomQueryKey(input, customSearchInputMode(mode))

    /**
     * Map the platform [com.siansiansu.taigikeyboard.ime.core.settings.InputMode]
     * to the engine `input_mode` string. Android's enum has no TPS case (`"tps"`
     * settings collapses to `TL` upstream via `InputMode.fromPrefString`); the
     * engine upgrades to the TPS family via `contains_tps` on the raw input.
     * Mirrors iOS `RustEngineBridge+Phonetics.swift` `customSearchInputMode`.
     */
    private fun customSearchInputMode(
        mode: com.siansiansu.taigikeyboard.ime.core.settings.InputMode,
    ): String =
        when (mode) {
            com.siansiansu.taigikeyboard.ime.core.settings.InputMode.POJ -> "poj"
            com.siansiansu.taigikeyboard.ime.core.settings.InputMode.ENGLISH -> "english"
            com.siansiansu.taigikeyboard.ime.core.settings.InputMode.TL -> "tl"
        }

    // Composing's derived display uses this to skip POJ/TL tone conversion.
    fun containsTps(text: String): Boolean = PhoneticsBridge.containsTps(text)

    // orMapsToER selects the er/or variant mapping.
    fun tlNumericToTps(
        text: String,
        orMapsToER: Boolean,
    ): String = PhoneticsBridge.tlNumericToTps(text, orMapsToER)

    fun tlDisplayToTps(
        text: String,
        orMapsToER: Boolean,
    ): String = PhoneticsBridge.tlDisplayToTps(text, orMapsToER)

    fun isTpsToneMark(char: Char): Boolean = PhoneticsBridge.isTpsToneMark(char)

    // Key-level TPS adjust: when replaceLast is non-empty the caller must replace the previous char with it.
    fun tpsInputAdjust(
        incoming: String,
        rawInput: String,
    ): TpsAdjustOutcome = PhoneticsBridge.tpsInputAdjust(incoming, rawInput)

    // endregion
    // region Lexicon ranking facade — delegates to [LexiconBridge]

    /**
     * Per-candidate score breakdown returned alongside the ranked list when
     * the caller opts in via `includeBreakdown = true`. Six fields sum to
     * the engine's sort key. Mirrors iOS `RustEngineBridge.ScoreBreakdown`
     * and proto `Taigi_Engine_ScoreBreakdown`.
     */
    data class ScoreBreakdown(
        val userFreqScore: Int,
        val recencyBonus: Int,
        val exactBonus: Int,
        val completionPenalty: Int,
        val closenessBonus: Int,
        val baseFreqScore: Int,
    ) {
        val total: Int
            get() = userFreqScore + recencyBonus + exactBonus + completionPenalty + closenessBonus + baseFreqScore
    }

    /**
     * Composite return for the lexicon ranking pipeline. Production
     * callers typically read [ranked]; tests inspect [breakdowns] to pin
     * the engine's six score components on the FFI boundary.
     */
    data class CandidateRanking(
        val ranked: List<TaigiWord>,
        val breakdowns: List<ScoreBreakdown>,
    )

    /** See [LexiconBridge.processCandidates]. */
    fun processCandidates(
        raw: List<TaigiWord>,
        normalizedInput: String,
        tpsDedupEnabled: Boolean,
        frequencyData: Map<String, FrequencyData>,
        nowMs: Long,
    ): List<TaigiWord> = LexiconBridge.processCandidates(raw, normalizedInput, tpsDedupEnabled, frequencyData, nowMs)

    /** See [LexiconBridge.processCandidatesDetailed]. */
    fun processCandidatesDetailed(
        raw: List<TaigiWord>,
        normalizedInput: String,
        tpsDedupEnabled: Boolean,
        frequencyData: Map<String, FrequencyData>,
        nowMs: Long,
        includeBreakdown: Boolean,
    ): CandidateRanking =
        LexiconBridge.processCandidatesDetailed(
            raw,
            normalizedInput,
            tpsDedupEnabled,
            frequencyData,
            nowMs,
            includeBreakdown,
        )

    /**
     * Marshal a `Map<String, FrequencyData>` snapshot into the proto
     * `FrequencyEntry` wire shape. Used by the legacy `ProcessCandidatesRequest`
     * ranking path (test-only on both platforms — no production caller).
     * `canonicalTl` defaults to "" (the engine's legacy tolerant-fallback
     * bucket), which is correct for that path's word-keyed map.
     */
    internal fun frequencyDataToProtoEntries(
        data: Map<String, FrequencyData>,
    ): List<FrequencyEntry> =
        data.map { (word, snapshot) ->
            FrequencyEntry
                .newBuilder()
                .setDisplayTextKey(word)
                .setCount(maxOf(0, snapshot.count))
                .setLastUsedMs(snapshot.lastUsedMillis)
                .build()
        }

    /**
     * R5 (#7): marshal `(word, tl)` pair-key rows into proto `FrequencyEntry`
     * carrying `canonicalTl`, so the engine builds a
     * `(display_text, canonical_tl)`-keyed `FrequencyMap`. Used by the
     * Continuous-input `FetchAtPos` fetch (the production user-frequency
     * path). Centralises the `count` clamp + field naming. Mirrors iOS
     * `ComposingManager.buildFrequencyEntries`.
     */
    internal fun frequencyRowsToProtoEntries(
        rows: List<FrequencyRow>,
    ): List<FrequencyEntry> =
        rows.map { row ->
            FrequencyEntry
                .newBuilder()
                .setDisplayTextKey(row.word)
                .setCanonicalTl(row.tl)
                .setCount(maxOf(0, row.data.count))
                .setLastUsedMs(row.data.lastUsedMillis)
                .build()
        }

    // endregion
    // region Composing facade — delegates to [ComposingBridge]

    /**
     * Bridge-synthesized companion to the proto `ComposingResponse`.
     * Consumed by `ComposingManager` and its delegate.
     */
    data class ComposingTransition(
        val rawInput: String,
        val displayText: String,
        val effects: List<Effect>,
        val selectedCandidateIndex: Int,
        val isComposing: Boolean,
    ) {
        sealed class Effect {
            data class UpdatePreedit(
                val display: String,
            ) : Effect()

            object ClearPreeditWithoutCommit : Effect()

            data class CommitTextReplacingPreedit(
                val text: String,
            ) : Effect()

            object DeleteBackwardFromDocument : Effect()

            object ResetAutocomplete : Effect()

            object PerformAutocomplete : Effect()

            object ResetAutocompleteContext : Effect()

            /**
             * v3.5.8 Phase 4 — continuous-input mid-commit handshake. Maps to
             * `NextWordRequest::UpdateLastSelectedWord(text, roman, now_ms)`.
             * Platform delegate forwards to `NextWordHandler.updateLastSelectedWord`
             * which injects `nowMs` + envelope generation.
             */
            data class NextWordUpdateLastSelectedWord(
                val text: String,
                val roman: String,
            ) : Effect()

            /**
             * v3.5.8 Phase 4 — continuous-input final-commit handshake. Maps to
             * `NextWordRequest::WordSelected(text, roman, require_roman_mode=false,
             * trigger_prediction, now_ms)`. Forward `triggerPrediction` exactly —
             * hardcoding either value breaks the Phase 4 commit contract.
             */
            data class NextWordWordSelected(
                val text: String,
                val roman: String,
                val triggerPrediction: Boolean,
            ) : Effect()

            /**
             * v3.5.8 Phase 4 — continuous-input abort handshake. Maps to
             * `NextWordRequest::ClearForNewComposing(now_ms)`. Platform delegate
             * forwards to `NextWordHandler.onClearCandidates()` (Android equivalent
             * of iOS `NextWordController.clearDisplay()`); NOT the structurally
             * distinct `ResetFull` intent.
             */
            object NextWordClearForNewComposing : Effect()
        }

        companion object {
            val NOOP = ComposingTransition(
                rawInput = "",
                displayText = "",
                effects = emptyList(),
                selectedCandidateIndex = -1,
                isComposing = false,
            )
        }
    }

    /**
     * MOE-aligned candidate-type discriminator. Wire mirror of
     * `protos::engine::CandidateMode` (Phase 9.2). Engine derives in Rust
     * from `DictionaryRecord.hanzi` presence + NFKD-normalized Latin-letter
     * detection (`engine/lexicon/src/continuous.rs::derive_mode`); the
     * platform reads but never recomputes (no display-text sniffing —
     * that would parallel-implement the derive and violate
     * `.claude/rules/cross-platform-alignment.md`).
     *
     * Metadata-only in v3.5.8 — does NOT enter the engine's `SortKey`
     * tie-break (per `docs/releases/v3.5.8/plan.md` § Phase 9 R2 Q3.a). `UNSPECIFIED`
     * is the proto3 default and means "unknown carrier — old engine or
     * dropped field"; never emitted by the current Rust engine.
     * Platforms must treat `UNSPECIFIED` as "ignore mode" rather than
     * falling back to any local classification.
     */
    enum class CandidateMode {
        UNSPECIFIED,
        HANT,
        TAILO,
        MIXED,
        ;

        companion object {
            /**
             * Decode the wire integer (`CandidateMessage.getModeValue()`)
             * produced by protobuf-javalite. Unrecognized values
             * (forward-compat from a newer engine) collapse to `UNSPECIFIED`
             * so the platform never crashes on a binding mismatch.
             */
            fun decode(wire: Int): CandidateMode =
                when (wire) {
                    1 -> HANT
                    2 -> TAILO
                    3 -> MIXED
                    else -> UNSPECIFIED
                }
        }
    }

    /**
     * Single span-local continuous-input candidate. Wire mirror of
     * `protos::engine::CandidateMessage` (Phase 6 + 9.2 `mode`).
     *
     * `consumedSpanStart` / `consumedSpanEnd` are byte offsets into the
     * **original raw user input** stored in `Phase::Continuous { raw }` —
     * TL/POJ users → ASCII bytes, TPS users → Bopomofo bytes. Platform UI
     * slices `pending` from `start` to `end` on commit. `form` is currently
     * always 1 (FORM_NOTONE). `mode` is the Phase 9.2 carrier;
     * metadata-only.
     */
    data class ContinuousCandidate(
        val consumedSpanStart: Int,
        val consumedSpanEnd: Int,
        val syllableCount: Int,
        val displayText: String,
        val score: Float,
        val form: Int,
        val mode: CandidateMode,
        /**
         * v3.5.8 Phase 9 Item 5 — display-romanization sidechannel for
         * dual-line cell render. Always non-empty for dictionary-
         * sourced candidates; the engine renders it for the active
         * input mode (TL, or POJ-display in POJ mode). UI reads
         * `displayText` for commit / `user_frequency.db` writes and
         * `roman` only for cell-title display.
         */
        val roman: String,
        /**
         * v3.5.8 Phase 9 Item 5 — hanji display sidechannel. `null`
         * iff the proto3 `optional string hanji` was absent on the
         * wire (TAILO candidate). Present-empty is treated as
         * present (engine never emits `Some("")` today; defensive).
         */
        val hanji: String?,
        /**
         * v3.6.1 R2 — canonical TL identity sidechannel
         * (`CandidateMessage.canonical_tl`). Unlike `roman` (the
         * POJ-rendered display form in POJ mode), this stays the canonical
         * TL the `(hanji, canonical-TL)` word identity is keyed on. The tap
         * path round-trips it into `commitContinuous(associationTl = …)` so
         * the NextWord association learns the same TL a normal candidate
         * commit records. Empty only for TPS-OOV hanji-absent candidates
         * with no dict TL.
         */
        val canonicalTl: String,
    )

    /**
     * Read-query result for `composingFetchAtPos`. The `candidates` tri-state
     * is only authoritative when `isBridgeFailure == false`:
     * - `null` → engine reached `handle_fetch_at_pos` but `Phase::Continuous`
     *   was not active (proto `continuous` field absent).
     * - `emptyList()` → continuous phase active but no candidates (no syllable
     *   inventory installed, no FST hits, or `position != 0`).
     * - non-empty → candidates returned in score-desc order.
     *
     * `transition` carries the engine snapshot (preedit / `selectedCandidateIndex`
     * / `isComposing`); FetchAtPos is read-only so its `effects` is empty.
     *
     * `isBridgeFailure` distinguishes "the engine returned Idle" (legit
     * generation-mismatch reset; `transition` reflects the new Idle state,
     * caller should `applyTransition` it) from "the FFI roundtrip itself
     * failed" (encode / decode / non-OK engine response; `transition ==
     * NOOP` is synthesized and applying it would clobber the cache mirror
     * with false state). Set `true` only on the `composingFetchDispatch`
     * early-return path via `ContinuousFetchResult.NOOP`; every successful
     * dispatch sets `false`. Phase 9.3c plumb relies on this to fall back
     * to phase-1 candidates on a transient phase-2 FFI failure rather than
     * dropping suggestions and resetting state. Mirrors iOS PR #265
     * r3216857164 — `ios/Sources/TaigiKeyboard/Engine/RustEngineBridge.swift`.
     */
    data class ContinuousFetchResult(
        val transition: ComposingTransition,
        val candidates: List<ContinuousCandidate>?,
        val isBridgeFailure: Boolean,
    ) {
        companion object {
            val NOOP = ContinuousFetchResult(
                transition = ComposingTransition.NOOP,
                candidates = null,
                isBridgeFailure = true,
            )
        }
    }

    /**
     * Multi-return for [ComposingManager.commitContinuous]. iOS uses a labeled
     * tuple `(didCommit: Bool, didFinalCommit: Bool)`; Kotlin's `Pair` loses the
     * label semantics so we surface a named data class instead.
     *
     * - `didCommit` = engine emitted at least one `CommitTextReplacingPreedit` Effect
     *   (real commit happened, mid OR final).
     * - `didFinalCommit` = `didCommit && !transition.isComposing` (engine returned
     *   to Idle this call, i.e. the last consumed-span emptied the buffer).
     *
     * Invariant: `didFinalCommit` implies `didCommit`. Stale-tap / generation-mismatch
     * → `(false, false)`. Used by [com.siansiansu.taigikeyboard.ime.text.smartbar
     * .CandidateClickHandler] to gate per-segment frequency learning + auto-space.
     */
    data class CommitContinuousResult(
        val didCommit: Boolean,
        val didFinalCommit: Boolean,
    ) {
        companion object {
            val NOOP = CommitContinuousResult(didCommit = false, didFinalCommit = false)
        }
    }

    @JvmStatic
    fun composingStart(
        text: String,
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
        generation: Long,
    ): ComposingTransition = ComposingBridge.composingStart(text, mode, toggles, generation)

    @JvmStatic
    fun composingAppend(
        ch: String,
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
        generation: Long,
    ): ComposingTransition = ComposingBridge.composingAppend(ch, mode, toggles, generation)

    // Separator hyphen distinguishes raw "tai-uan" from "taiuan", which changes the candidate trie key.
    @JvmStatic
    fun composingAppendHyphen(
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
        generation: Long,
    ): ComposingTransition = ComposingBridge.composingAppendHyphen(mode, toggles, generation)

    @JvmStatic
    fun composingReplaceLast(
        replacement: String,
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
        generation: Long,
    ): ComposingTransition = ComposingBridge.composingReplaceLast(replacement, mode, toggles, generation)

    // Engine owns the delete-to-empty → Idle transition and the 1-char delete path that must not eat document text.
    @JvmStatic
    fun composingDeleteBackward(
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
        generation: Long,
    ): ComposingTransition = ComposingBridge.composingDeleteBackward(mode, toggles, generation)

    // Commits the derived display string, e.g. raw "ho2" commits as "hó".
    @JvmStatic
    fun composingCommitDerived(
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
        generation: Long,
    ): ComposingTransition = ComposingBridge.composingCommitDerived(mode, toggles, generation)

    // Dispatched by phase: the Composing arm commits literal keystrokes ("ho2"), the Continuous arm commits
    // derived_display(pending) ("hó") — which is why the Continuous side needs mode + toggles.
    // v3.5.8 §10.2 platform pass: under `Phase::Continuous`, `CommitRaw`
    // routes to `commit_raw_continuous` which renders the whole
    // composition via `combined_display(nailed, raw, config)` — so the
    // continuous spacing flags ride here. Composing-arm `CommitRaw`
    // ignores them. Defaults = v3.5.7 roman-first so contract tests stay
    // behavior-identical; EVERY production Continuous call site MUST pass
    // explicit live values via `ComposingManager.continuousSpacingFlags`
    // (the sole production caller does — verified) or hanji-first
    // silently regresses.
    @JvmStatic
    fun composingCommitRaw(
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
        generation: Long,
        effectiveSwapped: Boolean = false,
        outputBothScripts: Boolean = false,
        candidateDisplayMode: CandidateDisplayMode = CandidateDisplayMode.SIDE_BY_SIDE,
    ): ComposingTransition =
        ComposingBridge.composingCommitRaw(
            mode,
            toggles,
            generation,
            effectiveSwapped,
            outputBothScripts,
            candidateDisplayMode,
        )

    // v3.5.8 §10.2 platform pass: under `Phase::Continuous`,
    // `SelectSuggestion` routes to `select_suggestion_under_continuous`
    // which prepends `nailed_prefix(nailed, config)` — so the continuous
    // spacing flags must ride here (previously `config = null` →
    // `AppConfig::default()` → spacing always ON → hanji-first spurious
    // spaces). The composing-arm `select_suggestion` ignores `config`
    // entirely (commits `text` verbatim), so this is a no-op there.
    // Defaults = v3.5.7 roman-first; the sole production caller
    // (`ComposingManager.selectSuggestion`) passes explicit live values.
    @JvmStatic
    fun composingSelectSuggestion(
        text: String,
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
        generation: Long,
        effectiveSwapped: Boolean = false,
        outputBothScripts: Boolean = false,
        candidateDisplayMode: CandidateDisplayMode = CandidateDisplayMode.SIDE_BY_SIDE,
    ): ComposingTransition =
        ComposingBridge.composingSelectSuggestion(
            text,
            mode,
            toggles,
            generation,
            effectiveSwapped,
            outputBothScripts,
            candidateDisplayMode,
        )

    // Commits the preedit and inserts the external string (space / Enter / punctuation) atomically, to avoid flicker.
    // v3.5.8 §10.2 platform pass: under `Phase::Continuous` (e.g. emoji
    // tap mid-continuous) this routes to
    // `commit_preedit_then_insert_external_under_continuous` which
    // renders the nailed prefix via `combined_display(nailed, raw,
    // config)` — so the continuous spacing flags ride here too. (Not in
    // the 2026-05-18 enumerated 4 ops, but the same class of Continuous
    // nailed-rendering path: excluding it would re-create the exact
    // hanji-first spurious-space regression the narrowed plumb
    // minimizes — see continuous-input-ranking.md §10.2.) Defaults =
    // v3.5.7 roman-first; production callers pass explicit live values.
    @JvmStatic
    fun composingCommitPreeditThenInsertExternal(
        text: String,
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
        generation: Long,
        effectiveSwapped: Boolean = false,
        outputBothScripts: Boolean = false,
        candidateDisplayMode: CandidateDisplayMode = CandidateDisplayMode.SIDE_BY_SIDE,
    ): ComposingTransition =
        ComposingBridge.composingCommitPreeditThenInsertExternal(
            text,
            mode,
            toggles,
            generation,
            effectiveSwapped,
            outputBothScripts,
            candidateDisplayMode,
        )

    @JvmStatic
    fun composingReset(generation: Long): ComposingTransition = ComposingBridge.composingReset(generation)

    // Reports the selected index so NextWord / Booster can read the context word. Does not commit.
    @JvmStatic
    fun composingSetSelectedCandidateIndex(
        index: Int,
        generation: Long,
    ): ComposingTransition = ComposingBridge.composingSetSelectedCandidateIndex(index, generation)

    @JvmStatic
    fun composingQueryState(generation: Long): ComposingTransition = ComposingBridge.composingQueryState(generation)

    /**
     * `Phase::Composing { raw }` → `Phase::Continuous { raw, committed: [] }`.
     * Phase 6 contract: no payload — buffer is whatever earlier `Start` /
     * `Append` populated. Engine no-ops on Idle / already-Continuous / empty
     * `Composing.raw`. AppConfig is required because the snapshot's preedit
     * display goes through `derived_display(raw, config)`.
     */
    @JvmStatic
    fun composingEnterContinuous(
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
        generation: Long,
    ): ComposingTransition = ComposingBridge.composingEnterContinuous(mode, toggles, generation)

    /**
     * Read-only candidate query for the current `Phase::Continuous { raw }`.
     * `position` is reserved as `0` in v3.5.8 (Phase 6 dispatch validates).
     * Caller MUST share the active composing-session generation — FetchAtPos
     * is read-only and bumping generation would reset engine state before
     * the fetch (`engine/composing/src/dispatch.rs:103-160`).
     *
     * `frequencyEntries` + `nowMs` are the v3.5.8 Phase 9.3a/9.3c plumb for
     * `user_freq_boost` + `SortKey.recency_rank`. Caller pre-filters entries
     * to candidate-relevant `displayTextKey`s (`hanji ?? roman`) — see
     * `engine/protos/proto/composing.proto:144-148`. Defaults `emptyList()`
     * + `0L` reproduce the PR-9.2 neutral-boost behaviour (`user_freq_boost
     * = 1.0`, `recency_rank = 1` everywhere); the platform plumb is
     * responsible for populating real values via a two-phase fetch
     * (`ComposingManager.fetchContinuousCandidates`). Mirrors iOS
     * `RustEngineBridge.composingFetchAtPos` PR-9.3b.
     *
     * v3.5.8 Phase 9 Item 12 — `customEntries` carries the platform's
     * `custom_dictionary.db` matches (raw stored `(roman, hanji)`
     * columns; DB stays native). Default `emptyList()` = no custom
     * matches / feature off — backward-compatible no-op. The engine
     * synthesizes a full-buffer candidate per entry and dedupes
     * `(roman, hanji)` against the FST hits (custom wins the
     * collision). Mirrors iOS `RustEngineBridge.composingFetchAtPos`.
     *
     * v3.5.8 §10.2 platform pass: the FetchAtPos snapshot renders the
     * combined marked region (`combined_display`) and per-segment recased
     * candidates, so it needs the continuous spacing flags to match the
     * commit-time rendering. Defaults = v3.5.7 roman-first; production
     * callers pass explicit live values via continuousSpacingFlags.
     */
    @JvmStatic
    fun composingFetchAtPos(
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
        generation: Long,
        frequencyEntries: List<FrequencyEntry> = emptyList(),
        nowMs: Long = 0L,
        customEntries: List<CustomDictEntry> = emptyList(),
        effectiveSwapped: Boolean = false,
        outputBothScripts: Boolean = false,
        // PR-9.6 — dictionary source-toggle bitmask (same one Tab3 browse
        // sends). Default `0u` = proto3-absent sentinel → engine all-on,
        // preserving pre-PR-9.6 behaviour for callers (incl. tests).
        enabledSourcesBitmask: UInt = 0u,
        // §34/S22 — invert of the 顯示當咧拍的字 setting. Default `false` = show
        // (proto3-absent sentinel → engine prepends the literal-roman
        // candidate, the pre-toggle always-on behaviour for callers/tests).
        literalRomanCandidateDisabled: Boolean = false,
        // 候選詞顯示 — ROMAN_ONLY makes the engine collapse same-roman rows.
        candidateDisplayMode: CandidateDisplayMode = CandidateDisplayMode.SIDE_BY_SIDE,
    ): ContinuousFetchResult =
        ComposingBridge.composingFetchAtPos(
            mode,
            toggles,
            generation,
            frequencyEntries,
            nowMs,
            customEntries,
            effectiveSwapped,
            outputBothScripts,
            enabledSourcesBitmask,
            literalRomanCandidateDisabled,
            candidateDisplayMode,
        )

    /**
     * Commit a candidate segment in `Phase::Continuous`. `displayText` /
     * `consumedBytes` / `syllableCount` MUST come from a [ContinuousCandidate]
     * returned by an immediately preceding [composingFetchAtPos] call —
     * sending mismatched values mis-aligns the committed segment.
     * `consumedBytes >= pending.utf8.size` triggers a final commit (exit
     * to Idle). Programmer-error inputs collapse to noop on the engine side.
     *
     * v3.5.8 §10.2 platform pass: the repro path. Mid-commit renders
     * `combined_display(nailed, pending, config)`; final-commit renders
     * `nailed_prefix(nailed, config)` — both need the spacing flags so
     * segments join with the right (roman: space / hanji-first: none /
     * both-scripts: space) word boundary. Defaults = v3.5.7 roman-first;
     * production callers pass explicit live values.
     */
    @JvmStatic
    fun composingCommitContinuous(
        displayText: String,
        canonicalText: String,
        associationTl: String,
        consumedBytes: Int,
        syllableCount: Int,
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
        generation: Long,
        effectiveSwapped: Boolean = false,
        outputBothScripts: Boolean = false,
        candidateDisplayMode: CandidateDisplayMode = CandidateDisplayMode.SIDE_BY_SIDE,
    ): ComposingTransition =
        ComposingBridge.composingCommitContinuous(
            displayText,
            canonicalText,
            associationTl,
            consumedBytes,
            syllableCount,
            mode,
            toggles,
            generation,
            effectiveSwapped,
            outputBothScripts,
            candidateDisplayMode,
        )

    /**
     * Abort continuous-input. Drops `Phase::Continuous` committed list +
     * pending raw, exits to Idle, emits the standard abort effect trio
     * (`ClearPreeditWithoutCommit` + `ResetAutocomplete` +
     * `NextWordClearForNewComposing`). Committed segments stay in the
     * document — earlier `CommitTextReplacingPreedit` effects already wrote
     * them.
     */
    @JvmStatic
    fun composingResetContinuous(generation: Long): ComposingTransition = ComposingBridge.composingResetContinuous(generation)

    // endregion
    // region NextWord facade — delegates to [NextWordBridge]

    /**
     * Bridge-synthesized companion to the proto `DecideResult`. Consumed
     * by the Android NextWord platform executor (`NextWordHandler`);
     * effect list executes in order.
     */
    data class NextWordDecideResult(
        val effects: List<Effect>,
        val currentGeneration: Long,
        val isShowing: Boolean,
        /**
         * `null` when the engine has no last-selected word; otherwise the
         * echo of `state.last_selected_word`. Empty wire string maps to
         * `null` per proto contract (`""` == `nil`).
         */
        val lastSelectedWord: String?,
    ) {
        sealed class Effect {
            data class RescheduleContextTimeout(
                val afterMs: Long,
            ) : Effect()

            object CancelContextTimeout : Effect()

            data class RecordAssociation(
                val pair: NextWordAssociationPair,
            ) : Effect()

            data class RecordCompoundAssociations(
                val pairs: List<NextWordAssociationPair>,
            ) : Effect()

            /**
             * `nowMs` is reused by the platform predict() call so the
             * association-window clock and the user-row decay scoring see
             * ONE consistent "now" per intent. Per
             * `nextword-engine-boundary.md` §13.3.
             */
            data class QueryPredictions(
                val word: String,
                val roman: String,
                val generation: Long,
                val nowMs: Long,
            ) : Effect()

            data class ClearPredictionsUI(
                val generation: Long,
            ) : Effect()
        }

        companion object {
            val NOOP = NextWordDecideResult(
                effects = emptyList(),
                currentGeneration = 0L,
                isShowing = false,
                lastSelectedWord = null,
            )
        }
    }

    /**
     * Bigram association pair surfaced through `RecordAssociation` /
     * `RecordCompoundAssociations` effects.
     */
    data class NextWordAssociationPair(
        val prev: String,
        val prevTl: String,
        val next: String,
        val nextTl: String,
    )

    /**
     * UI-ready prediction value. `subtitle` is `null` when the wire string
     * is empty (filter contract — happens iff roman is empty).
     */
    data class NextWordEnginePrediction(
        val text: String,
        val subtitle: String?,
        val hanzi: String,
        val tl: String,
        /**
         * Merged score. Android maps to `TaigiWord.lengthScore`. iOS does
         * not currently consume this field (predictions render in array
         * order); kept for parity + diagnostics.
         */
        val score: Double,
    )

    /**
     * Filter+merge+sort+limit result. `wasStale=true` indicates the
     * platform-supplied `queryGeneration` did not match the engine's
     * current generation — late async result; predictions are empty.
     */
    data class NextWordFilterResult(
        val predictions: List<NextWordEnginePrediction>,
        val wasStale: Boolean,
    )

    /**
     * Pre-merge un-scored row from the platform `NextWordService.predict`
     * SQL pipeline. Crosses the bridge to the Rust filter step.
     */
    data class NextWordRawRow(
        val hanzi: String,
        val tl: String,
        val count: Long,
        val lastUsedMs: Long,
        val source: Source,
    ) {
        enum class Source { DICT, USER }
    }

    /** Engine-state read for executor lookup. */
    data class NextWordStateSnapshot(
        val lastSelectedWord: String?,
        val isShowing: Boolean,
        val currentGeneration: Long,
    )

    // -- Decide intents (6) --
    // UpdateLastSelectedWord was originally Android-only (Space-path); v3.5.8
    // Phase 4 brought iOS into the call site through a continuous-input
    // mid-commit handshake. iOS bridge wraps it in
    // RustEngineBridge+NextWord.swift::nextwordUpdateLastSelectedWord.

    // Records the association, starts the context timer, and optionally queries the next-word prediction.
    @JvmStatic
    fun nextwordWordSelected(
        text: String,
        roman: String,
        requireRomanMode: Boolean,
        triggerPrediction: Boolean,
        nowMs: Long,
        mode: InputMode,
        translateSwapped: Boolean,
        associationRecordingEnabled: Boolean,
        generation: Long,
    ): NextWordDecideResult =
        NextWordBridge.wordSelected(
            text,
            roman,
            requireRomanMode,
            triggerPrediction,
            nowMs,
            mode,
            translateSwapped,
            associationRecordingEnabled,
            generation,
        )

    // Whether lastChar is a boundary character decides if the NextWord display clears and the timer reschedules.
    @JvmStatic
    fun nextwordBackspace(
        lastChar: String,
        nowMs: Long,
        mode: InputMode,
        translateSwapped: Boolean,
        associationRecordingEnabled: Boolean,
        generation: Long,
    ): NextWordDecideResult =
        NextWordBridge.backspace(
            lastChar,
            nowMs,
            mode,
            translateSwapped,
            associationRecordingEnabled,
            generation,
        )

    @JvmStatic
    fun nextwordContextTimeoutFired(
        nowMs: Long,
        mode: InputMode,
        translateSwapped: Boolean,
        associationRecordingEnabled: Boolean,
        generation: Long,
    ): NextWordDecideResult =
        NextWordBridge.contextTimeoutFired(
            nowMs,
            mode,
            translateSwapped,
            associationRecordingEnabled,
            generation,
        )

    // Clears the NextWord display but keeps lastSelectedWord for the next selection.
    @JvmStatic
    fun nextwordClearForNewComposing(
        nowMs: Long,
        mode: InputMode,
        translateSwapped: Boolean,
        associationRecordingEnabled: Boolean,
        generation: Long,
    ): NextWordDecideResult =
        NextWordBridge.clearForNewComposing(
            nowMs,
            mode,
            translateSwapped,
            associationRecordingEnabled,
            generation,
        )

    // Full reset of lastSelectedWord / lastSelectionTimeMs / isShowing (focus change, input-mode switch).
    @JvmStatic
    fun nextwordResetFull(
        nowMs: Long,
        mode: InputMode,
        translateSwapped: Boolean,
        associationRecordingEnabled: Boolean,
        generation: Long,
    ): NextWordDecideResult =
        NextWordBridge.resetFull(
            nowMs,
            mode,
            translateSwapped,
            associationRecordingEnabled,
            generation,
        )

    /**
     * Platform → engine UI visibility sync. Call after rendering an async
     * predict() result (or clearing it on empty result) so the engine's
     * `state.is_showing` stays accurate. Downstream
     * `nextwordClearForNewComposing` / sentence-end / context timeout /
     * `nextwordResetFull` paths gate `ClearPredictionsUI` emission on it.
     * No effects, no `current_generation` bump.
     */
    @JvmStatic
    fun nextwordSetIsShowing(
        isShowing: Boolean,
        mode: InputMode,
        translateSwapped: Boolean,
        associationRecordingEnabled: Boolean,
        generation: Long,
    ): NextWordDecideResult =
        NextWordBridge.setIsShowing(
            isShowing,
            mode,
            translateSwapped,
            associationRecordingEnabled,
            generation,
        )

    /**
     * Android-only Space-path intent. Codex v1 P1: preserves the
     * "compound-only / no timer reschedule / no generation bump"
     * semantics of the legacy `NextWordHandler.updateLastSelectedWord`.
     * The iOS bridge intentionally omits this intent.
     */
    @JvmStatic
    fun nextwordUpdateLastSelectedWord(
        text: String,
        roman: String,
        nowMs: Long,
        mode: InputMode,
        translateSwapped: Boolean,
        associationRecordingEnabled: Boolean,
        generation: Long,
    ): NextWordDecideResult =
        NextWordBridge.updateLastSelectedWord(
            text,
            roman,
            nowMs,
            mode,
            translateSwapped,
            associationRecordingEnabled,
            generation,
        )

    // -- Filter / Boost / QueryState --

    // Platform SQL supplies the raw dict + user prediction rows; Rust does score+merge+sort+limit.
    // A queryGeneration that no longer matches currentGeneration returns wasStale=true — caller drops the result.
    @JvmStatic
    fun nextwordFilter(
        raw: List<NextWordRawRow>,
        queryGeneration: Long,
        nowMs: Long,
        limit: Int,
        mode: InputMode,
        translateSwapped: Boolean,
        associationRecordingEnabled: Boolean,
        generation: Long,
        candidateDisplayMode: CandidateDisplayMode = CandidateDisplayMode.SIDE_BY_SIDE,
    ): NextWordFilterResult =
        NextWordBridge.filter(
            raw,
            queryGeneration,
            nowMs,
            limit,
            mode,
            translateSwapped,
            associationRecordingEnabled,
            generation,
            candidateDisplayMode,
        )

    // Autocomplete context booster: candidates whose first char is in the predicted set float up.
    @JvmStatic
    fun nextwordBoostCandidates(
        words: List<String>,
        predictedFirstChars: Set<String>,
        mode: InputMode,
        translateSwapped: Boolean,
        associationRecordingEnabled: Boolean,
        generation: Long,
    ): List<String> =
        NextWordBridge.boostCandidates(
            words,
            predictedFirstChars,
            mode,
            translateSwapped,
            associationRecordingEnabled,
            generation,
        )

    @JvmStatic
    fun nextwordQueryState(
        mode: InputMode,
        translateSwapped: Boolean,
        associationRecordingEnabled: Boolean,
        generation: Long,
    ): NextWordStateSnapshot = NextWordBridge.queryState(mode, translateSwapped, associationRecordingEnabled, generation)

    // endregion
    // region Diagnostics (Codex v2 §8 / v3 §7 / v4 §5)

    data class DiagnosticsEntry(
        val timestampMs: Long,
        val op: String,
        val errorCode: Int,
        val message: String,
    )

    /**
     * Read-only snapshot of in-memory failure tracking. For debug menu
     * + test inspection. Counter increments on every fallback path
     * (encode error, dispatch returned non-OK, missing result variant).
     * Recent entries capped at 32 to bound memory. NEVER throws.
     */
    @JvmStatic
    fun diagnostics(): DiagnosticsSnapshot {
        synchronized(diagnosticsLock) {
            return DiagnosticsSnapshot(
                failureCount = failureCounter.get(),
                recentErrors = recentErrors.toList(),
            )
        }
    }

    /** Test-only: clears counters so independent test cases don't bleed. */
    @JvmStatic
    fun resetDiagnosticsForTesting() {
        synchronized(diagnosticsLock) {
            failureCounter.set(0)
            recentErrors.clear()
        }
    }

    data class DiagnosticsSnapshot(
        val failureCount: Int,
        val recentErrors: List<DiagnosticsEntry>,
    )

    // endregion
    // region Test seam

    /** Sends arbitrary bytes for T4 / T5 / T7' tests. Returns null on parse failure. */
    fun sendRawBytes(bytes: ByteArray): Response? {
        val responseBytes = processRequestBytes(bytes)
        return runCatching { Response.parseFrom(responseBytes) }.getOrNull()
    }

    /** Drives T1. Throws UnsatisfiedLinkError on release `.so` (no panic-injector). */
    fun panicForTestRaw(): Response? {
        val responseBytes = panicForTest()
        return runCatching { Response.parseFrom(responseBytes) }.getOrNull()
    }

    // endregion
    // region JNI

    @JvmStatic
    private external fun processRequestBytes(bytes: ByteArray): ByteArray

    /**
     * Internal dispatch seam for sibling bridges that live outside this
     * object but share the same JNI plumbing. Same package only —
     * `internal` Kotlin visibility plus `engine` package. Wraps
     * `processRequestBytes` so the JNI symbol stays bound to
     * `RustEngineBridge`.
     */
    internal fun dispatchRaw(bytes: ByteArray): ByteArray = processRequestBytes(bytes)

    /**
     * Internal request-id allocator for sibling bridges. Increments the
     * shared atomic so request IDs are unique across all bridges in the
     * process.
     */
    internal fun nextRequestIdInternal(): Int = nextId.incrementAndGet()

    @JvmStatic
    private external fun registerLogger()

    @JvmStatic
    private external fun setLogLevel(level: Int)

    @JvmStatic
    private external fun panicForTest(): ByteArray

    /**
     * Called from native code via cached `JStaticMethodID`. MUST stay
     * `@JvmStatic` with signature `(I, Ljava/lang/String;,
     * Ljava/lang/String;)V` — see `engine/android-jni/src/lib.rs`.
     */
    @JvmStatic
    fun dispatchLog(
        level: Int,
        tag: String,
        msg: String,
    ) {
        val backend = installedBackend
        when (level) {
            LEVEL_ERROR -> backend.e(tag, msg)
            LEVEL_WARN -> backend.w(tag, msg)
            LEVEL_INFO -> backend.i(tag, msg)
            LEVEL_DEBUG -> backend.d(tag, msg)
            else -> backend.d(tag, msg)
        }
    }

    // endregion
    // region Diagnostics + AppConfig helpers (shared by sibling bridges)

    private val diagnosticsLock = Any()
    private val failureCounter = AtomicInteger(0)
    private val recentErrors = ArrayDeque<DiagnosticsEntry>(RECENT_ERRORS_CAP)

    /**
     * Records an FFI failure into the bounded diagnostics queue + emits
     * a warn-level log. `internal` so sibling impl objects route their
     * own failures through the same counter.
     */
    internal fun recordFailure(
        op: String,
        message: String,
        code: Int = -1,
    ) {
        synchronized(diagnosticsLock) {
            failureCounter.incrementAndGet()
            val entry = DiagnosticsEntry(
                timestampMs = System.currentTimeMillis(),
                op = op,
                errorCode = code,
                message = message,
            )
            if (recentErrors.size >= RECENT_ERRORS_CAP) {
                recentErrors.removeFirst()
            }
            recentErrors.addLast(entry)
            installedBackend.w("RustEngineBridge", "[$op] $message")
            // Codex v3 §7 / v4 §5: also surface as error severity for logcat
            // traceability. The facade is itself DEBUG-gated (release builds
            // emit zero output per `security-rules.md` zero-logs contract),
            // so no outer `if (DEBUG)` guard is required. No throw — would
            // kill the IME mid-keystroke.
            installedBackend.e("RustEngineBridge", "[$op] $message")
        }
    }

    /**
     * Phonetics / composing base [AppConfig] — input mode + POJ doubletap
     * toggles. `internal` so sibling impl objects share one canonical
     * factory (no per-slice drift).
     */
    internal fun appConfig(
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
    ): AppConfig =
        AppConfig
            .newBuilder()
            .setInputMode(
                when (mode) {
                    NormalizeMode.POJ -> "poj"
                    NormalizeMode.TL -> "tl"
                    NormalizeMode.ENGLISH -> "english"
                },
            ).setOoDoubletapEnabled(toggles.isDoubleTapOoEnabled)
            .setNnDoubletapEnabled(toggles.isDoubleTapNnEnabled)
            .build()

    /**
     * Continuous-rendering [AppConfig]: base [appConfig] plus the two
     * v3.5.8 §10.2 word-boundary-spacing flags the engine's
     * `continuous_word_space` predicate consumes.
     *
     * `effectiveSwapped` (= translate-swap OR TPS layout, combined
     * platform-side because [NormalizeMode] has no TPS and TPS resolves
     * to `"tl"`/`"poj"` `input_mode`, so the engine's own
     * `input_mode == "tps"` branch never fires) rides
     * `is_translate_swapped`. `outputBothScripts` distinguishes
     * hanji-first (no inter-segment space) from both-scripts
     * (`hit (彼)` — space wanted); `is_translate_swapped` is `true` for
     * both, so the second flag is required.
     *
     * Applied ONLY at the Continuous-phase entry points that render the
     * nailed prefix — `commit_continuous`, `commit_raw_continuous`,
     * `select_suggestion_under_continuous`,
     * `commit_preedit_then_insert_external_under_continuous`, and the
     * FetchAtPos snapshot — so the hanji-first regression surface stays
     * minimal (continuous-input-ranking.md §10.2; platform pass decided
     * 2026-05-18). All other composing methods keep the flag-free base
     * [appConfig].
     *
     * CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Engine/RustEngineBridge.swift continuousAppConfig.
     * Drift causes silent divergence (hanji-first spurious word-boundary spaces).
     */
    internal fun continuousAppConfig(
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
        effectiveSwapped: Boolean,
        outputBothScripts: Boolean,
        candidateDisplayMode: CandidateDisplayMode,
    ): AppConfig =
        appConfig(mode, toggles)
            .toBuilder()
            .setIsTranslateSwapped(effectiveSwapped)
            .setOutputBothScripts(outputBothScripts)
            .setCandidateDisplayMode(candidateDisplayMode.toProto())
            .build()

    private const val LEVEL_ERROR = 0
    private const val LEVEL_WARN = 1
    private const val LEVEL_INFO = 2
    private const val LEVEL_DEBUG = 3
    private const val RECENT_ERRORS_CAP = 32

    // endregion
}

// =========================================================================
// Public DTOs (Kotlin doesn't allow named-tuple returns; using data classes)
// =========================================================================

/** `Method::NormalizeTone` mode parameter. Mirrors iOS `InputMode` minus `.tps` */
enum class NormalizeMode { POJ, TL, ENGLISH }

/**
 * Carrier for the two POJ doubletap preprocessing toggles. Caller (e.g.
 * ComposingManager) MUST construct this from live settings per Codex v2 §7
 * — no default value at the wrapper level.
 */
data class ToneTogglesCarrier(
    val isDoubleTapOoEnabled: Boolean,
    val isDoubleTapNnEnabled: Boolean,
)

/**
 * `AppConfig.candidate_display_mode` (field 9). The engine collapses
 * same-roman candidate rows itself under ROMAN_ONLY (`handle_fetch_at_pos`
 * + nextword `filter`); every other request family ignores the field.
 */
internal fun CandidateDisplayMode.toProto(): ProtoCandidateDisplayMode =
    when (this) {
        CandidateDisplayMode.SIDE_BY_SIDE -> ProtoCandidateDisplayMode.CANDIDATE_DISPLAY_MODE_SIDE_BY_SIDE
        CandidateDisplayMode.ROMAN_ONLY -> ProtoCandidateDisplayMode.CANDIDATE_DISPLAY_MODE_ROMAN_ONLY
        CandidateDisplayMode.COMBINED -> ProtoCandidateDisplayMode.CANDIDATE_DISPLAY_MODE_COMBINED
    }

/** Result of `Method::StripTone`. */
data class StripToneOutcome(
    val bare: String,
    val tone: String,
)

/**
 * One custom-dictionary cross-mode search key (v3.6.1 R3). Mirrors the proto
 * `CustomSearchKey` and iOS `RustEngineBridge+Phonetics.swift` `CustomSearchKey`:
 * `family` ∈ {tl, poj, tps}, `form` ∈ {num, notone, abbrev}, `key` the fused
 * family-native search string. Written to the `custom_search_key` side table;
 * the query op returns one to match against it.
 */
data class CustomSearchKey(
    val family: String,
    val form: String,
    val key: String,
)

/** Result of `Method::TpsInputAdjust`. */
data class TpsAdjustOutcome(
    val adjusted: String,
    val replaceLast: String?,
)

/** Init-bulk-pull cache for the callout tone variation tables. */
data class ToneVariationsCache(
    val poj: Map<String, List<String>>,
    val tl: Map<String, List<String>>,
)
