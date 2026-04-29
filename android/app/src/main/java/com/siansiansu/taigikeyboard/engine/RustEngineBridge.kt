package com.siansiansu.taigikeyboard.engine

import android.util.Log
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.engine.proto.AppConfig
import com.siansiansu.taigikeyboard.engine.proto.BoolResult
import com.siansiansu.taigikeyboard.engine.proto.ContainsTps
import com.siansiansu.taigikeyboard.engine.proto.DeriveAbbrev
import com.siansiansu.taigikeyboard.engine.proto.DeriveNotone
import com.siansiansu.taigikeyboard.engine.proto.ErrorCode
import com.siansiansu.taigikeyboard.engine.proto.FrequencyEntry
import com.siansiansu.taigikeyboard.engine.proto.GetToneVariations
import com.siansiansu.taigikeyboard.engine.proto.NfdPreprocessForLookup
import com.siansiansu.taigikeyboard.engine.proto.HasToneMarks
import com.siansiansu.taigikeyboard.engine.proto.IsTpsToneMark
import com.siansiansu.taigikeyboard.engine.proto.LexiconRequest
import com.siansiansu.taigikeyboard.engine.proto.LexiconResponse
import com.siansiansu.taigikeyboard.engine.proto.NormalizeInput
import com.siansiansu.taigikeyboard.engine.proto.NormalizeToTl
import com.siansiansu.taigikeyboard.engine.proto.NormalizeTone
import com.siansiansu.taigikeyboard.engine.proto.OptionalStringResult
import com.siansiansu.taigikeyboard.engine.proto.PhoneticsRequest
import com.siansiansu.taigikeyboard.engine.proto.PhoneticsResponse
import com.siansiansu.taigikeyboard.engine.proto.PojToTl
import com.siansiansu.taigikeyboard.engine.proto.ProcessCandidatesRequest
import com.siansiansu.taigikeyboard.engine.proto.Request
import com.siansiansu.taigikeyboard.engine.proto.Response
import com.siansiansu.taigikeyboard.engine.proto.RestoreTone
import com.siansiansu.taigikeyboard.engine.proto.StringResult
import com.siansiansu.taigikeyboard.engine.proto.StripTone
import com.siansiansu.taigikeyboard.engine.proto.StripToneResult
import com.siansiansu.taigikeyboard.engine.proto.TlDisplayToTps
import com.siansiansu.taigikeyboard.engine.proto.TlNumericToTps
import com.siansiansu.taigikeyboard.engine.proto.TlToPoj
import com.siansiansu.taigikeyboard.engine.proto.ToneVariationsResult
import com.siansiansu.taigikeyboard.engine.proto.TpsAdjustResult
import com.siansiansu.taigikeyboard.engine.proto.TpsInputAdjust
import com.siansiansu.taigikeyboard.engine.proto.TpsToTl
import com.siansiansu.taigikeyboard.ime.core.logging.LoggerBackend
import com.siansiansu.taigikeyboard.ime.core.logging.NullLoggerBackend
import com.siansiansu.taigikeyboard.ime.dictionary.FrequencyData
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord
import com.siansiansu.taigikeyboard.engine.proto.ScoreBreakdown as ProtoScoreBreakdown
import com.siansiansu.taigikeyboard.engine.proto.TaigiWord as ProtoTaigiWord
import java.util.ArrayDeque
import java.util.concurrent.atomic.AtomicInteger

/**
 * Thin Kotlin wrapper around the Rust shared-core FFI exposed by
 * `engine/android-jni/src/lib.rs`.
 *
 * D9.4 surface: 17 typed methods + lazy `toneVariations` cache + structured
 * error visibility. Production phonetics call sites swap to these methods
 * in commit 8; legacy `TaigiPhonetics` / `TPSConverter` deletes land in
 * commit 9.
 *
 * Per Codex v2 §7: `normalizeTone` requires `ToneToggles` mandatory
 * parameter — no `ToneToggles(true, true)` silent default.
 *
 * Per Codex v2 §8 + v3 §7 + v4 §5: error visibility is hardened. Failures
 * increment a counter and append a structured `DiagnosticsEntry` to a
 * 32-entry bounded queue (synchronized). DEBUG additionally calls
 * `Log.e` for logcat traceability — NEVER throws (would kill IME
 * mid-keystroke).
 */
object RustEngineBridge {
    init {
        System.loadLibrary("rust_taigi")
    }

    @Volatile
    private var installedBackend: LoggerBackend = NullLoggerBackend

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

    // region Phonetics core (9 ops)

