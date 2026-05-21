// 中文: Rust shared-core 的 Kotlin 薄殼 — 對應 engine/android-jni FFI。
// 中文: 包覆 phonetics / lexicon / composing / nextword / case-transform 等 op,
// 中文: 處理錯誤統計、log 註冊、ToneVariations 快取等平台粘合,失敗永不丟例外。
// 中文: 對應 iOS RustEngineBridge.swift。

package com.siansiansu.taigikeyboard.engine

import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.engine.proto.AppConfig
import com.siansiansu.taigikeyboard.engine.proto.BoolResult
import com.siansiansu.taigikeyboard.engine.proto.ContainsTps
import com.siansiansu.taigikeyboard.engine.proto.DeriveAbbrev
import com.siansiansu.taigikeyboard.engine.proto.CustomDictEntry
import com.siansiansu.taigikeyboard.engine.proto.DeriveNotone
import com.siansiansu.taigikeyboard.engine.proto.ErrorCode
import com.siansiansu.taigikeyboard.engine.proto.FrequencyEntry
import com.siansiansu.taigikeyboard.engine.proto.GetToneVariations
import com.siansiansu.taigikeyboard.engine.proto.IsTpsToneMark
import com.siansiansu.taigikeyboard.engine.proto.LexiconRequest
import com.siansiansu.taigikeyboard.engine.proto.LexiconResponse
import com.siansiansu.taigikeyboard.engine.proto.NfdPreprocessForLookup
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
import java.util.ArrayDeque
import java.util.concurrent.atomic.AtomicInteger
import com.siansiansu.taigikeyboard.engine.proto.ScoreBreakdown as ProtoScoreBreakdown
import com.siansiansu.taigikeyboard.engine.proto.TaigiWord as ProtoTaigiWord

