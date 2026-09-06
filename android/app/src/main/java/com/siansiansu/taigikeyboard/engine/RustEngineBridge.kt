package com.siansiansu.taigikeyboard.engine

import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.engine.proto.AppConfig
import com.siansiansu.taigikeyboard.engine.proto.ErrorCode
import com.siansiansu.taigikeyboard.engine.proto.FrequencyEntry
import com.siansiansu.taigikeyboard.engine.proto.Request
import com.siansiansu.taigikeyboard.engine.proto.Response
import com.siansiansu.taigikeyboard.ime.core.logging.LoggerBackend
import com.siansiansu.taigikeyboard.ime.core.logging.NullLoggerBackend
import com.siansiansu.taigikeyboard.ime.core.settings.CandidateDisplayMode
import com.siansiansu.taigikeyboard.ime.core.settings.EngineSettings
import com.siansiansu.taigikeyboard.ime.dictionary.DictionarySource
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
 * This object owns the JNI seam, [dispatch], the logger / diagnostics
 * state, the [AppConfig] factories and every nested DTO type. The per-slice
 * ops are extension functions on it, one file per slice (mirrors iOS
 * `RustEngineBridge+<Slice>.swift`):
 * - `PhoneticsBridge.kt` — phonetics core (8) + derivation (2) + TPS (5) + tone-variations cache
 * - `ComposingBridge.kt` — composing slice (12) + continuous-input (4)
 * - `NextWordBridge.kt` — NextWord slice (9)
 * - `LexiconBridge.kt` — lexicon read path + ranking pipeline
 * - `CaseTransformBridge.kt` — per-char/per-word case operations
 * Callers outside this package import each extension by name
 * (`import com.siansiansu.taigikeyboard.engine.normalizeTone`).
 *
 * Every slice sends through [dispatch] — one request-id allocator, one
 * JNI hop, one `try/Throwable` boundary (a JNI throw or a parse failure
 * never escapes into the IME keystroke path), one [recordFailure] sink.
 * Slices keep only their own payload check.
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

    /** Sibling-bridge accessor for slice-level debug logging — same backend
     *  the JNI layer routes through. Read-only; mutation goes through
     *  [install]. */
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

    // region Phonetics — ops are extensions in PhoneticsBridge.kt

    /** Lazy-init cache for `Method::GetToneVariations`. See [PhoneticsBridge.toneVariations]. */
    val toneVariations: ToneVariationsCache
        get() = PhoneticsBridge.toneVariations

    // endregion
    // region Lexicon DTOs — ops are extensions in LexiconBridge.kt

    /** Bridge-synthesized companion to proto `TaigiWord` (lexicon search row). */
    data class LexiconRow(
        val id: Long,
        val roman: String,
        val hanzi: String?,
        val lengthScore: Int?,
        val sourceBitmask: UInt?,
    )

    /** Bridge-synthesized companion to proto `LexiconAssocEntry`. */
    data class LexiconAssocEntry(
        val previousWord: String,
        val candidateWord: String,
        val candidateTl: String,
        val count: UInt,
    )

    /** Engine install diagnostic counts (for dogfood logging). */
    data class LexiconInstallStats(
        val dictionaryRecordCount: ULong,
        val prefixIndexEntryCount: ULong,
    )

    /** Lexicon engine `inputMode` enum (mirrors proto `InputMode`). */
    enum class LexiconInputMode(
        val protoValue: Int,
    ) {
        UNSPECIFIED(0),
        TL(1),
        POJ(2),
        TPS(3),
    }

    /**
     * Snapshot of the user's dictionary preference state. Field order mirrors
     * `engine/protos/proto/lexicon.proto::DictionaryToggles` (12 source toggles
     * + nested [KautianSubcoll]). Build via `from(settings)`; never construct
     * piecemeal at search call sites — that splits the snapshot.
     */
    data class DictionaryToggles(
        val kautian: Boolean,
        val taigitv: Boolean,
        val itaigi: Boolean,
        val sitbut: Boolean,
        val taihoa: Boolean,
        val taijit: Boolean,
        val kungge: Boolean,
        val stti: Boolean,
        val khpoo: Boolean,
        val variant: Boolean,
        val khiin: Boolean,
        val lkk: Boolean,
        val dev: Boolean,
        val kautianSubcoll: KautianSubcoll,
    ) {
        /**
         * kautian subcollection enable state (10 accents + name appendix).
         * Android always populates this (the app ships the toggles), so the
         * `kautian_subcoll` proto message is always present and the engine
         * always runs the subcollection gate. Field order mirrors config.yaml
         * `dialect_columns` / proto `KautianSubcollToggles`.
         * Mirrors iOS `RustEngineBridge.DictionaryToggles.KautianSubcoll`.
         */
        data class KautianSubcoll(
            val lukang: Boolean,
            val sansia: Boolean,
            val taipak: Boolean,
            val gilan: Boolean,
            val tainan: Boolean,
            val kaohsiung: Boolean,
            val kinmen: Boolean,
            val makung: Boolean,
            val sintik: Boolean,
            val taichung: Boolean,
            val nameAppendix: Boolean,
        )

        companion object {
            fun from(settings: EngineSettings): DictionaryToggles =
                DictionaryToggles(
                    kautian = settings.isMoeDictEnabled,
                    taigitv = settings.isNewwordDictEnabled,
                    itaigi = settings.isITaigiDictEnabled,
                    sitbut = settings.isTaiwanPlantDictEnabled,
                    taihoa = settings.isTaiHuaDictEnabled,
                    taijit = settings.isTaiwanJapanDictEnabled,
                    kungge = settings.isKunggeDictEnabled,
                    stti = settings.isSttiDictEnabled,
                    khpoo = settings.isKhpooDictEnabled,
                    variant = settings.isVariantEnabled,
                    khiin = settings.isKhiinEnabled,
                    lkk = settings.isLkkDictEnabled,
                    dev = settings.isDevDictEnabled,
                    kautianSubcoll = KautianSubcoll(
                        lukang = settings.isKautianAccentLukangEnabled,
                        sansia = settings.isKautianAccentSansiaEnabled,
                        taipak = settings.isKautianAccentTaipakEnabled,
                        gilan = settings.isKautianAccentGilanEnabled,
                        tainan = settings.isKautianAccentTainanEnabled,
                        kaohsiung = settings.isKautianAccentKaohsiungEnabled,
                        kinmen = settings.isKautianAccentKinmenEnabled,
                        makung = settings.isKautianAccentMakungEnabled,
                        sintik = settings.isKautianAccentSintikEnabled,
                        taichung = settings.isKautianAccentTaichungEnabled,
                        nameAppendix = settings.isKautianNameAppendixEnabled,
                    ),
                )
        }
    }

    /**
     * Output of `dictionaryFilters` — ready-to-send bitmasks plus the
     * decoded enabled-source set for Dictionary tab retag. Replaces verbatim
     * platform `EnabledDictionaries` bit math (deleted in v3.5.8 slice).
     *
     * `assocLookupBitmask` carries the `UInt.MAX_VALUE` sentinel when all 9
     * association sources are on — preserves the documented
     * `lexicon.proto:166-173` shortcut. Caller forwards directly to
     * `assocLookup(enabledSourcesBitmask = ...)`.
     */
    data class DictionaryFilters(
        val dictionaryFilterBitmask: UInt,
        val assocLookupBitmask: UInt,
        val enabledSources: Set<DictionarySource>,
    )

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
    // region Case-transform DTO — ops are extensions in CaseTransformBridge.kt

    /**
     * Three-state shift / case indicator. Bridge-side mirror of the proto
     * `LetterCase` enum + the iOS `CaseTransformLetterCase`. Adapts
     * Android's existing `(caps: Boolean, capsLock: Boolean)` pair at the
     * call site (CapsLock=true → CapsLocked; caps=true → Uppercased;
     * else Lowercased) — see `from()` factory.
     */
    enum class LetterCase(
        val protoValue: Int,
    ) {
        LOWERCASED(1),
        UPPERCASED(2),
        CAPS_LOCKED(3),
        ;

        companion object {
            /** Adapter from Android's existing caps + capsLock boolean pair. */
            @JvmStatic
            fun from(
                caps: Boolean,
                capsLock: Boolean,
            ): LetterCase =
                when {
                    capsLock -> CAPS_LOCKED
                    caps -> UPPERCASED
                    else -> LOWERCASED
                }
        }
    }

    // endregion
    // region Composing DTOs — ops are extensions in ComposingBridge.kt

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

    // endregion
    // region NextWord DTOs — ops are extensions in NextWordBridge.kt

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
     * Single request → JNI → parse → error-check hop for every sibling
     * bridge. `build` fills the slice payload (and any generation /
     * config snapshot) on a [Request.Builder] whose id is already
     * allocated from the process-wide counter. Both the JNI call and
     * `Response.parseFrom` catch `Exception` so an engine failure degrades
     * to "no candidates" instead of taking the keystroke path down; every
     * failure mode goes through [recordFailure] under `op`. `Error` is
     * deliberately NOT caught — `OutOfMemoryError` and the `LinkageError`
     * family mean the JVM is already unrecoverable, and swallowing them
     * would surface the real fault somewhere later and harder to read.
     * (`System.loadLibrary` runs in this object's initializer, so a
     * missing library fails at first access, never here.) Returns `null`
     * on any failure, else the envelope — callers check their own slice
     * payload.
     */
    internal fun dispatch(
        op: String,
        build: Request.Builder.() -> Unit,
    ): Response? {
        val request = Request
            .newBuilder()
            .setId(nextId.incrementAndGet())
            .apply(build)
            .build()
        val responseBytes = try {
            processRequestBytes(request.toByteArray())
        } catch (t: Exception) {
            recordFailure(op, "dispatch failed: $t")
            return null
        }
        val response = try {
            Response.parseFrom(responseBytes)
        } catch (t: Exception) {
            recordFailure(op, "response decode failed")
            return null
        }
        if (response.error != ErrorCode.OK) {
            recordFailure(op, "engine returned ${response.error}", response.error.number)
            return null
        }
        return response
    }

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
