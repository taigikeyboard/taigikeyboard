// 中文: Phonetics + Derivation + TPS 橋 — 15 ops + toneVariations 快取。
// 中文: 對應 iOS RustEngineBridge+Phonetics.swift(TPS 同檔合併 per simplify 結論)。
// 中文: 走 RustEngineBridge.sendRawBytes(...) 做 JNI roundtrip — 保留 pre-split facade
// 中文: dispatch 路徑的 parse-fail 行為(JNI exception 透傳、parseFrom 失敗回 null)。
// 中文: 不要改成 dispatchRaw 包 try/Throwable,否則 op-name recordFailure 與例外語意都會偏。

package com.siansiansu.taigikeyboard.engine

import com.siansiansu.taigikeyboard.engine.proto.AppConfig
import com.siansiansu.taigikeyboard.engine.proto.BoolResult
import com.siansiansu.taigikeyboard.engine.proto.ContainsTps
import com.siansiansu.taigikeyboard.engine.proto.CustomSearchKeysResult
import com.siansiansu.taigikeyboard.engine.proto.DeriveAbbrev
import com.siansiansu.taigikeyboard.engine.proto.DeriveCustomQueryKey
import com.siansiansu.taigikeyboard.engine.proto.DeriveCustomSearchKeys
import com.siansiansu.taigikeyboard.engine.proto.DeriveNotone
import com.siansiansu.taigikeyboard.engine.proto.ErrorCode
import com.siansiansu.taigikeyboard.engine.proto.GetToneVariations
import com.siansiansu.taigikeyboard.engine.proto.IsTpsToneMark
import com.siansiansu.taigikeyboard.engine.proto.NfdPreprocessForLookup
import com.siansiansu.taigikeyboard.engine.proto.NormalizeInput
import com.siansiansu.taigikeyboard.engine.proto.NormalizeToTl
import com.siansiansu.taigikeyboard.engine.proto.NormalizeTone
import com.siansiansu.taigikeyboard.engine.proto.OptionalStringResult
import com.siansiansu.taigikeyboard.engine.proto.PhoneticsRequest
import com.siansiansu.taigikeyboard.engine.proto.PhoneticsResponse
import com.siansiansu.taigikeyboard.engine.proto.PojToTl
import com.siansiansu.taigikeyboard.engine.proto.Request
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

/**
 * Impl object backing `RustEngineBridge` phonetics / derivation / TPS facade
 * methods. NOT a public API — callers stay on `RustEngineBridge.*` per F1=A
 * facade contract. Module-internal visibility keeps the surface honest.
 *
 * Lazy `toneVariations` cache lives here so the FFI roundtrip + cache share
 * the same dispatcher; facade exposes `RustEngineBridge.toneVariations` as
 * a thin getter forward.
 */