/**
 * Thin Kotlin wrapper around the Rust shared-core FFI exposed by
 * `engine/android-jni/src/lib.rs`.
 *
 * D9.4 surface: 15 typed phonetics methods + lazy `toneVariations` cache +
 * structured error visibility. Composing / NextWord / Lexicon / case-transform
 * methods live on dedicated bridge files. Retired-op history available via
 * git log on `engine/protos/proto/phonetics.proto`.
 *
 * Per Codex v2 §7: `normalizeTone` requires `ToneToggles` mandatory
 * parameter — no `ToneToggles(true, true)` silent default.
 *
 * Per Codex v2 §8 + v3 §7 + v4 §5: error visibility is hardened. Failures
 * increment a counter and append a structured `DiagnosticsEntry` to a
 * 32-entry bounded queue (synchronized). DEBUG additionally surfaces the
 * failure via `installedBackend.e` for logcat traceability — NEVER throws
 * (would kill IME mid-keystroke).
 *
 * 中文: 失敗一律不丟例外(否則會中斷打字),改記入 32-entry diagnostics 環狀佇列;
 *       DEBUG 同時透過 facade .e 打 logcat 方便追蹤。
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
     *
     * 中文: 冪等安裝 JNI logger 橋。DEBUG 模式 Rust log level 拉到 Debug,release 保持 Warn 走 zero-cost。
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
     *
     * 中文: 把數字調 ASCII 輸入轉為帶調符字串;mode 與 toggles 必填以強制 live-read 不快照。
     */
    fun normalizeTone(
        input: String,
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
    ): String {
        val payload = NormalizeTone.newBuilder().setInput(input).build()
        return stringDispatch(
            methodSetter = { it.normalizeTone = payload },
            input = input,
            op = "normalizeTone",
            config = appConfig(mode, toggles),
        )
    }

    // 中文: 把音節聲調 combining mark 剝離,回傳 (bare, tone) 對;無聲調時 tone 為空字串。
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

    // 中文: POJ 顯示字串 → TL 顯示字串轉換,逐音節重排版,保留聲調記號。
    fun pojToTl(input: String): String {
        val payload = PojToTl.newBuilder().setInput(input).build()
        return stringDispatch({ it.pojToTl = payload }, input, "pojToTl", null)
    }

    // 中文: TL 顯示字串 → POJ 顯示字串轉換,逐音節重排版,保留聲調記號。
    fun tlToPoj(input: String): String {
        val payload = TlToPoj.newBuilder().setInput(input).build()
        return stringDispatch({ it.tlToPoj = payload }, input, "tlToPoj", null)
    }

    // 中文: 將輸入規範化為 TL 拼寫形式(POJ 拼法→TL 拼法)以便後續解析。
    fun normalizeToTl(input: String): String {
        val payload = NormalizeToTl.newBuilder().setInput(input).build()
        return stringDispatch({ it.normalizeToTl = payload }, input, "normalizeToTl", null)
    }

    // 中文: NormalizeInput 完整管線:TPS preprocess → 小寫 → 切音節 → 鼻音/o͘ 預處理 + checked-ending 推論,產 trie-query key。
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
     *
     * 中文: 外部查詢 URL 用的 NFD 預處理 — 保留聲調符號,只把鼻音(ⁿ/ᴺ)→"nn" 與 ͘ → o。
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

    // 中文: Backspace 路徑用 — 找到最後一個 NFD 聲調 mark 拔掉、NFC 重組;無 mark 回 null。
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

    // 中文: 自訂字典 search-key 衍生 — 去聲調的 toneless 形式,給 toneless prefix search 用。
    fun deriveNotone(roman: String): String {
        val payload = DeriveNotone.newBuilder().setRoman(roman).build()
        return stringDispatch({ it.deriveNotone = payload }, roman, "deriveNotone", null)
    }

    // 中文: 自訂字典 search-key 衍生 — 取每音節首字母縮寫(連字號/空白切),單音節回空字串。
    fun deriveAbbrev(roman: String): String {
        val payload = DeriveAbbrev.newBuilder().setRoman(roman).build()
        return stringDispatch({ it.deriveAbbrev = payload }, roman, "deriveAbbrev", null)
    }

    // endregion
    // region TPS (5 ops)

    // 中文: 判斷字串是否含 TPS(注音符號)— Composing 衍生顯示用來略過 POJ/TL 聲調轉換。
    fun containsTps(text: String): Boolean {
        val payload = ContainsTps.newBuilder().setText(text).build()
        return boolDispatch({ it.containsTps = payload }, "containsTps")
    }

    // 中文: TL 數字調 → TPS(注音);orMapsToER 控制 er↔or 變體對應。
    fun tlNumericToTps(
        text: String,
        orMapsToER: Boolean,
    ): String {
        val payload = TlNumericToTps
            .newBuilder()
            .setText(text)
            .setOrMapsToEr(orMapsToER)
            .build()
        return stringDispatch({ it.tlNumericToTps = payload }, text, "tlNumericToTps", null)
    }

    // 中文: TL 顯示字串 → TPS(注音);orMapsToER 同 tlNumericToTps。
    fun tlDisplayToTps(
        text: String,
        orMapsToER: Boolean,
    ): String {
        val payload = TlDisplayToTps
            .newBuilder()
            .setText(text)
            .setOrMapsToEr(orMapsToER)
            .build()
        return stringDispatch({ it.tlDisplayToTps = payload }, text, "tlDisplayToTps", null)
    }

    // 中文: 判斷字元是否為 TPS 聲調記號(用於鍵盤觸發後處理 + composing 預編輯顯示判斷)。
    fun isTpsToneMark(char: Char): Boolean {
        val payload = IsTpsToneMark.newBuilder().setChar(char.toString()).build()
        return boolDispatch({ it.isTpsToneMark = payload }, "isTpsToneMark")
    }

    // 中文: TPS 鍵級輸入調整 — 依 incoming 字元與當前 rawInput 決定 (adjusted, replaceLast?);
    // 中文: replaceLast 非空時呼叫端應把上一字以 replaceLast 取代。
    fun tpsInputAdjust(
        incoming: String,
        rawInput: String,
    ): TpsAdjustOutcome {
        val payload = TpsInputAdjust
            .newBuilder()
            .setIncoming(incoming)
            .setRawInput(rawInput)
            .build()
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
     *
     * 中文: 排序生產入口 — 單次 FFI 跑完 dedup→score→sort→(TPS 模式)display-dedup;
     *       tpsDedupEnabled 由平台端決定(讀 settings.inputMode == "tps"),Engine 不自行推。
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
     *
     * 中文: 測試用入口,可取出每筆候選的 ScoreBreakdown(六項分數);production 走 processCandidates 即可。
     */
    fun processCandidatesDetailed(
        raw: List<TaigiWord>,
        normalizedInput: String,
        tpsDedupEnabled: Boolean,
        frequencyData: Map<String, FrequencyData>,
        nowMs: Long,
        includeBreakdown: Boolean,
    ): CandidateRanking {
        val payloadBuilder = ProcessCandidatesRequest
            .newBuilder()
            .setNormalizedInput(normalizedInput)
            .setTpsDedupEnabled(tpsDedupEnabled)
            .setNowMs(nowMs)
            .setIncludeBreakdown(includeBreakdown)
        for (word in raw) {
            payloadBuilder.addRaw(taigiWordToProto(word))
        }
        payloadBuilder.addAllFreq(frequencyDataToProtoEntries(frequencyData))
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
     * delegated to Kotlin-side dedup helpers (since removed) as a
     * defense-in-depth dedup. That silently masked Rust dispatch bugs
     * by producing a near-correct candidate list. The
     * `tpsDedupEnabled` parameter no longer changes behaviour here —
     * kept on the signature for caller-shape parity with the iOS
     * mirror (Codex audit § 1 Q3).
     */
    private fun fallbackRanked(
        raw: List<TaigiWord>,
        @Suppress("UNUSED_PARAMETER") tpsDedupEnabled: Boolean,
    ): List<TaigiWord> = raw

    /**
     * Marshal a `Map<String, FrequencyData>` snapshot into the proto
     * `FrequencyEntry` wire shape consumed by `ProcessCandidatesRequest`
     * (legacy lexicon ranking) and `FetchAtPos` (Continuous-input fetch,
     * v3.5.8 Phase 9.3a/9.3c). Centralises the `count` clamp + field
     * naming so the two callers cannot drift. Mirrors iOS implicit
     * convention — Swift inlines the same shape but at one call site.
     */
    internal fun frequencyDataToProtoEntries(
        data: Map<String, FrequencyData>,
    ): List<FrequencyEntry> = data.map { (word, snapshot) ->
        FrequencyEntry
            .newBuilder()
            .setDisplayTextKey(word)
            .setCount(maxOf(0, snapshot.count))
            .setLastUsedMs(snapshot.lastUsedMillis)
            .build()
    }

    private fun taigiWordToProto(word: TaigiWord): ProtoTaigiWord {
        val builder = ProtoTaigiWord
            .newBuilder()
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
     * `rules/cross-platform-alignment.md`).
     *
     * Metadata-only in v3.5.8 — does NOT enter the engine's `SortKey`
     * tie-break (per `docs/roadmap.md` § Phase 9 R2 Q3.a). `UNSPECIFIED`
     * is the proto3 default and means "unknown carrier — old engine or
     * dropped field"; never emitted by the current Rust engine.
     * Platforms must treat `UNSPECIFIED` as "ignore mode" rather than
     * falling back to any local classification.
     */
    // 中文: Phase 9.2 候選類型軸;Rust 端 derive_mode 推導,平台僅讀不算(禁 display_text sniff)。
    // 中文:   metadata-only,不入 SortKey。UNSPECIFIED = wire 上 mode 缺漏 → 視為「無 mode 資訊」。
    enum class CandidateMode {
        UNSPECIFIED,
        HANT,
        TAILO,
        MIXED;

        companion object {
            /**
             * Decode the wire integer (`CandidateMessage.getModeValue()`)
             * produced by protobuf-javalite. Unrecognized values
             * (forward-compat from a newer engine) collapse to `UNSPECIFIED`
             * so the platform never crashes on a binding mismatch.
             */
            // 中文: 由 proto wire 整數解碼;未知值 fall back UNSPECIFIED,避免 binding mismatch crash。
            fun decode(wire: Int): CandidateMode = when (wire) {
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
     * slices `pending[start..end]` on commit. `form` is currently always 1
     * (FORM_NOTONE). `mode` is the Phase 9.2 carrier; metadata-only.
     */
    // 中文: 連續輸入候選詞,對應 proto CandidateMessage。consumed span 是 raw
    // 中文: 緩衝區的 byte offset(TL/POJ = ASCII;TPS = Bopomofo)。form 目前固定 1;mode 為 Phase 9.2 metadata-only。
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
        // 中文: Item 5 — 顯示羅馬字 sidechannel(引擎依 input mode 渲染:TL 或 POJ),dual-line 候選列 render 用。
        val roman: String,
        /**
         * v3.5.8 Phase 9 Item 5 — hanji display sidechannel. `null`
         * iff the proto3 `optional string hanji` was absent on the
         * wire (TAILO candidate). Present-empty is treated as
         * present (engine never emits `Some("")` today; defensive).
         */
        // 中文: Item 5 — 漢字 sidechannel;TAILO 候選 wire 上 absent → Kotlin null。
        val hanji: String?,
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
    // 中文: composingFetchAtPos 的查詢結果。candidates 三態只在 isBridgeFailure == false 時有意義。
    // 中文:   null = 不在 Continuous phase;emptyList = 在但無候選;non-empty = 有候選。
    // 中文: transition 帶 engine 狀態(FetchAtPos 只讀,effects 必為空)。
    // 中文: isBridgeFailure 區分「引擎回 Idle」與「FFI 失敗」— 後者套用 transition 會清掉鏡射狀態。
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

    // 中文: 開始 composing — Idle → Composing { raw=text };發出對應 UpdatePreedit Effect。
    @JvmStatic
    fun composingStart(
        text: String,
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
        generation: Long,
    ): ComposingTransition {
        val payload = com.siansiansu.taigikeyboard.engine.proto.Start
            .newBuilder()
            .setText(text)
            .build()
        return composingDispatch(
            methodSetter = { it.start = payload },
            op = "composingStart",
            generation = generation,
            config = appConfig(mode, toggles),
        )
    }

    // 中文: 追加一個字元到 composing buffer 末端;raw += ch,Engine 重算 displayText。
    @JvmStatic
    fun composingAppend(
        ch: String,
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
        generation: Long,
    ): ComposingTransition {
        val payload = com.siansiansu.taigikeyboard.engine.proto.Append
            .newBuilder()
            .setChar(ch)
            .build()
        return composingDispatch(
            methodSetter = { it.append = payload },
            op = "composingAppend",
            generation = generation,
            config = appConfig(mode, toggles),
        )
    }

    // 中文: 追加音節分隔連字號 — 區分 raw "tai-uan" 與 "taiuan",影響候選 trie key。
    @JvmStatic
    fun composingAppendHyphen(
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
        generation: Long,
    ): ComposingTransition {
        val payload = com.siansiansu.taigikeyboard.engine.proto.AppendHyphen
            .newBuilder()
            .build()
        return composingDispatch(
            methodSetter = { it.appendHyphen = payload },
            op = "composingAppendHyphen",
            generation = generation,
            config = appConfig(mode, toggles),
        )
    }

    // 中文: 取代 raw 最後一個字元(用於 TPS 鍵級調整、聲調覆蓋等場景)。
    @JvmStatic
    fun composingReplaceLast(
        replacement: String,
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
        generation: Long,
    ): ComposingTransition {
        val payload = com.siansiansu.taigikeyboard.engine.proto.ReplaceLast
            .newBuilder()
            .setReplacement(replacement)
            .build()
        return composingDispatch(
            methodSetter = { it.replaceLast = payload },
            op = "composingReplaceLast",
            generation = generation,
            config = appConfig(mode, toggles),
        )
    }

    // 中文: composing buffer 退一格;Engine 處理「刪到空就回 Idle」與 1-char delete 的特殊路徑(避免誤刪文件字)。
    @JvmStatic
    fun composingDeleteBackward(
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
        generation: Long,
    ): ComposingTransition {
        val payload = com.siansiansu.taigikeyboard.engine.proto.DeleteBackward
            .newBuilder()
            .build()
        return composingDispatch(
            methodSetter = { it.deleteBackward = payload },
            op = "composingDeleteBackward",
            generation = generation,
            config = appConfig(mode, toggles),
        )
    }

    // 中文: 提交 derived(顯示用)字串到文件 — 例如 "ho2" 顯示為 "hó",commit "hó"。
    @JvmStatic
    fun composingCommitDerived(
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
        generation: Long,
    ): ComposingTransition {
        val payload = com.siansiansu.taigikeyboard.engine.proto.CommitDerived
            .newBuilder()
            .build()
        return composingDispatch(
            methodSetter = { it.commitDerived = payload },
            op = "composingCommitDerived",
            generation = generation,
            config = appConfig(mode, toggles),
        )
    }

    // 中文: 提交 raw 字串。Composing 階段送字面 keystrokes (e.g. commit "ho2"),
    // 中文: Continuous 階段送 derived_display(pending) (e.g. "hó")。引擎依 phase 自動分派,
    // 中文: 故 Continuous 端需要 AppConfig (input mode + tone toggles) 才能正確 render derived。
    // 中文: Phase 9 Item 3 (2026-05-13) 起加入 mode/toggles 參數。
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
    ): ComposingTransition {
        val payload = com.siansiansu.taigikeyboard.engine.proto.CommitRaw
            .newBuilder()
            .build()
        return composingDispatch(
            methodSetter = { it.commitRaw = payload },
            op = "composingCommitRaw",
            generation = generation,
            config = continuousAppConfig(mode, toggles, effectiveSwapped, outputBothScripts),
        )
    }

    // 中文: 從候選列表選定一筆 suggestion — commit 該 suggestion 並重置 composing。
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
    ): ComposingTransition {
        val payload = com.siansiansu.taigikeyboard.engine.proto.SelectSuggestion
            .newBuilder()
            .setText(text)
            .build()
        return composingDispatch(
            methodSetter = { it.selectSuggestion = payload },
            op = "composingSelectSuggestion",
            generation = generation,
            config = continuousAppConfig(mode, toggles, effectiveSwapped, outputBothScripts),
        )
    }

    // 中文: 先 commit 當前 preedit、再插入外部字串(空白 / Enter / 標點等),原子操作避免閃爍。
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
    ): ComposingTransition {
        val payload = com.siansiansu.taigikeyboard.engine.proto
            .CommitPreeditThenInsertExternal
            .newBuilder()
            .setText(text)
            .build()
        return composingDispatch(
            methodSetter = { it.commitPreeditThenInsertExternal = payload },
            op = "composingCommitPreeditThenInsertExternal",
            generation = generation,
            config = continuousAppConfig(mode, toggles, effectiveSwapped, outputBothScripts),
        )
    }

    // 中文: 清空 composing buffer 不 commit — 用於切 input mode、切焦點欄位、退出 composing 等狀況。
    @JvmStatic
    fun composingReset(generation: Long): ComposingTransition {
        val payload = com.siansiansu.taigikeyboard.engine.proto.Reset
            .newBuilder()
            .build()
        return composingDispatch(
            methodSetter = { it.reset = payload },
            op = "composingReset",
            generation = generation,
            config = null,
        )
    }

    // 中文: UI 端通知當前選中候選 index — 給 NextWord/Booster 取 contextword 用,不 commit。
    @JvmStatic
    fun composingSetSelectedCandidateIndex(
        index: Int,
        generation: Long,
    ): ComposingTransition {
        val payload = com.siansiansu.taigikeyboard.engine.proto
            .SetSelectedCandidateIndex
            .newBuilder()
            .setIndex(index)
            .build()
        return composingDispatch(
            methodSetter = { it.setSelectedCandidateIndex = payload },
            op = "composingSetSelectedCandidateIndex",
            generation = generation,
            config = null,
        )
    }

    // 中文: 純讀 — 取當前 composing 狀態快照,不變更 Engine。1-char delete 路徑用此查 buffer 長度。
    @JvmStatic
    fun composingQueryState(generation: Long): ComposingTransition {
        val payload = com.siansiansu.taigikeyboard.engine.proto.QueryState
            .newBuilder()
            .build()
        return composingDispatch(
            methodSetter = { it.queryState = payload },
            op = "composingQueryState",
            generation = generation,
            config = null,
        )
    }

    // region Continuous-input (4 ops) — v3.5.8

    /**
     * `Phase::Composing { raw }` → `Phase::Continuous { raw, committed: [] }`.
     * Phase 6 contract: no payload — buffer is whatever earlier `Start` /
     * `Append` populated. Engine no-ops on Idle / already-Continuous / empty
     * `Composing.raw`. AppConfig is required because the snapshot's preedit
     * display goes through `derived_display(raw, config)`.
     */
    // 中文: 把 Composing 轉到 Continuous。Phase 6 規約 — 無 payload,raw 來自先前的
    // 中文: Start / Append。空 raw / 非 Composing 一律 noop。
    @JvmStatic
    fun composingEnterContinuous(
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
        generation: Long,
    ): ComposingTransition {
        val payload = com.siansiansu.taigikeyboard.engine.proto.EnterContinuous
            .newBuilder()
            .build()
        return composingDispatch(
            methodSetter = { it.enterContinuous = payload },
            op = "composingEnterContinuous",
            generation = generation,
            config = appConfig(mode, toggles),
        )
    }

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
     */
    // 中文: 連續輸入候選查詢。position 固定為 0(Phase 6 dispatch 驗證)。
    // 中文: generation 必須沿用當前 composing session — 不可 bump,否則會在 fetch 前重置狀態。
    // 中文: frequencyEntries + nowMs 為 Phase 9.3a/9.3c 的 user_freq_boost / recency_rank 來源,
    // 中文: 預設空陣列 + 0 維持中性 boost,實際填充由 ComposingManager two-phase fetch 負責。
    //
    /**
     * v3.5.8 Phase 9 Item 12 — `customEntries` carries the platform's
     * `custom_dictionary.db` matches (raw stored `(roman, hanji)`
     * columns; DB stays native). Default `emptyList()` = no custom
     * matches / feature off — backward-compatible no-op. The engine
     * synthesizes a full-buffer candidate per entry and dedupes
     * `(roman, hanji)` against the FST hits (custom wins the
     * collision). Mirrors iOS `RustEngineBridge.composingFetchAtPos`.
     */
    // 中文: Item 12 — customEntries 帶平台 custom_dictionary.db 原始 (roman,hanji);預設空 = no-op。
    // v3.5.8 §10.2 platform pass: the FetchAtPos snapshot renders the
    // combined marked region (`combined_display`) and per-segment recased
    // candidates, so it needs the continuous spacing flags to match the
    // commit-time rendering. Defaults = v3.5.7 roman-first; production
    // callers pass explicit live values via continuousSpacingFlags.
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
    ): ContinuousFetchResult {
        val payload = com.siansiansu.taigikeyboard.engine.proto.FetchAtPos
            .newBuilder()
            .setPosition(0)
            .addAllFrequencyEntries(frequencyEntries)
            .setNowMs(nowMs)
            .addAllCustomEntries(customEntries)
            .build()
        return composingFetchDispatch(
            methodSetter = { it.fetchAtPos = payload },
            op = "composingFetchAtPos",
            generation = generation,
            config = continuousAppConfig(mode, toggles, effectiveSwapped, outputBothScripts),
        )
    }

    /**
     * Commit a candidate segment in `Phase::Continuous`. `displayText` /
     * `consumedBytes` / `syllableCount` MUST come from a [ContinuousCandidate]
     * returned by an immediately preceding [composingFetchAtPos] call —
     * sending mismatched values mis-aligns the committed segment.
     * `consumedBytes >= pending.utf8.size` triggers a final commit (exit
     * to Idle). Programmer-error inputs collapse to noop on the engine side.
     */
    // 中文: 連續輸入提交候選段。displayText / consumedBytes / syllableCount 必須與
    // 中文: 上一個 composingFetchAtPos 回傳的 ContinuousCandidate 對齊。
    // v3.5.8 §10.2 platform pass: the repro path. Mid-commit renders
    // `combined_display(nailed, pending, config)`; final-commit renders
    // `nailed_prefix(nailed, config)` — both need the spacing flags so
    // segments join with the right (roman: space / hanji-first: none /
    // both-scripts: space) word boundary. Defaults = v3.5.7 roman-first;
    // production callers pass explicit live values.
    @JvmStatic
    fun composingCommitContinuous(
        displayText: String,
        canonicalText: String,
        consumedBytes: Int,
        syllableCount: Int,
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
        generation: Long,
        effectiveSwapped: Boolean = false,
        outputBothScripts: Boolean = false,
    ): ComposingTransition {
        val payload = com.siansiansu.taigikeyboard.engine.proto.CommitContinuous
            .newBuilder()
            .setDisplayText(displayText)
            .setCanonicalText(canonicalText)
            .setConsumedBytes(consumedBytes)
            .setSyllableCount(syllableCount)
            .build()
        return composingDispatch(
            methodSetter = { it.commitContinuous = payload },
            op = "composingCommitContinuous",
            generation = generation,
            config = continuousAppConfig(mode, toggles, effectiveSwapped, outputBothScripts),
        )
    }

    /**
     * Abort continuous-input. Drops `Phase::Continuous` committed list +
     * pending raw, exits to Idle, emits the standard abort effect trio
     * (`ClearPreeditWithoutCommit` + `ResetAutocomplete` +
     * `NextWordClearForNewComposing`). Committed segments stay in the
     * document — earlier `CommitTextReplacingPreedit` effects already wrote
     * them.
     */
    // 中文: 連續輸入中止。pending 與 committed 一起丟,Phase 退回 Idle,發 abort 三 effects。
    @JvmStatic
    fun composingResetContinuous(generation: Long): ComposingTransition {
        val payload = com.siansiansu.taigikeyboard.engine.proto.ResetContinuous
            .newBuilder()
            .build()
        return composingDispatch(
            methodSetter = { it.resetContinuous = payload },
            op = "composingResetContinuous",
            generation = generation,
            config = null,
        )
    }

    // endregion

    /**
     * Encode → FFI roundtrip → decode for the composing slice. Returns the
     * raw `ComposingResponse` proto so callers that need access to the
     * `continuous` carrier (FetchAtPos) can reach it without a second
     * dispatch. Generation is passed through verbatim — composing-slice
     * generation bumping is owned by `ComposingManager.bumpGeneration()`,
     * not this layer.
     */
    // 中文: composing slice 的 FFI roundtrip,回傳原始 proto 供需要 continuous 載體的 caller(FetchAtPos)使用。
    private inline fun composingProtoRoundtrip(
        methodSetter: (com.siansiansu.taigikeyboard.engine.proto.ComposingRequest.Builder) -> Unit,
        op: String,
        generation: Long,
        config: AppConfig?,
    ): com.siansiansu.taigikeyboard.engine.proto.ComposingResponse? {
        val composingBuilder = com.siansiansu.taigikeyboard.engine.proto.ComposingRequest
            .newBuilder()
        methodSetter(composingBuilder)
        val requestBuilder = Request
            .newBuilder()
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
            return null
        }
        if (response.error != ErrorCode.OK) {
            recordFailure(op, "engine returned ${response.error}", response.error.number)
            return null
        }
        if (!response.hasComposing()) {
            recordFailure(op, "missing composing payload")
            return null
        }
        return response.composing
    }

    private inline fun composingDispatch(
        methodSetter: (com.siansiansu.taigikeyboard.engine.proto.ComposingRequest.Builder) -> Unit,
        op: String,
        generation: Long,
        config: AppConfig?,
    ): ComposingTransition {
        val payload = composingProtoRoundtrip(methodSetter, op, generation, config)
            ?: return ComposingTransition.NOOP
        val transition = synthComposing(payload)
        installedBackend.tdebug("RustEngineBridge") {
            "[FFI<-] fn=composingDispatch op=$op effects=${transition.effects.size} composing=${transition.isComposing}"
        }
        return transition
    }

    /**
     * Phase 6 FetchAtPos dispatcher. Synthesizes both the standard
     * [ComposingTransition] (for engine snapshot mirroring) and the
     * [ContinuousFetchResult.candidates] tri-state read off
     * `ComposingResponse.continuous`.
     */
    // 中文: Phase 6 FetchAtPos 專用分派 — 同時產生 ComposingTransition 與
    // 中文: ContinuousFetchResult.candidates(從 proto.continuous 三態解碼)。
    private inline fun composingFetchDispatch(
        methodSetter: (com.siansiansu.taigikeyboard.engine.proto.ComposingRequest.Builder) -> Unit,
        op: String,
        generation: Long,
        config: AppConfig?,
    ): ContinuousFetchResult {
        val payload = composingProtoRoundtrip(methodSetter, op, generation, config)
            ?: return ContinuousFetchResult.NOOP
        val transition = synthComposing(payload)
        val candidates: List<ContinuousCandidate>? = if (payload.hasContinuous()) {
            payload.continuous.candidatesList.map { msg ->
                // v3.5.8 Phase 9 Item 5 — `hanji` is proto3 `optional`;
                // protobuf-javalite exposes presence via `hasHanji()`.
                // Map absent → `null` (NOT empty string) so the
                // bridge data class's `hanji: String?` carries the
                // wire-absent distinction faithfully (TAILO candidate).
                //
                // Defensive `roman` fallback per
                // `docs/engine/continuous-candidate-display.md` §7 +
                // Codex pre-impl F4 verdict A: if `msg.roman` is
                // empty (old-Rust-new-platform wire skew, or proto
                // regen skipped), fall back to `displayText` so the
                // Item 6 dual-line render does not show a blank title
                // row. Bundled releases never hit this branch.
                // 中文: Item 5 — hanji 為 proto3 optional;wire absent → Kotlin null。
                // 中文: roman 防禦性 fallback — wire skew 時 displayText 兜底,避免空 title。
                val roman = if (msg.roman.isEmpty()) msg.displayText else msg.roman
                ContinuousCandidate(
                    consumedSpanStart = msg.consumedSpanStart,
                    consumedSpanEnd = msg.consumedSpanEnd,
                    syllableCount = msg.syllableCount,
                    displayText = msg.displayText,
                    score = msg.score,
                    form = msg.form,
                    mode = CandidateMode.decode(msg.modeValue),
                    roman = roman,
                    hanji = if (msg.hasHanji()) msg.hanji else null,
                )
            }
        } else {
            null
        }
        installedBackend.tdebug("RustEngineBridge") {
            val count = candidates?.size ?: -1
            "[FFI<-] fn=composingFetchDispatch op=$op effects=${transition.effects.size} candidates=$count"
        }
        return ContinuousFetchResult(
            transition = transition,
            candidates = candidates,
            isBridgeFailure = false,
        )
    }

    private fun synthComposing(
        proto: com.siansiansu.taigikeyboard.engine.proto.ComposingResponse,
    ): ComposingTransition {
        // Effect contract is exhaustive — every emitted Effect.kind maps to a
        // Kotlin case. The InputConnection-bound effects route through
        // DefaultComposingDelegate; the 3 NextWord-shaped effects route
        // through SmartbarManager.dispatchComposingNextWordEffect via the
        // NextWordEffectRouter sibling on ComposingManager.
        val effects: List<ComposingTransition.Effect> = proto.effectList.mapNotNull { eff ->
            when {
                eff.hasUpdatePreedit() -> {
                    ComposingTransition.Effect.UpdatePreedit(eff.updatePreedit.display)
                }

                eff.hasClearPreeditWithoutCommit() -> {
                    ComposingTransition.Effect.ClearPreeditWithoutCommit
                }

                eff.hasCommitTextReplacingPreedit() -> {
                    ComposingTransition.Effect.CommitTextReplacingPreedit(eff.commitTextReplacingPreedit.text)
                }

                eff.hasDeleteBackwardFromDocument() -> {
                    ComposingTransition.Effect.DeleteBackwardFromDocument
                }

                eff.hasResetAutocomplete() -> {
                    ComposingTransition.Effect.ResetAutocomplete
                }

                eff.hasPerformAutocomplete() -> {
                    ComposingTransition.Effect.PerformAutocomplete
                }

                eff.hasResetAutocompleteContext() -> {
                    ComposingTransition.Effect.ResetAutocompleteContext
                }

                eff.hasNextWordUpdateLastSelectedWord() -> {
                    ComposingTransition.Effect.NextWordUpdateLastSelectedWord(
                        text = eff.nextWordUpdateLastSelectedWord.text,
                        roman = eff.nextWordUpdateLastSelectedWord.roman,
                    )
                }

                eff.hasNextWordWordSelected() -> {
                    ComposingTransition.Effect.NextWordWordSelected(
                        text = eff.nextWordWordSelected.text,
                        roman = eff.nextWordWordSelected.roman,
                        triggerPrediction = eff.nextWordWordSelected.triggerPrediction,
                    )
                }

                eff.hasNextWordClearForNewComposing() -> {
                    ComposingTransition.Effect.NextWordClearForNewComposing
                }

                else -> {
                    null
                }
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

    // 中文: 使用者選定一個候選詞 — 觸發 association 紀錄、context 計時、可選的下個詞預測查詢。
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
        val payload = com.siansiansu.taigikeyboard.engine.proto.WordSelected
            .newBuilder()
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

    // 中文: 退格通知 — 視 lastChar 是否邊界字符決定是否清 NextWord 顯示與重排 timer。
    @JvmStatic
    fun nextwordBackspace(
        lastChar: String,
        nowMs: Long,
        mode: InputMode,
        translateSwapped: Boolean,
        associationRecordingEnabled: Boolean,
        generation: Long,
    ): NextWordDecideResult {
        val payload = com.siansiansu.taigikeyboard.engine.proto.Backspace
            .newBuilder()
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

    // 中文: context timeout 觸發 — 平台 timer 到時呼叫,Engine 視當下狀態決定是否清 NextWord UI。
    @JvmStatic
    fun nextwordContextTimeoutFired(
        nowMs: Long,
        mode: InputMode,
        translateSwapped: Boolean,
        associationRecordingEnabled: Boolean,
        generation: Long,
    ): NextWordDecideResult {
        val payload = com.siansiansu.taigikeyboard.engine.proto.ContextTimeoutFired
            .newBuilder()
            .setInput(decisionInput(nowMs))
            .build()
        return nextwordDecideDispatch(
            methodSetter = { it.contextTimeoutFired = payload },
            op = "nextwordContextTimeoutFired",
            generation = generation,
            config = nextwordConfig(mode, translateSwapped, associationRecordingEnabled),
        )
    }

    // 中文: 開始新 composing 時清掉 NextWord 顯示但保留 lastSelectedWord(下次選詞時仍能用)。
    @JvmStatic
    fun nextwordClearForNewComposing(
        nowMs: Long,
        mode: InputMode,
        translateSwapped: Boolean,
        associationRecordingEnabled: Boolean,
        generation: Long,
    ): NextWordDecideResult {
        val payload = com.siansiansu.taigikeyboard.engine.proto.ClearForNewComposing
            .newBuilder()
            .setInput(decisionInput(nowMs))
            .build()
        return nextwordDecideDispatch(
            methodSetter = { it.clearForNewComposing = payload },
            op = "nextwordClearForNewComposing",
            generation = generation,
            config = nextwordConfig(mode, translateSwapped, associationRecordingEnabled),
        )
    }

    // 中文: 完整重置 — 清 lastSelectedWord/lastSelectionTimeMs/isShowing,適用切焦點欄位 / 切 input mode 等情境。
    @JvmStatic
    fun nextwordResetFull(
        nowMs: Long,
        mode: InputMode,
        translateSwapped: Boolean,
        associationRecordingEnabled: Boolean,
        generation: Long,
    ): NextWordDecideResult {
        val payload = com.siansiansu.taigikeyboard.engine.proto.ResetFull
            .newBuilder()
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
     *
     * 中文: 平台 → engine 同步 NextWord UI 是否顯示中;讓 engine 後續 clear 路徑正確 gate ClearPredictionsUI Effect。
     */
    @JvmStatic
    fun nextwordSetIsShowing(
        isShowing: Boolean,
        mode: InputMode,
        translateSwapped: Boolean,
        associationRecordingEnabled: Boolean,
        generation: Long,
    ): NextWordDecideResult {
        val payload = com.siansiansu.taigikeyboard.engine.proto.SetIsShowing
            .newBuilder()
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
     *
     * 中文: Android 限定意圖(Space 路徑)— 只更新 lastSelectedWord、不重排 timer、不 bump generation;iOS 故意不做此 op。
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
        val payload = com.siansiansu.taigikeyboard.engine.proto.UpdateLastSelectedWord
            .newBuilder()
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

    // 中文: 把平台 SQL 撈到的原始 raw 預測列(dict + user)送進 Rust 做 score+merge+sort+limit;
    // 中文: queryGeneration 對不上 currentGeneration 時回 wasStale=true,呼叫端應丟棄。
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
        val builder = com.siansiansu.taigikeyboard.engine.proto.FilterPredictions
            .newBuilder()
            .setQueryGeneration(queryGeneration)
            .setNowMs(nowMs)
            .setLimit(limit)
        for (row in raw) {
            builder.addRaw(
                com.siansiansu.taigikeyboard.engine.proto.RawNextWordPrediction
                    .newBuilder()
                    .setHanzi(row.hanzi)
                    .setTl(row.tl)
                    .setCount(row.count)
                    .setLastUsedMs(row.lastUsedMs)
                    .setSource(
                        when (row.source) {
                            NextWordRawRow.Source.DICT -> com.siansiansu.taigikeyboard.engine.proto.Source.SOURCE_DICT
                            NextWordRawRow.Source.USER -> com.siansiansu.taigikeyboard.engine.proto.Source.SOURCE_USER
                        },
                    ).build(),
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

    // 中文: 用 NextWord 預測首字集合對 autocomplete 候選做重排 — 首字命中者上浮(autocomplete context booster)。
    @JvmStatic
    fun nextwordBoostCandidates(
        words: List<String>,
        predictedFirstChars: Set<String>,
        mode: InputMode,
        translateSwapped: Boolean,
        associationRecordingEnabled: Boolean,
        generation: Long,
    ): List<String> {
        val payload = com.siansiansu.taigikeyboard.engine.proto.BoostCandidates
            .newBuilder()
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

    // 中文: 純讀 — 取 NextWord 當前狀態(lastSelectedWord/isShowing/currentGeneration),不 mutate。
    @JvmStatic
    fun nextwordQueryState(
        mode: InputMode,
        translateSwapped: Boolean,
        associationRecordingEnabled: Boolean,
        generation: Long,
    ): NextWordStateSnapshot {
        val payload = com.siansiansu.taigikeyboard.engine.proto.NextWordQueryState
            .newBuilder()
            .build()
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
        com.siansiansu.taigikeyboard.engine.proto.DecisionInput
            .newBuilder()
            .setNowMs(nowMs)
            .build()

    private fun nextwordConfig(
        mode: InputMode,
        translateSwapped: Boolean,
        associationRecordingEnabled: Boolean,
    ): AppConfig =
        AppConfig
            .newBuilder()
            .setInputMode(
                when (mode) {
                    InputMode.POJ -> "poj"
                    InputMode.TL -> "tl"
                    InputMode.ENGLISH -> "english"
                },
            ).setOoDoubletapEnabled(false)
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
        val nextwordBuilder = com.siansiansu.taigikeyboard.engine.proto.NextWordRequest
            .newBuilder()
        methodSetter(nextwordBuilder)
        val request = Request
            .newBuilder()
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
                eff.hasRescheduleContextTimeout() -> {
                    NextWordDecideResult.Effect.RescheduleContextTimeout(eff.rescheduleContextTimeout.afterMs)
                }

                eff.hasCancelContextTimeout() -> {
                    NextWordDecideResult.Effect.CancelContextTimeout
                }

                eff.hasRecordAssociation() -> {
                    NextWordDecideResult.Effect.RecordAssociation(synthAssociationPair(eff.recordAssociation.pair))
                }

                eff.hasRecordCompoundAssociations() -> {
                    NextWordDecideResult.Effect.RecordCompoundAssociations(
                        eff.recordCompoundAssociations.pairsList.map(::synthAssociationPair),
                    )
                }

                eff.hasQueryPredictions() -> {
                    NextWordDecideResult.Effect.QueryPredictions(
                        word = eff.queryPredictions.word,
                        roman = eff.queryPredictions.roman,
                        generation = eff.queryPredictions.generation,
                        nowMs = eff.queryPredictions.nowMs,
                    )
                }

                eff.hasClearPredictionsUi() -> {
                    NextWordDecideResult.Effect.ClearPredictionsUI(eff.clearPredictionsUi.generation)
                }

                else -> {
                    null
                }
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
    ): NextWordAssociationPair =
        NextWordAssociationPair(
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
     *
     * 中文: 取目前累計的 FFI 失敗統計(總次數 + 最近 32 筆 entry);供 debug menu 與測試檢視。永不丟例外。
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
    // region Private dispatch + diagnostics

    private val diagnosticsLock = Any()
    private val failureCounter = AtomicInteger(0)
    private val recentErrors = ArrayDeque<DiagnosticsEntry>(RECENT_ERRORS_CAP)

    private fun recordFailure(
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

    private fun appConfig(
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
     */
    // 中文: 連續輸入渲染用 AppConfig — base appConfig + §10.2 字界空格兩旗標。
    // 中文: effectiveSwapped(翻譯反轉 OR TPS,平台端合併,因 NormalizeMode 無 TPS 且 TPS→"tl"
    // 中文: 故引擎 input_mode=="tps" 永不觸發)走 is_translate_swapped;outputBothScripts
    // 中文: 區分漢字優先(無空格)vs 雙腳本(要空格)。只用在會渲染 nailed prefix 的進入點。
    // CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Engine/RustEngineBridge.swift continuousAppConfig.
    // Drift causes silent divergence (hanji-first spurious word-boundary spaces).
    private fun continuousAppConfig(
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
        effectiveSwapped: Boolean,
        outputBothScripts: Boolean,
    ): AppConfig =
        appConfig(mode, toggles)
            .toBuilder()
            .setIsTranslateSwapped(effectiveSwapped)
            .setOutputBothScripts(outputBothScripts)
            .build()

    private inline fun dispatch(
        methodSetter: (PhoneticsRequest.Builder) -> Unit,
        op: String,
        config: AppConfig?,
    ): PhoneticsResponse? {
        val phoneticsBuilder = PhoneticsRequest.newBuilder()
        methodSetter(phoneticsBuilder)
        val requestBuilder = Request
            .newBuilder()
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
        val request = Request
            .newBuilder()
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

    private fun isDebugBuild(): Boolean =
        try {
            Class
                .forName("com.siansiansu.taigikeyboard.BuildConfig")
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
data class StripToneOutcome(
    val bare: String,
    val tone: String,
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
