// Case-transform ops — extensions on RustEngineBridge that delegate char/string case handling
// (including POJ/TL tone-mark case mapping) to Rust `phonetics::case_transform`; iOS counterpart is
// `RustEngineBridge+CaseTransform.swift`. Single FFI hop per per-char or per-string case operation.
// Mode and ⁿ-becomes-ᴺ-in-capitals (§53) are forwarded via the envelope `AppConfig`; case-transform is independent
// of POJ doubletap preprocessing so the fold fields are left off. The `LetterCase` indicator is defined
// below.

package com.siansiansu.taigikeyboard.engine

import com.siansiansu.taigikeyboard.engine.proto.AppConfig
import com.siansiansu.taigikeyboard.engine.proto.CaseRequest
import com.siansiansu.taigikeyboard.engine.proto.CaseResponse
import com.siansiansu.taigikeyboard.engine.proto.FullUppercaseToneString
import com.siansiansu.taigikeyboard.engine.proto.LowercaseToneChar
import com.siansiansu.taigikeyboard.engine.proto.TransformInputCase
import com.siansiansu.taigikeyboard.engine.proto.UppercaseToneChar
import com.siansiansu.taigikeyboard.ime.settings.InputMode
import com.siansiansu.taigikeyboard.ime.settings.PojMarkerOptions
import com.siansiansu.taigikeyboard.engine.proto.LetterCase as ProtoLetterCase

private const val TAG = "CaseTransformBridge"

// region Case-transform DTO

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

// region Per-char helpers

/**
 * Replaces `ToneUtilities.uppercaseToneLetter`. A single char (including its combining mark)
 * uppercases via the mode's tone table; multi-char input uppercases only the first character.
 */
fun RustEngineBridge.uppercaseToneChar(
    input: String,
    mode: InputMode,
    isNasalMarkerUppercaseEnabled: Boolean,
): String {
    val payload = UppercaseToneChar.newBuilder().setInput(input).build()
    return caseStringDispatch(
        CaseRequest.newBuilder().setUppercaseToneChar(payload).build(),
        op = "uppercaseToneChar",
        mode = mode,
        isNasalMarkerUppercaseEnabled = isNasalMarkerUppercaseEnabled,
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
    isNasalMarkerUppercaseEnabled: Boolean,
): String {
    val payload = FullUppercaseToneString.newBuilder().setInput(input).build()
    return caseStringDispatch(
        CaseRequest.newBuilder().setFullUppercaseToneString(payload).build(),
        op = "fullUppercaseToneString",
        mode = mode,
        isNasalMarkerUppercaseEnabled = isNasalMarkerUppercaseEnabled,
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
    isNasalMarkerUppercaseEnabled: Boolean,
): String {
    val payload = LowercaseToneChar.newBuilder().setInput(input).build()
    return caseStringDispatch(
        CaseRequest.newBuilder().setLowercaseToneChar(payload).build(),
        op = "lowercaseToneChar",
        mode = mode,
        isNasalMarkerUppercaseEnabled = isNasalMarkerUppercaseEnabled,
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
    letterCase: LetterCase,
    mode: InputMode,
    isNasalMarkerUppercaseEnabled: Boolean,
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
        isNasalMarkerUppercaseEnabled = isNasalMarkerUppercaseEnabled,
        fallback = text,
    )
}

// endregion

// region Private dispatch

private fun caseStringDispatch(
    caseRequest: CaseRequest,
    op: String,
    mode: InputMode,
    isNasalMarkerUppercaseEnabled: Boolean,
    fallback: String,
): String {
    val resp = caseDispatch(caseRequest, op, mode, isNasalMarkerUppercaseEnabled) ?: return fallback
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
    isNasalMarkerUppercaseEnabled: Boolean,
): CaseResponse? {
    val response = RustEngineBridge.dispatch(op) {
        setConfigSnapshot(caseAppConfig(mode, isNasalMarkerUppercaseEnabled))
        setCaseTransform(caseRequest)
    } ?: return null
    return if (response.hasCaseTransform()) response.caseTransform else null
}

// The one canonical factory, folds off: case-transform never preprocesses `oo` / `nn`.
private fun caseAppConfig(
    mode: InputMode,
    isNasalMarkerUppercaseEnabled: Boolean,
): AppConfig =
    appConfig(
        inputMode = mode.engineInputMode(),
        pojMarkers = PojMarkerOptions(
            isDoubleTapOOEnabled = false,
            isDoubleTapNNEnabled = false,
            isNasalMarkerUppercaseEnabled = isNasalMarkerUppercaseEnabled,
        ),
    )

// endregion