internal object PhoneticsBridge {
    // region Phonetics core (8 ops)

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
            config = RustEngineBridge.appConfig(mode, toggles),
        )
    }

    fun stripTone(input: String): StripToneOutcome {
        val payload = StripTone.newBuilder().setInput(input).build()
        val resp = dispatch({ it.stripTone = payload }, "stripTone", null)
            ?: return StripToneOutcome(input, "")
        if (!resp.hasStripToneResult()) {
            RustEngineBridge.recordFailure("stripTone", "missing StripToneResult")
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
            RustEngineBridge.recordFailure("restoreTone", "missing OptionalStringResult")
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
            RustEngineBridge.recordFailure("getToneVariations", "missing ToneVariationsResult")
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

    /**
     * `Method::DeriveCustomSearchKeys` — WRITE side (v3.6.1 R3). Full
     * {tl, poj, tps} × {num, notone, abbrev} (+ TPS er/or variant) bundle for
     * a stored custom-dict roman, materialized into the `custom_search_key`
     * side table. Empty on FFI failure / residue-only input.
     */
    // 中文: 自訂詞寫入端 — 把 roman 展成跨家族搜尋鍵 bundle,落地到 custom_search_key 側表。
    fun deriveCustomSearchKeys(roman: String): List<CustomSearchKey> {
        val payload = DeriveCustomSearchKeys.newBuilder().setRoman(roman).build()
        return customSearchKeys({ it.deriveCustomSearchKeys = payload }, "deriveCustomSearchKeys")
    }

    /**
     * `Method::DeriveCustomQueryKey` — READ side (v3.6.1 R3). Single
     * family-native key for the current raw `input` + `inputMode` string. The
     * engine upgrades the family to TPS when the raw input carries Bopomofo,
     * so the caller passes its settings mode verbatim. `null` for residue-only
     * / empty input or FFI failure.
     */
    // 中文: 自訂詞查詢端 — 依 input + inputMode 產生單一家族鍵;raw 含注音時引擎自動升 tps 家族。
    fun deriveCustomQueryKey(
        input: String,
        inputMode: String,
    ): CustomSearchKey? {
        val payload = DeriveCustomQueryKey
            .newBuilder()
            .setInput(input)
            .setInputMode(inputMode)
            .build()
        return customSearchKeys({ it.deriveCustomQueryKey = payload }, "deriveCustomQueryKey").firstOrNull()
    }

    // endregion
    // region TPS (5 ops)

    fun containsTps(text: String): Boolean {
        val payload = ContainsTps.newBuilder().setText(text).build()
        return boolDispatch({ it.containsTps = payload }, "containsTps")
    }

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

    fun isTpsToneMark(char: Char): Boolean {
        val payload = IsTpsToneMark.newBuilder().setChar(char.toString()).build()
        return boolDispatch({ it.isTpsToneMark = payload }, "isTpsToneMark")
    }

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
            RustEngineBridge.recordFailure("tpsInputAdjust", "missing TpsAdjustResult")
            return TpsAdjustOutcome(incoming, null)
        }
        val r: TpsAdjustResult = resp.tpsAdjustResult
        val replace = if (r.hasReplaceLast() && r.replaceLast.present) r.replaceLast.output else null
        return TpsAdjustOutcome(r.adjusted, replace)
    }

    // endregion
    // region Private dispatch

    private inline fun dispatch(
        methodSetter: (PhoneticsRequest.Builder) -> Unit,
        op: String,
        config: AppConfig?,
    ): PhoneticsResponse? {
        val phoneticsBuilder = PhoneticsRequest.newBuilder()
        methodSetter(phoneticsBuilder)
        val requestBuilder = Request
            .newBuilder()
            .setId(RustEngineBridge.nextRequestIdInternal())
            .setPhonetics(phoneticsBuilder.build())
        if (config != null) {
            requestBuilder.configSnapshot = config
        }
        val response = RustEngineBridge.sendRawBytes(requestBuilder.build().toByteArray())
        if (response == null) {
            RustEngineBridge.recordFailure(op, "response decode failed")
            return null
        }
        if (response.error != ErrorCode.OK) {
            RustEngineBridge.recordFailure(op, "engine returned ${response.error}", response.error.number)
            return null
        }
        if (!response.hasPhonetics()) {
            RustEngineBridge.recordFailure(op, "missing phonetics payload")
            return null
        }
        return response.phonetics
    }

    private inline fun stringDispatch(
        methodSetter: (PhoneticsRequest.Builder) -> Unit,
        input: String,
        op: String,
        config: AppConfig?,
    ): String {
        val resp = dispatch(methodSetter, op, config) ?: return input
        if (!resp.hasStringResult()) {
            RustEngineBridge.recordFailure(op, "expected StringResult")
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
            RustEngineBridge.recordFailure(op, "expected BoolResult")
            return false
        }
        val r: BoolResult = resp.boolResult
        return r.value
    }

    /**
     * Shared decode for the two custom-dict search-key ops — both return a
     * `CustomSearchKeysResult` (the write op a full bundle, the query op 0/1).
     * Mirrors iOS `RustEngineBridge+Phonetics.swift` `customSearchKeys`.
     */
    // 中文: 兩個自訂詞搜尋鍵 op 共用的 decode;皆回 CustomSearchKeysResult。
    private inline fun customSearchKeys(
        methodSetter: (PhoneticsRequest.Builder) -> Unit,
        op: String,
    ): List<CustomSearchKey> {
        val resp = dispatch(methodSetter, op, null) ?: return emptyList()
        if (!resp.hasCustomSearchKeysResult()) {
            RustEngineBridge.recordFailure(op, "expected CustomSearchKeysResult")
            return emptyList()
        }
        val r: CustomSearchKeysResult = resp.customSearchKeysResult
        return r.keysList.map { CustomSearchKey(family = it.family, form = it.form, key = it.key) }
    }

    // endregion
}