    /**
     * `Method::NormalizeTone` — input + AppConfig.input_mode + ToneToggles →
     * tone-marked string. `mode` and `toggles` are mandatory (no default)
     * to enforce the live-read invariant per Codex v2 §7.
     */
    fun normalizeTone(input: String, mode: NormalizeMode, toggles: ToneTogglesCarrier): String {
        val payload = NormalizeTone.newBuilder().setInput(input).build()
        return stringDispatch(
            method = { it.normalizeTone = payload },
            input = input,
            op = "normalizeTone",
            config = appConfig(mode, toggles),
        )
    }

    fun stripTone(input: String): StripToneOutcome {
        val payload = StripTone.newBuilder().setInput(input).build()
        val resp = dispatch({ it.stripTone = payload }, "stripTone", null)
            ?: return StripToneOutcome(input, "")
        if (!resp.hasStripToneResult()) {
            recordFailure("stripTone", "missing StripToneResult")
            return StripToneOutcome(input, "")
        }
        val r: StripToneResult = resp.stripToneResult
        return StripToneOutcome(r.bare, r.tone)
    }

    fun pojToTl(input: String): String {
        val payload = PojToTl.newBuilder().setInput(input).build()
        return stringDispatch({ it.pojToTl = payload }, input, "pojToTl", null)
    }

    fun tlToPoj(input: String): String {
        val payload = TlToPoj.newBuilder().setInput(input).build()
        return stringDispatch({ it.tlToPoj = payload }, input, "tlToPoj", null)
    }

    fun normalizeToTl(input: String): String {
        val payload = NormalizeToTl.newBuilder().setInput(input).build()
        return stringDispatch({ it.normalizeToTl = payload }, input, "normalizeToTl", null)
    }

    fun normalizeInput(input: String): String {
        val payload = NormalizeInput.newBuilder().setInput(input).build()
        return stringDispatch({ it.normalizeInput = payload }, input, "normalizeInput", null)
    }

    /**
     * Replaces platform `TaigiUnicode.nfdPreprocessed(...)`. Lookup-side
     * NFD prep used by `ExternalLookupURLBuilder` before tone stripping.
     * Distinct semantics from [normalizeInput] — this preserves tone
     * diacritics; only nasal markers (ⁿ / ᴺ → "nn") and standalone
     * `\u{0358}` → `o` are rewritten.
     */
    fun nfdPreprocessForLookup(input: String): String {
        val payload = NfdPreprocessForLookup.newBuilder().setInput(input).build()
        return stringDispatch(
            { it.nfdPreprocessForLookup = payload },
            input,
            "nfdPreprocessForLookup",
            null,
        )
    }

    fun restoreTone(text: String): String? {
        val payload = RestoreTone.newBuilder().setText(text).build()
        val resp = dispatch({ it.restoreTone = payload }, "restoreTone", null) ?: return null
        if (!resp.hasOptionalStringResult()) {
            recordFailure("restoreTone", "missing OptionalStringResult")
            return null
        }
        val r: OptionalStringResult = resp.optionalStringResult
        return if (r.present) r.output else null
    }

    fun hasToneMarks(text: String): Boolean {
        val payload = HasToneMarks.newBuilder().setText(text).build()
        return boolDispatch({ it.hasToneMarks = payload }, "hasToneMarks")
    }

    /**
     * Lazy-init cache for Method::GetToneVariations. Kotlin `by lazy` defaults
     * to `LazyThreadSafetyMode.SYNCHRONIZED` — single execution + thread
     * safety guaranteed by language semantics. First reader pays the FFI
     * roundtrip; subsequent reads are zero-FFI.
     */
    val toneVariations: ToneVariationsCache by lazy {
        val payload = GetToneVariations.newBuilder().build()
        val resp = dispatch({ it.getToneVariations = payload }, "getToneVariations", null)
        if (resp == null || !resp.hasToneVariationsResult()) {
            recordFailure("getToneVariations", "missing ToneVariationsResult")
            ToneVariationsCache(emptyMap(), emptyMap())
        } else {
            val r: ToneVariationsResult = resp.toneVariationsResult
            ToneVariationsCache(
                poj = r.pojVariationsMap.mapValues { (_, v) -> v.variationsList.toList() },
                tl = r.tlVariationsMap.mapValues { (_, v) -> v.variationsList.toList() },
            )
        }
    }

    // endregion
    // region Derivation (2 ops)

    fun deriveNotone(roman: String): String {
        val payload = DeriveNotone.newBuilder().setRoman(roman).build()
        return stringDispatch({ it.deriveNotone = payload }, roman, "deriveNotone", null)
    }

