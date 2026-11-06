// Phonetics + TPS ops — extensions on RustEngineBridge.
// Mirrors iOS RustEngineBridge+Phonetics.swift (TPS merged into same file per simplify decision).
// Sends through RustEngineBridge.dispatch(op) { … } — shared JNI hop, exception boundary, recordFailure sink.

package com.siansiansu.taigikeyboard.engine

import com.siansiansu.taigikeyboard.engine.proto.BoolResult
import com.siansiansu.taigikeyboard.engine.proto.ExternalLookupDigitForm
import com.siansiansu.taigikeyboard.engine.proto.IsAttachingPunctuation
import com.siansiansu.taigikeyboard.engine.proto.IsJoinerPunctuation
import com.siansiansu.taigikeyboard.engine.proto.IsTpsToneMark
import com.siansiansu.taigikeyboard.engine.proto.PhoneticsRequest
import com.siansiansu.taigikeyboard.engine.proto.PhoneticsResponse
import com.siansiansu.taigikeyboard.engine.proto.StringResult
import com.siansiansu.taigikeyboard.engine.proto.TlDisplayToTps
import com.siansiansu.taigikeyboard.engine.proto.TlToPoj
import com.siansiansu.taigikeyboard.engine.proto.TpsAdjustResult
import com.siansiansu.taigikeyboard.engine.proto.TpsInputAdjust

// region Phonetics core

fun RustEngineBridge.tlToPoj(input: String): String {
    val payload = TlToPoj.newBuilder().setInput(input).build()
    return stringDispatch({ it.tlToPoj = payload }, input, "tlToPoj")
}

// endregion
// region TPS

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
