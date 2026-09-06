// Case-transform ops — extensions on RustEngineBridge that delegate char/string case handling
// (including POJ/TL tone-mark case mapping) to Rust `phonetics::case_transform`; iOS counterpart is
// `RustEngineBridge+CaseTransform.swift`. Single FFI hop per per-char or per-word case operation.
// Mode is forwarded via envelope `AppConfig.input_mode`; case-transform is independent of POJ
// doubletap preprocessing so the toggles fields are left at default. Suggestion skip rules
// (`id < 0 && id != -2` and `id == 0`) stay platform-side — only transform-eligible items reach
// `transformSuggestion(...)`. The `LetterCase` indicator lives on `RustEngineBridge.LetterCase`.

package com.siansiansu.taigikeyboard.engine

import com.siansiansu.taigikeyboard.engine.proto.AppConfig
import com.siansiansu.taigikeyboard.engine.proto.CaseRequest
import com.siansiansu.taigikeyboard.engine.proto.CaseResponse
import com.siansiansu.taigikeyboard.engine.proto.FullUppercaseToneString
import com.siansiansu.taigikeyboard.engine.proto.LowercaseToneChar
import com.siansiansu.taigikeyboard.engine.proto.TransformInputCase
import com.siansiansu.taigikeyboard.engine.proto.TransformSuggestion
import com.siansiansu.taigikeyboard.engine.proto.UppercaseToneChar
import com.siansiansu.taigikeyboard.ime.core.settings.InputMode
import com.siansiansu.taigikeyboard.engine.proto.LetterCase as ProtoLetterCase

private const val TAG = "CaseTransformBridge"

// region Per-char helpers

/**
 * Replaces `ToneUtilities.uppercaseToneLetter`. A single char (including its combining mark)
 * uppercases via the mode's tone table; multi-char input uppercases only the first character.
 */
fun RustEngineBridge.uppercaseToneChar(
    input: String,
    mode: InputMode,
): String {
    val payload = UppercaseToneChar.newBuilder().setInput(input).build()
    return caseStringDispatch(
        CaseRequest.newBuilder().setUppercaseToneChar(payload).build(),
        op = "uppercaseToneChar",
        mode = mode,
        fallback = input,
    )
}

/**
 * Replaces `ToneUtilities.fullUppercaseToneLetter`. Uppercases the whole string via the mode's
 * tone table; used by the CapsLock path.
 */
fun RustEngineBridge.fullUppercaseToneString(
    input: String,
    mode: InputMode,
): String {
    val payload = FullUppercaseToneString.newBuilder().setInput(input).build()
    return caseStringDispatch(
        CaseRequest.newBuilder().setFullUppercaseToneString(payload).build(),
        op = "fullUppercaseToneString",
        mode = mode,
        fallback = input,
    )
}

/**
 * Replaces `ToneUtilities.lowercaseToneLetter`. Lowercases a single char via the mode's tone
 * table, including the ᴺ→ⁿ nasal-marker shortcut.
 */
fun RustEngineBridge.lowercaseToneChar(
    input: String,
    mode: InputMode,
): String {
    val payload = LowercaseToneChar.newBuilder().setInput(input).build()
    return caseStringDispatch(
        CaseRequest.newBuilder().setLowercaseToneChar(payload).build(),
        op = "lowercaseToneChar",
        mode = mode,
        fallback = input,
    )
}

// endregion

// region Per-string compound transforms

/**
 * Apply `letterCase` to `text` per the engine's input-case pipeline. Used by `KeyLabelCaseCache`.
 */
fun RustEngineBridge.transformInputCase(
    text: String,
    letterCase: RustEngineBridge.LetterCase,
    mode: InputMode,
): String {
    val payload = TransformInputCase
        .newBuilder()
        .setText(text)
        .setLetterCase(ProtoLetterCase.forNumber(letterCase.protoValue) ?: ProtoLetterCase.LETTER_CASE_UNSPECIFIED)
        .build()
    return caseStringDispatch(
        CaseRequest.newBuilder().setTransformInputCase(payload).build(),
        op = "transformInputCase",
        mode = mode,
        fallback = text,
    )
}

/**
 * Per-suggestion case transformation. Output is post-processed via
 * engine-side `adjust_nasal_marker_case` (no separate FFI hop needed).
 * CapsLock uppercases everything; otherwise the candidate is split at the composing length —
 * typed portion matches the typed case, remaining portion is title- or lower-cased.
 */
fun RustEngineBridge.transformSuggestion(
    original: String,
    composing: String,
    letterCase: RustEngineBridge.LetterCase,
    mode: InputMode,
): String {
    val payload = TransformSuggestion
        .newBuilder()
        .setOriginalText(original)
        .setComposingText(composing)
        .setLetterCase(ProtoLetterCase.forNumber(letterCase.protoValue) ?: ProtoLetterCase.LETTER_CASE_UNSPECIFIED)
        .build()
    return caseStringDispatch(
        CaseRequest.newBuilder().setTransformSuggestion(payload).build(),
        op = "transformSuggestion",
        mode = mode,
        fallback = original,
    )
}

// endregion

// region Private dispatch

private fun caseStringDispatch(
    caseRequest: CaseRequest,
    op: String,
    mode: InputMode,
    fallback: String,
): String {
    val resp = caseDispatch(caseRequest, op, mode) ?: return fallback
    if (!resp.hasStringResult()) {
        RustEngineBridge.backend.w(TAG, "[$op] missing string_result")
        return fallback
    }
    return resp.stringResult.output
}

private fun caseDispatch(
    caseRequest: CaseRequest,
    op: String,
    mode: InputMode,
): CaseResponse? {
    val response = RustEngineBridge.dispatch(op) {
        setConfigSnapshot(caseAppConfig(mode))
        setCaseTransform(caseRequest)
    } ?: return null
    return if (response.hasCaseTransform()) response.caseTransform else null
}

private fun caseAppConfig(mode: InputMode): AppConfig =
    AppConfig
        .newBuilder()
        .setInputMode(
            when (mode) {
                InputMode.POJ -> "poj"
                InputMode.TL -> "tl"
                InputMode.ENGLISH -> "english"
            },
        ).build()

// endregion