    fun deriveAbbrev(roman: String): String {
        val payload = DeriveAbbrev.newBuilder().setRoman(roman).build()
        return stringDispatch({ it.deriveAbbrev = payload }, roman, "deriveAbbrev", null)
    }

    // endregion
    // region TPS (6 ops)

    fun containsTps(text: String): Boolean {
        val payload = ContainsTps.newBuilder().setText(text).build()
        return boolDispatch({ it.containsTps = payload }, "containsTps")
    }

    fun tpsToTl(text: String): String {
        val payload = TpsToTl.newBuilder().setText(text).build()
        return stringDispatch({ it.tpsToTl = payload }, text, "tpsToTl", null)
    }

    fun tlNumericToTps(text: String, orMapsToER: Boolean): String {
        val payload = TlNumericToTps.newBuilder().setText(text).setOrMapsToEr(orMapsToER).build()
        return stringDispatch({ it.tlNumericToTps = payload }, text, "tlNumericToTps", null)
    }

    fun tlDisplayToTps(text: String, orMapsToER: Boolean): String {
        val payload = TlDisplayToTps.newBuilder().setText(text).setOrMapsToEr(orMapsToER).build()
        return stringDispatch({ it.tlDisplayToTps = payload }, text, "tlDisplayToTps", null)
    }

    fun isTpsToneMark(char: Char): Boolean {
        val payload = IsTpsToneMark.newBuilder().setChar(char.toString()).build()
        return boolDispatch({ it.isTpsToneMark = payload }, "isTpsToneMark")
    }

    fun tpsInputAdjust(incoming: String, rawInput: String): TpsAdjustOutcome {
        val payload = TpsInputAdjust.newBuilder().setIncoming(incoming).setRawInput(rawInput).build()
        val resp = dispatch({ it.tpsInputAdjust = payload }, "tpsInputAdjust", null)
            ?: return TpsAdjustOutcome(incoming, null)
        if (!resp.hasTpsAdjustResult()) {
            recordFailure("tpsInputAdjust", "missing TpsAdjustResult")
            return TpsAdjustOutcome(incoming, null)
        }
        val r: TpsAdjustResult = resp.tpsAdjustResult
        val replace = if (r.hasReplaceLast() && r.replaceLast.present) r.replaceLast.output else null
        return TpsAdjustOutcome(r.adjusted, replace)
    }

    // endregion
    // region Lexicon ranking (1 op)

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
     * Production caller for the Rust ranking pipeline. Single FFI
     * round-trip runs dedup → score → sort → (TPS-gated) display-dedup
     * atomically inside `engine/ranking/`. Mirrors iOS
     * `RustEngineBridge.processCandidates`.
     *
     * `tpsDedupEnabled` is platform-decided per audit § 3 — pass
     * `settings?.inputMode == "tps"` from the call site. Engine never
     * derives it from any config field.
     *
     * `nowMs` is caller-supplied for deterministic recency-window math
     * in tests; production passes `System.currentTimeMillis()`.
     *
     * In `BuildConfig.DEBUG` builds, requests + emits the per-candidate
     * `ScoreBreakdown` so dogfood traces include the score arithmetic.
     * Release builds skip the breakdown (zero serialization overhead).
     */
    fun processCandidates(
        raw: List<TaigiWord>,
        normalizedInput: String,
        tpsDedupEnabled: Boolean,
        frequencyData: Map<String, FrequencyData>,
        nowMs: Long,
    ): List<TaigiWord> {
        val detailed = processCandidatesDetailed(
            raw = raw,
            normalizedInput = normalizedInput,
            tpsDedupEnabled = tpsDedupEnabled,
            frequencyData = frequencyData,
            nowMs = nowMs,
            includeBreakdown = BuildConfig.DEBUG,
        )
        if (BuildConfig.DEBUG && detailed.breakdowns.size == detailed.ranked.size) {
            for (i in detailed.ranked.indices) {
                val word = detailed.ranked[i]
                val b = detailed.breakdowns[i]
                installedBackend.d(
                    "RustEngineBridge",
                    "[SCORE] input='$normalizedInput' | ${word.roman} ${word.hanzi ?: ""}: " +
                        "user=${b.userFreqScore} recency=${b.recencyBonus} exact=${b.exactBonus} " +
                        "close=${b.closenessBonus} base=${b.baseFreqScore} " +
                        "completion=${b.completionPenalty} total=${b.total}",
                )
            }
        }
        return detailed.ranked
    }

