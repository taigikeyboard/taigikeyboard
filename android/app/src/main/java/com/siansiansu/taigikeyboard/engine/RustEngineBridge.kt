package com.siansiansu.taigikeyboard.engine

import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.engine.proto.CommitContinuous
import com.siansiansu.taigikeyboard.engine.proto.CommitOutcome
import com.siansiansu.taigikeyboard.engine.proto.CommitResolution
import com.siansiansu.taigikeyboard.engine.proto.CommitScript
import com.siansiansu.taigikeyboard.engine.proto.ErrorCode
import com.siansiansu.taigikeyboard.engine.proto.Request
import com.siansiansu.taigikeyboard.engine.proto.Response
import com.siansiansu.taigikeyboard.ime.core.logging.LoggerBackend
import com.siansiansu.taigikeyboard.ime.core.logging.NullLoggerBackend
import com.siansiansu.taigikeyboard.ime.dictionary.DictionarySource
import com.siansiansu.taigikeyboard.ime.settings.EngineSettings
import java.util.ArrayDeque
import java.util.concurrent.atomic.AtomicInteger

/**
 * Thin Kotlin wrapper around the Rust shared-core FFI exposed by
 * `engine/android-jni/src/lib.rs`.
 *
 * This object owns the JNI seam, [dispatch], the logger / diagnostics
 * state and every nested DTO type (`AppConfig` is built in `EngineAppConfig.kt`). The per-slice
 * ops are extension functions on it, one file per slice (mirrors iOS
 * `RustEngineBridge+<Slice>.swift`):
 * - `PhoneticsBridge.kt` — phonetics core + TPS
 * - `ComposingBridge.kt` — composing slice (10) + continuous-input (4)
 * - `NextWordBridge.kt` — NextWord slice (7 decide intents + filter)
 * - `LexiconBridge.kt` — lexicon read path
 * - `CaseTransformBridge.kt` — per-char/per-word case operations
 * - `UserDataBridge.kt` — the engine-owned user data (open, picks, dictionary pages, backup)
 * Callers outside this package import each extension by name
 * (`import com.siansiansu.taigikeyboard.engine.tlToPoj`).
 *
 * Every slice sends through [dispatch] — one request-id allocator, one
 * JNI hop, one `try/Throwable` boundary (a JNI throw or a parse failure
 * never escapes into the IME keystroke path), one [recordFailure] sink.
 * Slices keep only their own payload check.
 *
 * Per Codex v2 §7: every op that needs `PojMarkerOptions` takes it as a
 * mandatory parameter — no `PojMarkerOptions(true, true, true)` silent default.
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

    // region Lexicon DTOs — ops are extensions in LexiconBridge.kt

    /** Bridge-synthesized companion to proto `TaigiWord` (lexicon search row). */
    data class LexiconRow(
        val id: Long,
        val roman: String,
        val hanji: String?,
        /** The dictionaries the record belongs to, in the engine's (source-bit) order — the badge order. */
        val sources: List<DictionarySource>,
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
     * `engine/protos/proto/lexicon.proto::DictionarySourceToggles` (12 source toggles
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
         * `kautian_subcollections` proto message is always present and the engine
         * always runs the subcollection gate. Field order mirrors config.yaml
         * `dialect_columns` / proto `KautianSubcollectionToggles`.
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
     * Output of `dictionaryFilters` — ready-to-send bitmask plus the
     * decoded enabled-source set for dictionary-search retag. Replaces verbatim
     * platform `EnabledDictionaries` bit math (deleted in v3.5.8 slice).
     */
    data class DictionaryFilters(
        val dictionaryFilterBitmask: UInt,
        val enabledSources: Set<DictionarySource>,
    ) {
        companion object {
            /**
             * What a failed resolve degrades to: every source on. `UInt.MAX_VALUE`
             * is the engine's "filter disabled" sentinel on the search path
             * (`dictionary_reader.rs::Filter::from_enabled_bitmask`). Fail-open on
             * purpose — a wider candidate list is recoverable, an empty one looks
             * like a broken keyboard. Mirrors iOS
             * `RustEngineBridge.DictionaryFilters.allSourcesEnabled`.
             */
            val ALL_SOURCES_ENABLED = DictionaryFilters(
                dictionaryFilterBitmask = UInt.MAX_VALUE,
                enabledSources = DictionarySource.entries.toSet(),
            )
        }
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

            object ClearCandidates : Effect()

            object RefreshCandidates : Effect()

            object ResetCandidateContext : Effect()

            /**
             * v3.5.8 Phase 4 — continuous-input mid-commit handshake. Maps to
             * `NextWordRequest::UpdateLastSelectedWord(text, roman, now_ms)`.
             * Platform delegate forwards to `NextWordController.updateLastSelectedWord`
             * which injects `nowMs` + envelope generation.
             */
            data class NextWordUpdateLastSelectedWord(
                val text: String,
                val roman: String,
            ) : Effect()

            /**
             * v3.5.8 Phase 4 — continuous-input final-commit handshake. Maps to
             * `NextWordRequest::WordSelected(text, roman, require_roman_mode=false,
             * trigger_prediction, now_ms, preceding)`. Forward `triggerPrediction`
             * and `preceding` exactly — `preceding` is the nailed segments committed
             * before `text`, learned as one sequence (behavioral-invariants §40).
             */
            data class NextWordWordSelected(
                val text: String,
                val roman: String,
                val triggerPrediction: Boolean,
                val preceding: List<com.siansiansu.taigikeyboard.engine.proto.CommittedWord>,
            ) : Effect()

            /**
             * v3.5.8 Phase 4 — continuous-input abort handshake. Maps to
             * `NextWordRequest::ClearForNewComposing(now_ms)`. Platform delegate
             * forwards to `NextWordController.onClearCandidates()` (Android equivalent
             * of iOS `NextWordController.clearDisplay()`); NOT the structurally
             * distinct `ResetAll` intent.
             */
            object NextWordClearForNewComposing : Effect()
        }

        companion object {
            val NOOP = ComposingTransition(
                rawInput = "",
                displayText = "",
                effects = emptyList(),
                isComposing = false,
            )
        }
    }

    /**
     * MOE-aligned candidate-type discriminator. Wire mirror of
     * `protos::engine::CandidateScriptKind` (Phase 9.2). Engine derives in Rust
     * from `DictionaryRecord.hanji` presence + NFKD-normalized Latin-letter
     * detection (`engine/lexicon/src/continuous/mod.rs::derive_script_kind`); the
     * platform reads but never recomputes (no display-text sniffing —
     * that would parallel-implement the derive and violate
     * `docs/contributing/cross-platform-alignment.md`).
     *
     * Metadata-only in v3.5.8 — does NOT enter the engine's `CandidateSortKey`
     * tie-break (per `docs/releases/v3.5.8/plan.md` § Phase 9 R2 Q3.a). `UNSPECIFIED`
     * is the proto3 default and means "unknown carrier — old engine or
     * dropped field"; never emitted by the current Rust engine.
     * Platforms must treat `UNSPECIFIED` as "ignore it" rather than
     * falling back to any local classification.
     */
    enum class CandidateScriptKind {
        UNSPECIFIED,
        HANT,
        TAILO,
        MIXED,
        ;

        companion object {
            /**
             * Decode the wire integer (`CandidateMessage.getScriptKindValue()`)
             * produced by protobuf-javalite. Unrecognized values
             * (forward-compat from a newer engine) collapse to `UNSPECIFIED`
             * so the platform never crashes on a binding mismatch.
             */
            fun decode(wire: Int): CandidateScriptKind =
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
     * `protos::engine::CandidateMessage` (Phase 6 + 9.2 `script_kind`).
     *
     * `consumedSpanStart` / `consumedSpanEnd` are byte offsets into the
     * **original raw user input** stored in `Phase::Continuous { raw }` —
     * TL/POJ users → ASCII bytes, TPS users → Bopomofo bytes. Platform UI
     * slices `pending` from `start` to `end` on commit. `form` is currently
     * always 1 (FORM_NOTONE). `scriptKind` is the Phase 9.2 carrier;
     * metadata-only.
     */
    data class ContinuousCandidate(
        val consumedSpanStart: Int,
        val consumedSpanEnd: Int,
        val syllableCount: Int,
        val displayText: String,
        val score: Float,
        val form: Int,
        val scriptKind: CandidateScriptKind,
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
     * One continuous-candidate pick (R5): which of its scripts the document
     * gets, and the candidate metadata the engine resolves the text from
     * (`engine/composing/src/commit_text.rs`). Every field but [script]
     * round-trips verbatim from the [ContinuousCandidate] the user picked;
     * [consumedBytes] is its `consumedSpanEnd`, an absolute offset into the
     * pending raw buffer. Mirrors iOS `RustEngineBridge.ContinuousPick`.
     */
    data class ContinuousPick(
        val script: CommitScript,
        /** The candidate's display romanization ([ContinuousCandidate.roman]). */
        val roman: String,
        /**
         * Identity keys the engine counts the pick under — the canonical
         * `hanji ?: roman` and TL (Core Principle #6), never the rendering.
         */
        val canonicalText: String,
        val associationTl: String,
        /** §50 — the pick's hanji, null for a hanji-less candidate. */
        val hanji: String?,
        val consumedBytes: Int,
        val syllableCount: Int,
    ) {
        /** The wire request. */
        fun toRequest(): CommitContinuous {
            // A local, not the property: inside `apply` the builder's own
            // `hanji` would shadow it.
            val hanji = hanji
            return CommitContinuous
                .newBuilder()
                .setScript(script)
                .setRoman(roman)
                .setCanonicalText(canonicalText)
                // R2: canonical TL → NextWord next_tl/prev_tl. Empty → engine
                // falls back to the raw committed slice.
                .setAssociationTl(associationTl)
                .apply { if (!hanji.isNullOrEmpty()) setHanji(hanji) }
                .setConsumedBytes(consumedBytes)
                .setSyllableCount(syllableCount)
                .build()
        }
    }

    /**
     * What a continuous pick did, as the engine answered it
     * (`ComposingResponse.commit`) — never read off the composing mirror: a
     * generation mismatch resets the engine to Idle before the intent runs
     * (`engine/composing/src/handle.rs`), so a mirror read would report a
     * commit that never happened.
     */
    sealed interface ContinuousCommitOutcome {
        /** Nothing changed: a stale generation, a rejected pick, or a failed round-trip. */
        data object Ignored : ContinuousCommitOutcome

        /** The segment was nailed and the composition continues (Model B: no document write). */
        data object Nailed : ContinuousCommitOutcome

        /**
         * The whole composition was written and the engine is Idle.
         * [earnsAutoSpace] is the engine's §23 verdict on what the pick wrote;
         * the live Auto-Space setting is the caller's.
         */
        data class Finalized(
            val earnsAutoSpace: Boolean,
        ) : ContinuousCommitOutcome

        companion object {
            // CROSS-PLATFORM INVARIANT — mirrors iOS `RustEngineBridge.ContinuousCommitOutcome.init(_:)`.
            fun from(resolution: CommitResolution): ContinuousCommitOutcome =
                when (resolution.outcome) {
                    CommitOutcome.COMMIT_OUTCOME_NAILED -> Nailed
                    CommitOutcome.COMMIT_OUTCOME_FINALIZED -> Finalized(resolution.earnsAutoSpace)
                    // UNSPECIFIED: an answer without a resolution changed
                    // nothing this side can tell.
                    else -> Ignored
                }
        }
    }

    /** Result of [composingCommitContinuous]: the transition to replay and what the pick did. */
    data class ContinuousCommitResult(
        val transition: ComposingTransition,
        val outcome: ContinuousCommitOutcome,
    ) {
        companion object {
            val FAILED = ContinuousCommitResult(ComposingTransition.NOOP, ContinuousCommitOutcome.Ignored)
        }
    }

    // endregion
    // region NextWord DTOs — ops are extensions in NextWordBridge.kt

    /**
     * Bridge-synthesized companion to the proto `DecideResult`. Consumed
     * by the Android NextWord platform executor (`NextWordController`);
     * effect list executes in order.
     */
    data class NextWordDecideResult(
        val effects: List<Effect>,
        val currentGeneration: Long,
        val predictionsVisible: Boolean,
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
                predictionsVisible = false,
                lastSelectedWord = null,
            )
        }
    }

    /**
     * UI-ready prediction value. `subtitle` is `null` when the wire string
     * is empty (filter contract — happens iff roman is empty).
     */
    data class NextWordEnginePrediction(
        val text: String,
        val subtitle: String?,
        val hanji: String,
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
    // region Diagnostics (shared by sibling bridges)

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

/** Result of `Method::TpsInputAdjust`. */
data class TpsAdjustOutcome(
    val adjusted: String,
    val replaceLast: String?,
)
