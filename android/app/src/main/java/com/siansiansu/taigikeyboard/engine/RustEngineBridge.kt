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
import com.siansiansu.taigikeyboard.ime.core.logging.LoggerBackend
import com.siansiansu.taigikeyboard.ime.core.logging.NullLoggerBackend
import com.siansiansu.taigikeyboard.ime.core.logging.tdebug
import com.siansiansu.taigikeyboard.ime.core.settings.InputMode
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
 * D9.4 surface: 15 typed phonetics methods + lazy `toneVariations` cache +
 * structured error visibility. Composing / NextWord / Lexicon / case-transform
 * methods live on dedicated bridge files. See
 * `engine/protos/proto/phonetics.proto` reserved-tag block for retired ops.
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

    // region Phonetics core (8 ops)

    /**
     * `Method::NormalizeTone` — input + AppConfig.input_mode + ToneToggles →
     * tone-marked string. `mode` and `toggles` are mandatory (no default)
     * to enforce the live-read invariant per Codex v2 §7.
     */
    fun normalizeTone(input: String, mode: NormalizeMode, toggles: ToneTogglesCarrier): String {
        val payload = NormalizeTone.newBuilder().setInput(input).build()
        return stringDispatch(
            methodSetter = { it.normalizeTone = payload },
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
    // region TPS (5 ops)

    fun containsTps(text: String): Boolean {
        val payload = ContainsTps.newBuilder().setText(text).build()
        return boolDispatch({ it.containsTps = payload }, "containsTps")
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
    // region Composing slice (12 ops) — v3.5.4

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
            data class UpdatePreedit(val display: String) : Effect()
            object ClearPreeditWithoutCommit : Effect()
            data class CommitTextReplacingPreedit(val text: String) : Effect()
            object DeleteBackwardFromDocument : Effect()
            object ResetAutocomplete : Effect()
            object PerformAutocomplete : Effect()
            object ResetAutocompleteContext : Effect()
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

    @JvmStatic
    fun composingStart(
        text: String,
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
        generation: Long,
    ): ComposingTransition {
        val payload = com.siansiansu.taigikeyboard.engine.proto.Start.newBuilder().setText(text).build()
        return composingDispatch(
            methodSetter = { it.start = payload },
            op = "composingStart",
            generation = generation,
            config = appConfig(mode, toggles),
        )
    }

    @JvmStatic
    fun composingAppend(
        ch: String,
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
        generation: Long,
    ): ComposingTransition {
        val payload = com.siansiansu.taigikeyboard.engine.proto.Append.newBuilder().setChar(ch).build()
        return composingDispatch(
            methodSetter = { it.append = payload },
            op = "composingAppend",
            generation = generation,
            config = appConfig(mode, toggles),
        )
    }

    @JvmStatic
    fun composingAppendHyphen(
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
        generation: Long,
    ): ComposingTransition {
        val payload = com.siansiansu.taigikeyboard.engine.proto.AppendHyphen.newBuilder().build()
        return composingDispatch(
            methodSetter = { it.appendHyphen = payload },
            op = "composingAppendHyphen",
            generation = generation,
            config = appConfig(mode, toggles),
        )
    }

    @JvmStatic
    fun composingReplaceLast(
        replacement: String,
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
        generation: Long,
    ): ComposingTransition {
        val payload = com.siansiansu.taigikeyboard.engine.proto.ReplaceLast.newBuilder()
            .setReplacement(replacement).build()
        return composingDispatch(
            methodSetter = { it.replaceLast = payload },
            op = "composingReplaceLast",
            generation = generation,
            config = appConfig(mode, toggles),
        )
    }

    @JvmStatic
    fun composingDeleteBackward(
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
        generation: Long,
    ): ComposingTransition {
        val payload = com.siansiansu.taigikeyboard.engine.proto.DeleteBackward.newBuilder().build()
        return composingDispatch(
            methodSetter = { it.deleteBackward = payload },
            op = "composingDeleteBackward",
            generation = generation,
            config = appConfig(mode, toggles),
        )
    }

    @JvmStatic
    fun composingCommitDerived(
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
        generation: Long,
    ): ComposingTransition {
        val payload = com.siansiansu.taigikeyboard.engine.proto.CommitDerived.newBuilder().build()
        return composingDispatch(
            methodSetter = { it.commitDerived = payload },
            op = "composingCommitDerived",
            generation = generation,
            config = appConfig(mode, toggles),
        )
    }

    @JvmStatic
    fun composingCommitRaw(generation: Long): ComposingTransition {
        val payload = com.siansiansu.taigikeyboard.engine.proto.CommitRaw.newBuilder().build()
        return composingDispatch(
            methodSetter = { it.commitRaw = payload },
            op = "composingCommitRaw",
            generation = generation,
            config = null,
        )
    }

    @JvmStatic
    fun composingSelectSuggestion(text: String, generation: Long): ComposingTransition {
        val payload = com.siansiansu.taigikeyboard.engine.proto.SelectSuggestion.newBuilder()
            .setText(text).build()
        return composingDispatch(
            methodSetter = { it.selectSuggestion = payload },
            op = "composingSelectSuggestion",
            generation = generation,
            config = null,
        )
    }

    @JvmStatic
    fun composingCommitPreeditThenInsertExternal(
        text: String,
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
        generation: Long,
    ): ComposingTransition {
        val payload = com.siansiansu.taigikeyboard.engine.proto
            .CommitPreeditThenInsertExternal.newBuilder()
            .setText(text).build()
        return composingDispatch(
            methodSetter = { it.commitPreeditThenInsertExternal = payload },
            op = "composingCommitPreeditThenInsertExternal",
            generation = generation,
            config = appConfig(mode, toggles),
        )
    }

    @JvmStatic
    fun composingReset(generation: Long): ComposingTransition {
        val payload = com.siansiansu.taigikeyboard.engine.proto.Reset.newBuilder().build()
        return composingDispatch(
            methodSetter = { it.reset = payload },
            op = "composingReset",
            generation = generation,
            config = null,
        )
    }

    @JvmStatic
    fun composingSetSelectedCandidateIndex(index: Int, generation: Long): ComposingTransition {
        val payload = com.siansiansu.taigikeyboard.engine.proto
            .SetSelectedCandidateIndex.newBuilder()
            .setIndex(index).build()
        return composingDispatch(
            methodSetter = { it.setSelectedCandidateIndex = payload },
            op = "composingSetSelectedCandidateIndex",
            generation = generation,
            config = null,
        )
    }

    @JvmStatic
    fun composingQueryState(generation: Long): ComposingTransition {
        val payload = com.siansiansu.taigikeyboard.engine.proto.QueryState.newBuilder().build()
        return composingDispatch(
            methodSetter = { it.queryState = payload },
            op = "composingQueryState",
            generation = generation,
            config = null,
        )
    }

    private inline fun composingDispatch(
        methodSetter: (com.siansiansu.taigikeyboard.engine.proto.ComposingRequest.Builder) -> Unit,
        op: String,
        generation: Long,
        config: AppConfig?,
    ): ComposingTransition {
        val composingBuilder = com.siansiansu.taigikeyboard.engine.proto.ComposingRequest.newBuilder()
        methodSetter(composingBuilder)
        val requestBuilder = Request.newBuilder()
            .setId(nextId.incrementAndGet())
            .setGeneration(generation)
            .setComposing(composingBuilder.build())
        if (config != null) {
            requestBuilder.configSnapshot = config
        }
        val request = requestBuilder.build()
        installedBackend.tdebug("RustEngineBridge") {
            "[FFI->] fn=composingDispatch op=$op id=${request.id} generation=$generation"
        }
        val response = sendRawBytes(request.toByteArray())
        if (response == null) {
            recordFailure(op, "response decode failed")
            return ComposingTransition.NOOP
        }
        if (response.error != ErrorCode.OK) {
            recordFailure(op, "engine returned ${response.error}", response.error.number)
            return ComposingTransition.NOOP
        }
        if (!response.hasComposing()) {
            recordFailure(op, "missing composing payload")
            return ComposingTransition.NOOP
        }
        val transition = synthComposing(response.composing)
        installedBackend.tdebug("RustEngineBridge") {
            "[FFI<-] fn=composingDispatch op=$op id=${request.id} effects=${transition.effects.size} composing=${transition.isComposing}"
        }
        return transition
    }

    private fun synthComposing(
        proto: com.siansiansu.taigikeyboard.engine.proto.ComposingResponse,
    ): ComposingTransition {
        val effects: List<ComposingTransition.Effect> = proto.effectList.mapNotNull { eff ->
            when {
                eff.hasUpdatePreedit() -> ComposingTransition.Effect.UpdatePreedit(eff.updatePreedit.display)
                eff.hasClearPreeditWithoutCommit() -> ComposingTransition.Effect.ClearPreeditWithoutCommit
                eff.hasCommitTextReplacingPreedit() ->
                    ComposingTransition.Effect.CommitTextReplacingPreedit(eff.commitTextReplacingPreedit.text)
                eff.hasDeleteBackwardFromDocument() -> ComposingTransition.Effect.DeleteBackwardFromDocument
                eff.hasResetAutocomplete() -> ComposingTransition.Effect.ResetAutocomplete
                eff.hasPerformAutocomplete() -> ComposingTransition.Effect.PerformAutocomplete
                eff.hasResetAutocompleteContext() -> ComposingTransition.Effect.ResetAutocompleteContext
                else -> null
            }
        }
        return ComposingTransition(
            rawInput = proto.preedit.rawInput,
            displayText = proto.preedit.displayText,
            effects = effects,
            selectedCandidateIndex = proto.selectedCandidateIndex,
            isComposing = proto.isComposing,
        )
    }

    // endregion
    // region NextWord slice (9 ops) — v3.5.5

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
            data class RescheduleContextTimeout(val afterMs: Long) : Effect()
            object CancelContextTimeout : Effect()
            data class RecordAssociation(val pair: NextWordAssociationPair) : Effect()
            data class RecordCompoundAssociations(val pairs: List<NextWordAssociationPair>) : Effect()
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
            data class ClearPredictionsUI(val generation: Long) : Effect()
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

    // -- Decide intents (6 — UpdateLastSelectedWord is Android-only) --

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
    ): NextWordDecideResult {
        val payload = com.siansiansu.taigikeyboard.engine.proto.WordSelected.newBuilder()
            .setText(text)
            .setRoman(roman)
            .setRequireRomanMode(requireRomanMode)
            .setTriggerPrediction(triggerPrediction)
            .setInput(decisionInput(nowMs))
            .build()
        return nextwordDecideDispatch(
            methodSetter = { it.wordSelected = payload },
            op = "nextwordWordSelected",
            generation = generation,
            config = nextwordConfig(mode, translateSwapped, associationRecordingEnabled),
        )
    }

    @JvmStatic
    fun nextwordBackspace(
        lastChar: String,
        nowMs: Long,
        mode: InputMode,
        translateSwapped: Boolean,
        associationRecordingEnabled: Boolean,
        generation: Long,
    ): NextWordDecideResult {
        val payload = com.siansiansu.taigikeyboard.engine.proto.Backspace.newBuilder()
            .setLastChar(lastChar)
            .setInput(decisionInput(nowMs))
            .build()
        return nextwordDecideDispatch(
            methodSetter = { it.backspace = payload },
            op = "nextwordBackspace",
            generation = generation,
            config = nextwordConfig(mode, translateSwapped, associationRecordingEnabled),
        )
    }

    @JvmStatic
    fun nextwordContextTimeoutFired(
        nowMs: Long,
        mode: InputMode,
        translateSwapped: Boolean,
        associationRecordingEnabled: Boolean,
        generation: Long,
    ): NextWordDecideResult {
        val payload = com.siansiansu.taigikeyboard.engine.proto.ContextTimeoutFired.newBuilder()
            .setInput(decisionInput(nowMs))
            .build()
        return nextwordDecideDispatch(
            methodSetter = { it.contextTimeoutFired = payload },
            op = "nextwordContextTimeoutFired",
            generation = generation,
            config = nextwordConfig(mode, translateSwapped, associationRecordingEnabled),
        )
    }

    @JvmStatic
    fun nextwordClearForNewComposing(
        nowMs: Long,
        mode: InputMode,
        translateSwapped: Boolean,
        associationRecordingEnabled: Boolean,
        generation: Long,
    ): NextWordDecideResult {
        val payload = com.siansiansu.taigikeyboard.engine.proto.ClearForNewComposing.newBuilder()
            .setInput(decisionInput(nowMs))
            .build()
        return nextwordDecideDispatch(
            methodSetter = { it.clearForNewComposing = payload },
            op = "nextwordClearForNewComposing",
            generation = generation,
            config = nextwordConfig(mode, translateSwapped, associationRecordingEnabled),
        )
    }

    @JvmStatic
    fun nextwordResetFull(
        nowMs: Long,
        mode: InputMode,
        translateSwapped: Boolean,
        associationRecordingEnabled: Boolean,
        generation: Long,
    ): NextWordDecideResult {
        val payload = com.siansiansu.taigikeyboard.engine.proto.ResetFull.newBuilder()
            .setInput(decisionInput(nowMs))
            .build()
        return nextwordDecideDispatch(
            methodSetter = { it.resetFull = payload },
            op = "nextwordResetFull",
            generation = generation,
            config = nextwordConfig(mode, translateSwapped, associationRecordingEnabled),
        )
    }

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
    ): NextWordDecideResult {
        val payload = com.siansiansu.taigikeyboard.engine.proto.SetIsShowing.newBuilder()
            .setIsShowing(isShowing)
            .build()
        return nextwordDecideDispatch(
            methodSetter = { it.setIsShowing = payload },
            op = "nextwordSetIsShowing",
            generation = generation,
            config = nextwordConfig(mode, translateSwapped, associationRecordingEnabled),
        )
    }

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
    ): NextWordDecideResult {
        val payload = com.siansiansu.taigikeyboard.engine.proto.UpdateLastSelectedWord.newBuilder()
            .setText(text)
            .setRoman(roman)
            .setInput(decisionInput(nowMs))
            .build()
        return nextwordDecideDispatch(
            methodSetter = { it.updateLastSelectedWord = payload },
            op = "nextwordUpdateLastSelectedWord",
            generation = generation,
            config = nextwordConfig(mode, translateSwapped, associationRecordingEnabled),
        )
    }

    // -- Filter / Boost / QueryState --

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
    ): NextWordFilterResult {
        val builder = com.siansiansu.taigikeyboard.engine.proto.FilterPredictions.newBuilder()
            .setQueryGeneration(queryGeneration)
            .setNowMs(nowMs)
            .setLimit(limit)
        for (row in raw) {
            builder.addRaw(
                com.siansiansu.taigikeyboard.engine.proto.RawNextWordPrediction.newBuilder()
                    .setHanzi(row.hanzi)
                    .setTl(row.tl)
                    .setCount(row.count)
                    .setLastUsedMs(row.lastUsedMs)
                    .setSource(
                        when (row.source) {
                            NextWordRawRow.Source.DICT -> com.siansiansu.taigikeyboard.engine.proto.Source.SOURCE_DICT
                            NextWordRawRow.Source.USER -> com.siansiansu.taigikeyboard.engine.proto.Source.SOURCE_USER
                        },
                    )
                    .build(),
            )
        }
        val resp = nextwordDispatch(
            methodSetter = { it.filterPredictions = builder.build() },
            op = "nextwordFilter",
            generation = generation,
            config = nextwordConfig(mode, translateSwapped, associationRecordingEnabled),
        ) ?: return NextWordFilterResult(emptyList(), wasStale = false)
        if (!resp.hasFilter()) {
            recordFailure("nextwordFilter", "missing filter result")
            return NextWordFilterResult(emptyList(), wasStale = false)
        }
        val filter = resp.filter
        val predictions = filter.predictionsList.map { p ->
            NextWordEnginePrediction(
                text = p.text,
                subtitle = if (p.subtitle.isEmpty()) null else p.subtitle,
                hanzi = p.hanzi,
                tl = p.tl,
                score = p.score,
            )
        }
        return NextWordFilterResult(predictions = predictions, wasStale = filter.wasStale)
    }

    @JvmStatic
    fun nextwordBoostCandidates(
        words: List<String>,
        predictedFirstChars: Set<String>,
        mode: InputMode,
        translateSwapped: Boolean,
        associationRecordingEnabled: Boolean,
        generation: Long,
    ): List<String> {
        val payload = com.siansiansu.taigikeyboard.engine.proto.BoostCandidates.newBuilder()
            .addAllWords(words)
            .addAllPredictedFirstChars(predictedFirstChars)
            .build()
        val resp = nextwordDispatch(
            methodSetter = { it.boostCandidates = payload },
            op = "nextwordBoostCandidates",
            generation = generation,
            config = nextwordConfig(mode, translateSwapped, associationRecordingEnabled),
        ) ?: return words
        if (!resp.hasBoost()) {
            recordFailure("nextwordBoostCandidates", "missing boost result")
            return words
        }
        return resp.boost.wordsList.toList()
    }

    @JvmStatic
    fun nextwordQueryState(
        mode: InputMode,
        translateSwapped: Boolean,
        associationRecordingEnabled: Boolean,
        generation: Long,
    ): NextWordStateSnapshot {
        val payload = com.siansiansu.taigikeyboard.engine.proto.NextWordQueryState.newBuilder().build()
        val resp = nextwordDispatch(
            methodSetter = { it.queryState = payload },
            op = "nextwordQueryState",
            generation = generation,
            config = nextwordConfig(mode, translateSwapped, associationRecordingEnabled),
        ) ?: return NextWordStateSnapshot(null, false, 0L)
        if (!resp.hasStateSnapshot()) {
            recordFailure("nextwordQueryState", "missing state snapshot")
            return NextWordStateSnapshot(null, false, 0L)
        }
        val s = resp.stateSnapshot
        return NextWordStateSnapshot(
            lastSelectedWord = if (s.lastSelectedWord.isEmpty()) null else s.lastSelectedWord,
            isShowing = s.isShowing,
            currentGeneration = s.currentGeneration,
        )
    }

    // -- Private helpers --

    private fun decisionInput(nowMs: Long): com.siansiansu.taigikeyboard.engine.proto.DecisionInput =
        com.siansiansu.taigikeyboard.engine.proto.DecisionInput.newBuilder()
            .setNowMs(nowMs)
            .build()

    private fun nextwordConfig(
        mode: InputMode,
        translateSwapped: Boolean,
        associationRecordingEnabled: Boolean,
    ): AppConfig =
        AppConfig.newBuilder()
            .setInputMode(
                when (mode) {
                    InputMode.POJ -> "poj"
                    InputMode.TL -> "tl"
                    InputMode.ENGLISH -> "english"
                },
            )
            .setOoDoubletapEnabled(false)
            .setNnDoubletapEnabled(false)
            .setIsTranslateSwapped(translateSwapped)
            .setIsAssociationRecordingEnabled(associationRecordingEnabled)
            .setPlatformId(com.siansiansu.taigikeyboard.engine.proto.Platform.PLATFORM_ANDROID)
            .build()

    private inline fun nextwordDispatch(
        methodSetter: (com.siansiansu.taigikeyboard.engine.proto.NextWordRequest.Builder) -> Unit,
        op: String,
        generation: Long,
        config: AppConfig,
    ): com.siansiansu.taigikeyboard.engine.proto.NextWordResponse? {
        val nextwordBuilder = com.siansiansu.taigikeyboard.engine.proto.NextWordRequest.newBuilder()
        methodSetter(nextwordBuilder)
        val request = Request.newBuilder()
            .setId(nextId.incrementAndGet())
            .setGeneration(generation)
            .setConfigSnapshot(config)
            .setNextword(nextwordBuilder.build())
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
        if (!response.hasNextword()) {
            recordFailure(op, "missing nextword payload")
            return null
        }
        return response.nextword
    }

    private inline fun nextwordDecideDispatch(
        methodSetter: (com.siansiansu.taigikeyboard.engine.proto.NextWordRequest.Builder) -> Unit,
        op: String,
        generation: Long,
        config: AppConfig,
    ): NextWordDecideResult {
        val resp = nextwordDispatch(methodSetter, op, generation, config) ?: return NextWordDecideResult.NOOP
        if (!resp.hasDecide()) {
            recordFailure(op, "missing decide result")
            return NextWordDecideResult.NOOP
        }
        return synthDecideResult(resp.decide)
    }

    private fun synthDecideResult(
        proto: com.siansiansu.taigikeyboard.engine.proto.DecideResult,
    ): NextWordDecideResult {
        val effects: List<NextWordDecideResult.Effect> = proto.effectsList.mapNotNull { eff ->
            when {
                eff.hasRescheduleContextTimeout() ->
                    NextWordDecideResult.Effect.RescheduleContextTimeout(eff.rescheduleContextTimeout.afterMs)
                eff.hasCancelContextTimeout() -> NextWordDecideResult.Effect.CancelContextTimeout
                eff.hasRecordAssociation() ->
                    NextWordDecideResult.Effect.RecordAssociation(synthAssociationPair(eff.recordAssociation.pair))
                eff.hasRecordCompoundAssociations() ->
                    NextWordDecideResult.Effect.RecordCompoundAssociations(
                        eff.recordCompoundAssociations.pairsList.map(::synthAssociationPair),
                    )
                eff.hasQueryPredictions() ->
                    NextWordDecideResult.Effect.QueryPredictions(
                        word = eff.queryPredictions.word,
                        roman = eff.queryPredictions.roman,
                        generation = eff.queryPredictions.generation,
                        nowMs = eff.queryPredictions.nowMs,
                    )
                eff.hasClearPredictionsUi() ->
                    NextWordDecideResult.Effect.ClearPredictionsUI(eff.clearPredictionsUi.generation)
                else -> null
            }
        }
        return NextWordDecideResult(
            effects = effects,
            currentGeneration = proto.currentGeneration,
            isShowing = proto.isShowing,
            lastSelectedWord = if (proto.lastSelectedWord.isEmpty()) null else proto.lastSelectedWord,
        )
    }

    private fun synthAssociationPair(
        proto: com.siansiansu.taigikeyboard.engine.proto.AssociationPair,
    ): NextWordAssociationPair = NextWordAssociationPair(
        prev = proto.prev,
        prevTl = proto.prevTl,
        next = proto.next,
        nextTl = proto.nextTl,
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
     * Internal dispatch seam for sibling bridges (`LexiconBridge`) that
     * live outside this object but share the same JNI plumbing. Same
     * package only — `internal` Kotlin visibility plus `engine` package.
     * Wraps `processRequestBytes` so the JNI symbol stays bound to
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
