// Phonetics + TPS ops — extensions on RustEngineBridge.
// Mirrors iOS RustEngineBridge+Phonetics.swift (TPS merged into same file per simplify decision).
// Sends through RustEngineBridge.dispatch(op) { … } — shared JNI hop, exception boundary, recordFailure sink.

package com.siansiansu.taigikeyboard.engine

import com.siansiansu.taigikeyboard.engine.proto.BoolResult
import com.siansiansu.taigikeyboard.engine.proto.DoubleSpacePeriod
import com.siansiansu.taigikeyboard.engine.proto.ExternalLookupDigitForm
import com.siansiansu.taigikeyboard.engine.proto.GetToneVariations
import com.siansiansu.taigikeyboard.engine.proto.IsAttachingPunctuation
import com.siansiansu.taigikeyboard.engine.proto.IsJoinerPunctuation
import com.siansiansu.taigikeyboard.engine.proto.IsSpacedPunctuation
import com.siansiansu.taigikeyboard.engine.proto.PhoneticsRequest
import com.siansiansu.taigikeyboard.engine.proto.PhoneticsResponse
import com.siansiansu.taigikeyboard.engine.proto.StringResult

// region Phonetics DTOs

/**
 * The engine's long-press tone variations by key label (`a`, `oo`, `Ng` …), per script, plus
 * [universal]: both scripts' lists interleaved, both tone-9 marks. iOS `ToneVariationsCache`.
 */
data class ToneVariations(
    val tl: Map<String, List<String>>,
    val poj: Map<String, List<String>>,
    val universal: Map<String, List<String>>,
) {
    companion object {
        val EMPTY = ToneVariations(tl = emptyMap(), poj = emptyMap(), universal = emptyMap())
    }
}

// endregion

// region Phonetics core

/**
 * The long-press tone variations, pulled from the engine on first use (`GetToneVariations`) and kept;
 * empty when the engine fails, as on iOS (`RustEngineBridge.toneVariations`). A synchronized lazy:
 * layouts compute on both the main thread and Dispatchers.IO.
 */
val RustEngineBridge.toneVariations: ToneVariations by lazy {
    val op = "getToneVariations"
    val response = phoneticsDispatch({ it.getToneVariations = GetToneVariations.getDefaultInstance() }, op)
        ?: return@lazy ToneVariations.EMPTY
    if (!response.hasToneVariationsResult()) {
        RustEngineBridge.recordFailure(op, "expected ToneVariationsResult")
        return@lazy ToneVariations.EMPTY
    }
    val result = response.toneVariationsResult
    ToneVariations(
        tl = result.tlVariationsMap.mapValues { it.value.variationsList.toList() },
        poj = result.pojVariationsMap.mapValues { it.value.variationsList.toList() },
        universal = result.universalVariationsMap.mapValues { it.value.variationsList.toList() },
    )
}

// endregion
// region Platform text helpers

/**
 * Whether [text] is a single punctuation character that attaches to the
 * preceding word under auto-space (`guá ` + `?` → `guá? `) — the engine's
 * set, the one every platform asks. False when the engine fails: no swap,
 * the space stays where it is.
 */
fun RustEngineBridge.isAttachingPunctuation(text: String): Boolean {
    val payload = IsAttachingPunctuation.newBuilder().setText(text).build()
    return boolDispatch({ it.isAttachingPunctuation = payload }, "isAttachingPunctuation")
}

/**
 * Whether [text] is the khinsiann dot, which takes the place of an auto space
 * (`hōo ` + `·` → `hōo·`) — the engine's answer. False when the engine fails:
 * the space stays where it is.
 */
fun RustEngineBridge.isJoinerPunctuation(text: String): Boolean {
    val payload = IsJoinerPunctuation.newBuilder().setText(text).build()
    return boolDispatch({ it.isJoinerPunctuation = payload }, "isJoinerPunctuation")
}

/**
 * Whether attaching [text] keeps the auto space after itself — a half-width
 * mark (`guá? `); a full-width one takes the space's place (`taigi，`) — the
 * engine's answer. False when the engine fails: the mark is written with no
 * space after it.
 */
fun RustEngineBridge.isSpacedPunctuation(text: String): Boolean {
    val payload = IsSpacedPunctuation.newBuilder().setText(text).build()
    return boolDispatch({ it.isSpacedPunctuation = payload }, "isSpacedPunctuation")
}

/**
 * What a second space press writes in place of the first space (`. ` or
 * `。`), or null for a plain space — the engine's rule and window. Null when
 * the engine fails: a plain space.
 */
fun RustEngineBridge.doubleSpacePeriod(
    textBeforeCaret: String,
    millisSincePlainSpace: Long,
    fullWidth: Boolean,
): String? {
    val payload =
        DoubleSpacePeriod
            .newBuilder()
            .setTextBeforeCaret(textBeforeCaret)
            .setMillisSincePlainSpace(millisSincePlainSpace)
            .setFullWidth(fullWidth)
            .build()
    return stringDispatch({ it.doubleSpacePeriod = payload }, "", "doubleSpacePeriod").ifEmpty { null }
}

/**
 * The digit-tone spelling of a TL reading that the web dictionaries search
 * by (`tāi-tsì` → `tai7-tsi3`) — the engine's fold, the one every platform
 * asks. The reading as written when the engine fails.
 */
fun RustEngineBridge.externalLookupDigitForm(reading: String): String {
    val payload = ExternalLookupDigitForm.newBuilder().setReading(reading).build()
    return stringDispatch({ it.externalLookupDigitForm = payload }, reading, "externalLookupDigitForm")
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
