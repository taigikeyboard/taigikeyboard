package com.siansiansu.taigikeyboard.engine

import com.siansiansu.taigikeyboard.engine.proto.AppConfig
import com.siansiansu.taigikeyboard.engine.proto.CapitalizeCandidate
import com.siansiansu.taigikeyboard.engine.proto.CaseRequest
import com.siansiansu.taigikeyboard.engine.proto.CaseResponse
import com.siansiansu.taigikeyboard.engine.proto.FullUppercaseToneString
import com.siansiansu.taigikeyboard.engine.proto.LowercaseToneChar
import com.siansiansu.taigikeyboard.engine.proto.Request
import com.siansiansu.taigikeyboard.engine.proto.Response
import com.siansiansu.taigikeyboard.engine.proto.TransformInputCase
import com.siansiansu.taigikeyboard.engine.proto.TransformSuggestion
import com.siansiansu.taigikeyboard.engine.proto.UppercaseToneChar
import com.siansiansu.taigikeyboard.ime.core.settings.InputMode
import com.siansiansu.taigikeyboard.engine.proto.LetterCase as ProtoLetterCase

/**
 * Case-transform bridge. Top-level object (mirrors `LexiconBridge` pattern).
 * Single FFI hop per per-char or per-word case operation. Mode is forwarded
 * via envelope `AppConfig.input_mode`; case-transform is independent of POJ
 * doubletap preprocessing so the toggles fields are left at default.
 *
 * Replaces the algorithm body of:
 * - `dictionary/ToneUtilities.kt` (uppercase/lowercase tone letter, nasal adjust)
 * - `dictionary/SuggestionCaseTransformer.kt` (per-word transform)
 *
 * Suggestion skip rules (`id < 0 && id != -2` and `id == 0`) stay platform-side
 * — only transform-eligible items reach `transformSuggestion(...)`.
 */
object CaseTransformBridge {
    private const val TAG = "CaseTransformBridge"

    /**
     * Three-state shift / case indicator. Bridge-side mirror of the proto
     * `LetterCase` enum + the iOS `CaseTransformLetterCase`. Adapts
     * Android's existing `(caps: Boolean, capsLock: Boolean)` pair at the
     * call site (CapsLock=true → CapsLocked; caps=true → Uppercased;
     * else Lowercased) — see `from()` factory.
     */
    enum class LetterCase(val protoValue: Int) {
        LOWERCASED(1),
        UPPERCASED(2),
        CAPS_LOCKED(3),
        ;

        companion object {
            /** Adapter from Android's existing caps + capsLock boolean pair. */
            @JvmStatic
            fun from(caps: Boolean, capsLock: Boolean): LetterCase = when {
                capsLock -> CAPS_LOCKED
                caps -> UPPERCASED
                else -> LOWERCASED
            }
        }
    }

    // region Per-char helpers

    /** Replaces `ToneUtilities.uppercaseToneLetter`. */
    fun uppercaseToneChar(input: String, mode: InputMode): String {
        val payload = UppercaseToneChar.newBuilder().setInput(input).build()
        return stringDispatch(
            CaseRequest.newBuilder().setUppercaseToneChar(payload).build(),
            op = "uppercaseToneChar",
            mode = mode,
            fallback = input,
        )
    }

    /** Replaces `ToneUtilities.fullUppercaseToneLetter`. */
    fun fullUppercaseToneString(input: String, mode: InputMode): String {
        val payload = FullUppercaseToneString.newBuilder().setInput(input).build()
        return stringDispatch(
            CaseRequest.newBuilder().setFullUppercaseToneString(payload).build(),
            op = "fullUppercaseToneString",
            mode = mode,
            fallback = input,
        )
    }

    /** Replaces `ToneUtilities.lowercaseToneLetter`. */
    fun lowercaseToneChar(input: String, mode: InputMode): String {
        val payload = LowercaseToneChar.newBuilder().setInput(input).build()
        return stringDispatch(
            CaseRequest.newBuilder().setLowercaseToneChar(payload).build(),
            op = "lowercaseToneChar",
            mode = mode,
            fallback = input,
        )
    }

    // endregion

    // region Per-string compound transforms

    /** Apply `letterCase` to `text` per the engine's input-case pipeline. */
    fun transformInputCase(text: String, letterCase: LetterCase, mode: InputMode): String {
        val payload = TransformInputCase.newBuilder()
            .setText(text)
            .setLetterCase(ProtoLetterCase.forNumber(letterCase.protoValue) ?: ProtoLetterCase.LETTER_CASE_UNSPECIFIED)
            .build()
        return stringDispatch(
            CaseRequest.newBuilder().setTransformInputCase(payload).build(),
            op = "transformInputCase",
            mode = mode,
            fallback = text,
        )
    }

    /**
     * Per-suggestion case transformation. Output is post-processed via
     * engine-side `adjust_nasal_marker_case` (no separate FFI hop needed).
     */
    fun transformSuggestion(
        original: String,
        composing: String,
        letterCase: LetterCase,
        mode: InputMode,
    ): String {
        val payload = TransformSuggestion.newBuilder()
            .setOriginalText(original)
            .setComposingText(composing)
            .setLetterCase(ProtoLetterCase.forNumber(letterCase.protoValue) ?: ProtoLetterCase.LETTER_CASE_UNSPECIFIED)
            .build()
        return stringDispatch(
            CaseRequest.newBuilder().setTransformSuggestion(payload).build(),
            op = "transformSuggestion",
            mode = mode,
            fallback = original,
        )
    }

    // endregion

    // region Private dispatch

    private fun stringDispatch(
        caseRequest: CaseRequest,
        op: String,
        mode: InputMode,
        fallback: String,
    ): String {
        val resp = dispatch(caseRequest, mode) ?: return fallback
        if (!resp.hasStringResult()) {
            android.util.Log.w(TAG, "[$op] missing string_result")
            return fallback
        }
        return resp.stringResult.output
    }

    private fun dispatch(caseRequest: CaseRequest, mode: InputMode): CaseResponse? {
        val request = Request.newBuilder()
            .setId(RustEngineBridge.nextRequestIdInternal())
            .setConfigSnapshot(appConfig(mode))
            .setCaseTransform(caseRequest)
            .build()
        val responseBytes = try {
            RustEngineBridge.dispatchRaw(request.toByteArray())
        } catch (t: Throwable) {
            android.util.Log.w(TAG, "dispatch failed", t)
            return null
        }
        val response = try {
            Response.parseFrom(responseBytes)
        } catch (t: Throwable) {
            android.util.Log.w(TAG, "response parse failed", t)
            return null
        }
        if (response.errorValue != 0) {
            android.util.Log.w(TAG, "engine returned error: ${response.error}")
            return null
        }
        return if (response.hasCaseTransform()) response.caseTransform else null
    }

    private fun appConfig(mode: InputMode): AppConfig =
        AppConfig.newBuilder()
            .setInputMode(
                when (mode) {
                    InputMode.POJ -> "poj"
                    InputMode.TL -> "tl"
                    InputMode.ENGLISH -> "english"
                },
            )
            .build()

    // endregion
}