    /**
     * Test seam — same FFI call as [processCandidates], plus access to the
     * per-candidate [ScoreBreakdown] payload. Production code stays on
     * [processCandidates] which discards the breakdown after debug logging.
     *
     * NOTE: `src/test/` JVM tests cannot exercise this seam because
     * `System.loadLibrary("rust_taigi")` fails on host JVM. Bridge
     * parity is verified by the Rust workspace tests + iOS XCTest
     * (links the xcframework) + Android instrumented dogfood.
     */
    fun processCandidatesDetailed(
        raw: List<TaigiWord>,
        normalizedInput: String,
        tpsDedupEnabled: Boolean,
        frequencyData: Map<String, FrequencyData>,
        nowMs: Long,
        includeBreakdown: Boolean,
    ): CandidateRanking {
        val payloadBuilder = ProcessCandidatesRequest.newBuilder()
            .setNormalizedInput(normalizedInput)
            .setTpsDedupEnabled(tpsDedupEnabled)
            .setNowMs(nowMs)
            .setIncludeBreakdown(includeBreakdown)
        for (word in raw) {
            payloadBuilder.addRaw(taigiWordToProto(word))
        }
        for ((key, value) in frequencyData) {
            payloadBuilder.addFreq(
                FrequencyEntry.newBuilder()
                    .setDisplayTextKey(key)
                    .setCount(maxOf(0, value.count))
                    .setLastUsedMs(value.lastUsedMillis)
                    .build(),
            )
        }
        val resp = lexiconDispatch(
            methodSetter = { it.processCandidates = payloadBuilder.build() },
            op = "processCandidates",
        )
        if (resp == null) {
            return CandidateRanking(
                ranked = fallbackRanked(raw, tpsDedupEnabled),
                breakdowns = emptyList(),
            )
        }
        if (!resp.hasProcessCandidatesResult()) {
            recordFailure("processCandidates", "missing process_candidates_result")
            return CandidateRanking(
                ranked = fallbackRanked(raw, tpsDedupEnabled),
                breakdowns = emptyList(),
            )
        }
        val result = resp.processCandidatesResult
        val ranked = result.rankedList.map(::taigiWordFromProto)
        val breakdowns = result.breakdownList.map(::scoreBreakdownFromProto)
        return CandidateRanking(ranked = ranked, breakdowns = breakdowns)
    }

    /**
     * Raw-list fallback on the FFI error path. When the Rust lexicon
     * dispatch fails (encode / decode error, non-OK engine response,
     * or missing payload variant), return the input list unchanged.
     *
     * Simplified in the v3.5.3 follow-up (PR #192): previously this
     * delegated to the Kotlin `CandidateProcessor.removeDuplicates` /
     * `removeDisplayDuplicates`
     * helpers as a defense-in-depth dedup. That silently masked Rust
     * dispatch bugs by producing a near-correct candidate list. The
     * `tpsDedupEnabled` parameter no longer changes behaviour here —
     * kept on the signature for caller-shape parity with the iOS
     * mirror (Codex audit § 1 Q3).
     */
    private fun fallbackRanked(
        raw: List<TaigiWord>,
        @Suppress("UNUSED_PARAMETER") tpsDedupEnabled: Boolean,
    ): List<TaigiWord> = raw

    private fun taigiWordToProto(word: TaigiWord): ProtoTaigiWord {
        val builder = ProtoTaigiWord.newBuilder()
            .setId(word.id.toLong())
            .setRoman(word.roman)
        word.hanzi?.let { builder.setHanji(it) }
        word.lengthScore?.let { builder.setLengthScore(it) }
        word.sourceBitmask?.let { builder.setSourceBitmask(it) }
        return builder.build()
    }

    private fun taigiWordFromProto(proto: ProtoTaigiWord): TaigiWord =
        TaigiWord(
            id = proto.id.toInt(),
            roman = proto.roman,
            hanzi = if (proto.hasHanji()) proto.hanji else null,
            lengthScore = if (proto.hasLengthScore()) proto.lengthScore else null,
            sourceBitmask = if (proto.hasSourceBitmask()) proto.sourceBitmask else null,
        )

