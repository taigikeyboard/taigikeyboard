// Case-transform ops — extensions on RustEngineBridge that delegate char/string case handling
// (including POJ/TL tone-mark case mapping) to Rust `phonetics::case_transform`; iOS counterpart is
// `RustEngineBridge+CaseTransform.swift`. Single FFI hop per per-char or per-string case operation.
// Casing is the same in every input mode; the envelope `AppConfig` carries only ⁿ-becomes-ᴺ-in-capitals (§53),
// the one field the engine's case handler reads. The `LetterCase` indicator is defined below.

package com.siansiansu.taigikeyboard.engine

import com.siansiansu.taigikeyboard.engine.proto.AppConfig
import com.siansiansu.taigikeyboard.engine.proto.CaseRequest
import com.siansiansu.taigikeyboard.engine.proto.CaseResponse
import com.siansiansu.taigikeyboard.engine.proto.ToneCase
import com.siansiansu.taigikeyboard.engine.proto.ToneCaseKind
import com.siansiansu.taigikeyboard.engine.proto.TransformInputCase
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

// region Per-char helper

/**
 * Cases `input` per `kind` (proto `ToneCaseKind`, same in every mode): `UPPERCASE_FIRST`
 * uppercases only the first character, `UPPERCASE_ALL` the whole string (CapsLock path),
 * `LOWERCASE` lowercases, including the ᴺ→ⁿ nasal-marker shortcut.
 */
fun RustEngineBridge.toneCase(
    input: String,
    kind: ToneCaseKind,
    isNasalMarkerUppercaseEnabled: Boolean,
): String {
    val payload = ToneCase
        .newBuilder()
        .setInput(input)
        .setKind(kind)
        .build()
    return caseStringDispatch(
        CaseRequest.newBuilder().setToneCase(payload).build(),
        op = "toneCase($kind)",
        isNasalMarkerUppercaseEnabled = isNasalMarkerUppercaseEnabled,
        fallback = input,
    )
}

// endregion

// region Per-string compound transforms

/**
 * Apply `letterCase` to `text` per the engine's input-case pipeline. Used by `TextInputKeyHandler`.
 */
fun RustEngineBridge.transformInputCase(
    text: String,
    letterCase: LetterCase,
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
        isNasalMarkerUppercaseEnabled = isNasalMarkerUppercaseEnabled,
        fallback = text,
    )
}

// endregion

// region Private dispatch

private fun caseStringDispatch(
    caseRequest: CaseRequest,
    op: String,
    isNasalMarkerUppercaseEnabled: Boolean,
    fallback: String,
): String {
    val resp = caseDispatch(caseRequest, op, isNasalMarkerUppercaseEnabled) ?: return fallback
    if (!resp.hasStringResult()) {
        RustEngineBridge.backend.w(TAG, "[$op] missing string_result")
        return fallback
    }
    return resp.stringResult.output
}

private fun caseDispatch(
    caseRequest: CaseRequest,
    op: String,
    isNasalMarkerUppercaseEnabled: Boolean,
): CaseResponse? {
    val response = RustEngineBridge.dispatch(op) {
        setConfigSnapshot(caseAppConfig(isNasalMarkerUppercaseEnabled))
        setCaseTransform(caseRequest)
    } ?: return null
    return if (response.hasCaseTransform()) response.caseTransform else null
}

// Only the field `phonetics::case_requests::handle` reads — the input mode does not change casing.
// Inverted on the wire, as in `appConfig` (proto default = the marker follows the case, §53).
internal fun caseAppConfig(isNasalMarkerUppercaseEnabled: Boolean): AppConfig =
    AppConfig
        .newBuilder()
        .setForceLowercaseNasalMarker(!isNasalMarkerUppercaseEnabled)
        .build()

// endregion
