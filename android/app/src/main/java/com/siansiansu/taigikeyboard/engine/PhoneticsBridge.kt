// Phonetics + TPS ops — extensions on RustEngineBridge + the toneVariations cache.
// Mirrors iOS RustEngineBridge+Phonetics.swift (TPS merged into same file per simplify decision).
// Sends through RustEngineBridge.dispatch(op) { … } — shared JNI hop, exception boundary, recordFailure sink.

package com.siansiansu.taigikeyboard.engine

import com.siansiansu.taigikeyboard.engine.proto.BoolResult
import com.siansiansu.taigikeyboard.engine.proto.GetToneVariations
import com.siansiansu.taigikeyboard.engine.proto.IsTpsToneMark
import com.siansiansu.taigikeyboard.engine.proto.NfdPreprocessForLookup
import com.siansiansu.taigikeyboard.engine.proto.PhoneticsRequest
import com.siansiansu.taigikeyboard.engine.proto.PhoneticsResponse
import com.siansiansu.taigikeyboard.engine.proto.StringResult
import com.siansiansu.taigikeyboard.engine.proto.StripTone
import com.siansiansu.taigikeyboard.engine.proto.StripToneResult
import com.siansiansu.taigikeyboard.engine.proto.TlDisplayToTps
import com.siansiansu.taigikeyboard.engine.proto.TlNumericToTps
import com.siansiansu.taigikeyboard.engine.proto.TlToPoj
import com.siansiansu.taigikeyboard.engine.proto.ToneVariationsResult
import com.siansiansu.taigikeyboard.engine.proto.TpsAdjustResult
import com.siansiansu.taigikeyboard.engine.proto.TpsInputAdjust

// region Phonetics core (6 ops)

// Strips the syllable's tone combining mark; tone is "" when the syllable has none.
fun RustEngineBridge.stripTone(input: String): StripToneOutcome {
    val payload = StripTone.newBuilder().setInput(input).build()
    val resp = phoneticsDispatch({ it.stripTone = payload }, "stripTone")
        ?: return StripToneOutcome(input, "")
    if (!resp.hasStripToneResult()) {
        RustEngineBridge.recordFailure("stripTone", "missing StripToneResult")
        return StripToneOutcome(input, "")
    }
    val r: StripToneResult = resp.stripToneResult
    return StripToneOutcome(r.bare, r.tone)
}

fun RustEngineBridge.tlToPoj(input: String): String {
    val payload = TlToPoj.newBuilder().setInput(input).build()
    return stringDispatch({ it.tlToPoj = payload }, input, "tlToPoj")
}

/**
 * Replaces platform `TaigiUnicode.nfdPreprocessed(...)`. Lookup-side
 * NFD prep used by `ExternalLookupURLBuilder` before tone stripping.
 * It preserves tone diacritics; only nasal markers (ⁿ / ᴺ → "nn") and standalone
 * `\u{0358}` → `o` are rewritten.
 */
fun RustEngineBridge.nfdPreprocessForLookup(input: String): String {
    val payload = NfdPreprocessForLookup.newBuilder().setInput(input).build()
    return stringDispatch(
        { it.nfdPreprocessForLookup = payload },
        input,
        "nfdPreprocessForLookup",
    )
}

/**
 * Residual state holder — every phonetics op is an extension on
 * `RustEngineBridge` above; only the lazy cache needs an owner.
 */
internal object PhoneticsBridge {
    /**
     * Lazy-init cache for Method::GetToneVariations. Kotlin `by lazy` defaults
     * to `LazyThreadSafetyMode.SYNCHRONIZED` — single execution + thread
     * safety guaranteed by language semantics. First reader pays the FFI
     * roundtrip; subsequent reads are zero-FFI.
     */
    val toneVariations: ToneVariationsCache by lazy {
        val payload = GetToneVariations.newBuilder().build()
        val resp = phoneticsDispatch({ it.getToneVariations = payload }, "getToneVariations")
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
}

// endregion
// region TPS (4 ops)

// orMapsToER selects the er/or variant mapping.
fun RustEngineBridge.tlNumericToTps(
    text: String,
    orMapsToER: Boolean,
): String {
    val payload = TlNumericToTps
        .newBuilder()
        .setText(text)
        .setOrMapsToEr(orMapsToER)
        .build()
    return stringDispatch({ it.tlNumericToTps = payload }, text, "tlNumericToTps")
}

fun RustEngineBridge.tlDisplayToTps(
    text: String,
    orMapsToER: Boolean,
): String {
    val payload = TlDisplayToTps
        .newBuilder()
        .setText(text)
        .setOrMapsToEr(orMapsToER)
        .build()
    return stringDispatch({ it.tlDisplayToTps = payload }, text, "tlDisplayToTps")
}

fun RustEngineBridge.isTpsToneMark(char: Char): Boolean {
    val payload = IsTpsToneMark.newBuilder().setChar(char.toString()).build()
    return boolDispatch({ it.isTpsToneMark = payload }, "isTpsToneMark")
}

// Key-level TPS adjust: when replaceLast is non-empty the caller must replace the previous char with it.
fun RustEngineBridge.tpsInputAdjust(
    incoming: String,
    rawInput: String,
): TpsAdjustOutcome {
    val payload = TpsInputAdjust
        .newBuilder()
        .setIncoming(incoming)
        .setRawInput(rawInput)
        .build()
    val resp = phoneticsDispatch({ it.tpsInputAdjust = payload }, "tpsInputAdjust")
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

private inline fun phoneticsDispatch(
    methodSetter: (PhoneticsRequest.Builder) -> Unit,
    op: String,
): PhoneticsResponse? {
    val phoneticsBuilder = PhoneticsRequest.newBuilder()
    methodSetter(phoneticsBuilder)
    val phoneticsRequest = phoneticsBuilder.build()
    val response = RustEngineBridge.dispatch(op) {
        setPhonetics(phoneticsRequest)
    } ?: return null
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
): String {
    val resp = phoneticsDispatch(methodSetter, op) ?: return input
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
    val resp = phoneticsDispatch(methodSetter, op) ?: return false
    if (!resp.hasBoolResult()) {
        RustEngineBridge.recordFailure(op, "expected BoolResult")
        return false
    }
    val r: BoolResult = resp.boolResult
    return r.value
}

// endregion