    private fun scoreBreakdownFromProto(proto: ProtoScoreBreakdown): ScoreBreakdown =
        ScoreBreakdown(
            userFreqScore = proto.userFreqScore,
            recencyBonus = proto.recencyBonus,
            exactBonus = proto.exactBonus,
            completionPenalty = proto.completionPenalty,
            closenessBonus = proto.closenessBonus,
            baseFreqScore = proto.baseFreqScore,
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
    fun dispatchLog(level: Int, tag: String, msg: String) {
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
    // region Private dispatch + diagnostics

    private val diagnosticsLock = Any()
    private val failureCounter = AtomicInteger(0)
    private val recentErrors = ArrayDeque<DiagnosticsEntry>(RECENT_ERRORS_CAP)

    private fun recordFailure(op: String, message: String, code: Int = -1) {
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
            // Codex v3 §7 / v4 §5: in DEBUG also surface to logcat. No
            // throw — would kill the IME mid-keystroke.
            if (DEBUG) {
                Log.e("RustEngineBridge", "[$op] $message")
            }
        }
    }

    private fun appConfig(mode: NormalizeMode, toggles: ToneTogglesCarrier): AppConfig =
        AppConfig.newBuilder()
            .setInputMode(
                when (mode) {
                    NormalizeMode.POJ -> "poj"
                    NormalizeMode.TL -> "tl"
                    NormalizeMode.ENGLISH -> "english"
                },
            )
            .setOoDoubletapEnabled(toggles.isDoubleTapOoEnabled)
            .setNnDoubletapEnabled(toggles.isDoubleTapNnEnabled)
            .build()

    private inline fun dispatch(
        methodSetter: (PhoneticsRequest.Builder) -> Unit,
        op: String,
        config: AppConfig?,
    ): PhoneticsResponse? {
        val phoneticsBuilder = PhoneticsRequest.newBuilder()
        methodSetter(phoneticsBuilder)
        val requestBuilder = Request.newBuilder()
            .setId(nextId.incrementAndGet())
            .setPhonetics(phoneticsBuilder.build())
        if (config != null) {
            requestBuilder.configSnapshot = config
        }
        val response = sendRawBytes(requestBuilder.build().toByteArray())
        if (response == null) {
            recordFailure(op, "response decode failed")
            return null
        }
        if (response.error != ErrorCode.OK) {
            recordFailure(op, "engine returned ${response.error}", response.error.number)
            return null
        }
        if (!response.hasPhonetics()) {
            recordFailure(op, "missing phonetics payload")
            return null
        }
        return response.phonetics
    }

    private inline fun lexiconDispatch(
        methodSetter: (LexiconRequest.Builder) -> Unit,
        op: String,
    ): LexiconResponse? {
        val lexiconBuilder = LexiconRequest.newBuilder()
        methodSetter(lexiconBuilder)
        val request = Request.newBuilder()
            .setId(nextId.incrementAndGet())
            .setLexicon(lexiconBuilder.build())
            .build()
        val response = sendRawBytes(request.toByteArray())
        if (response == null) {
            recordFailure(op, "response decode failed")
            return null
        }
        if (response.error != ErrorCode.OK) {
            recordFailure(op, "engine returned ${response.error}", response.error.number)
            return null
        }
        if (!response.hasLexicon()) {
            recordFailure(op, "missing lexicon payload")
            return null
        }
        return response.lexicon
    }

    private inline fun stringDispatch(
        methodSetter: (PhoneticsRequest.Builder) -> Unit,
        input: String,
        op: String,
        config: AppConfig?,
    ): String {
        val resp = dispatch(methodSetter, op, config) ?: return input
        if (!resp.hasStringResult()) {
            recordFailure(op, "expected StringResult")
            return input
        }
        val r: StringResult = resp.stringResult
        return r.output
    }

    private inline fun boolDispatch(
        methodSetter: (PhoneticsRequest.Builder) -> Unit,
        op: String,
    ): Boolean {
        val resp = dispatch(methodSetter, op, null) ?: return false
        if (!resp.hasBoolResult()) {
            recordFailure(op, "expected BoolResult")
            return false
        }
        val r: BoolResult = resp.boolResult
        return r.value
    }

    private const val LEVEL_ERROR = 0
    private const val LEVEL_WARN = 1
    private const val LEVEL_INFO = 2
    private const val LEVEL_DEBUG = 3
    private const val RECENT_ERRORS_CAP = 32

    // Set true via BuildConfig in production project; hard-coded here so
    // bridge has no BuildConfig dependency. Override in tests / debug
    // builds via the diagnostics() / resetDiagnosticsForTesting() seams.
    private val DEBUG: Boolean = isDebugBuild()

    private fun isDebugBuild(): Boolean = try {
        Class.forName("com.siansiansu.taigikeyboard.BuildConfig")
            .getField("DEBUG")
            .getBoolean(null)
    } catch (_: Throwable) {
        false
    }

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

/** Result of `Method::StripTone`. */
data class StripToneOutcome(val bare: String, val tone: String)

/** Result of `Method::TpsInputAdjust`. */
data class TpsAdjustOutcome(val adjusted: String, val replaceLast: String?)

/** Init-bulk-pull cache for the callout tone variation tables. */
data class ToneVariationsCache(
    val poj: Map<String, List<String>>,
    val tl: Map<String, List<String>>,
)
