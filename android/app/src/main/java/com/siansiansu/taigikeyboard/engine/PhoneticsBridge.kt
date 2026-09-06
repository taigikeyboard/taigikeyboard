// Phonetics + Derivation + TPS ops — 15 extensions on RustEngineBridge + the toneVariations cache.
// Mirrors iOS RustEngineBridge+Phonetics.swift (TPS merged into same file per simplify decision).
// Sends through RustEngineBridge.dispatch(op) { … } — shared JNI hop, exception boundary, recordFailure sink.

package com.siansiansu.taigikeyboard.engine

import com.siansiansu.taigikeyboard.engine.proto.AppConfig
import com.siansiansu.taigikeyboard.engine.proto.BoolResult
import com.siansiansu.taigikeyboard.engine.proto.ContainsTps
import com.siansiansu.taigikeyboard.engine.proto.CustomSearchKeysResult
import com.siansiansu.taigikeyboard.engine.proto.DeriveAbbrev
import com.siansiansu.taigikeyboard.engine.proto.DeriveCustomQueryKey
import com.siansiansu.taigikeyboard.engine.proto.DeriveCustomSearchKeys
import com.siansiansu.taigikeyboard.engine.proto.DeriveNotone
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
import com.siansiansu.taigikeyboard.ime.core.settings.InputMode

// region Phonetics core (8 ops)

/**
 * `Method::NormalizeTone` — input + AppConfig.input_mode + ToneToggles →
 * tone-marked string. `mode` and `toggles` are mandatory (no default)
 * to enforce the live-read invariant per Codex v2 §7.
 */
fun RustEngineBridge.normalizeTone(
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

// Strips the syllable's tone combining mark; tone is "" when the syllable has none.
fun RustEngineBridge.stripTone(input: String): StripToneOutcome {
    val payload = StripTone.newBuilder().setInput(input).build()
    val resp = phoneticsDispatch({ it.stripTone = payload }, "stripTone", null)
        ?: return StripToneOutcome(input, "")
    if (!resp.hasStripToneResult()) {
        RustEngineBridge.recordFailure("stripTone", "missing StripToneResult")
        return StripToneOutcome(input, "")
    }
    val r: StripToneResult = resp.stripToneResult
    return StripToneOutcome(r.bare, r.tone)
}

fun RustEngineBridge.pojToTl(input: String): String {
    val payload = PojToTl.newBuilder().setInput(input).build()
    return stringDispatch({ it.pojToTl = payload }, input, "pojToTl", null)
}

fun RustEngineBridge.tlToPoj(input: String): String {
    val payload = TlToPoj.newBuilder().setInput(input).build()
    return stringDispatch({ it.tlToPoj = payload }, input, "tlToPoj", null)
}

fun RustEngineBridge.normalizeToTl(input: String): String {
    val payload = NormalizeToTl.newBuilder().setInput(input).build()
    return stringDispatch({ it.normalizeToTl = payload }, input, "normalizeToTl", null)
}

// Full NormalizeInput pipeline down to a trie-query key: TPS preprocess, lowercase, syllable split,
// nasal / o͘ prep, checked-ending inference.
fun RustEngineBridge.normalizeInput(input: String): String {
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
fun RustEngineBridge.nfdPreprocessForLookup(input: String): String {
    val payload = NfdPreprocessForLookup.newBuilder().setInput(input).build()
    return stringDispatch(
        { it.nfdPreprocessForLookup = payload },
        input,
        "nfdPreprocessForLookup",
        null,
    )
}

// Backspace path: drops the last NFD tone mark and recomposes; null when there is no mark.
fun RustEngineBridge.restoreTone(text: String): String? {
    val payload = RestoreTone.newBuilder().setText(text).build()
    val resp = phoneticsDispatch({ it.restoreTone = payload }, "restoreTone", null) ?: return null
    if (!resp.hasOptionalStringResult()) {
        RustEngineBridge.recordFailure("restoreTone", "missing OptionalStringResult")
        return null
    }
    val r: OptionalStringResult = resp.optionalStringResult
    return if (r.present) r.output else null
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
        val resp = phoneticsDispatch({ it.getToneVariations = payload }, "getToneVariations", null)
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
// region Derivation (2 ops)

// Custom-dictionary search key: toneless form used for toneless prefix search.
fun RustEngineBridge.deriveNotone(roman: String): String {
    val payload = DeriveNotone.newBuilder().setRoman(roman).build()
    return stringDispatch({ it.deriveNotone = payload }, roman, "deriveNotone", null)
}

// Custom-dictionary search key: per-syllable initials (split on hyphen/space); "" for a single syllable.
fun RustEngineBridge.deriveAbbrev(roman: String): String {
    val payload = DeriveAbbrev.newBuilder().setRoman(roman).build()
    return stringDispatch({ it.deriveAbbrev = payload }, roman, "deriveAbbrev", null)
}

/**
 * `Method::DeriveCustomSearchKeys` — WRITE side (v3.6.1 R3). Full
 * {tl, poj, tps} × {num, notone, abbrev} (+ TPS er/or variant) bundle for
 * a stored custom-dict roman, materialized into the `custom_search_key`
 * side table. Empty on FFI failure / residue-only input.
 */
fun RustEngineBridge.deriveCustomSearchKeys(roman: String): List<CustomSearchKey> {
    val payload = DeriveCustomSearchKeys.newBuilder().setRoman(roman).build()
    return customSearchKeys({ it.deriveCustomSearchKeys = payload }, "deriveCustomSearchKeys")
}

/**
 * `Method::DeriveCustomQueryKey` — READ side (v3.6.1 R3). Single
 * family-native key for the current raw `input` + settings `mode`. TPS
 * collapses to "tl" through [InputMode], so the engine upgrades the family
 * to TPS via `contains_tps(raw)` instead. `null` for residue-only / empty
 * input or FFI failure.
 */
fun RustEngineBridge.deriveCustomQueryKey(
    input: String,
    mode: InputMode,
): CustomSearchKey? {
    val payload = DeriveCustomQueryKey
        .newBuilder()
        .setInput(input)
        .setInputMode(customSearchInputMode(mode))
        .build()
    return customSearchKeys({ it.deriveCustomQueryKey = payload }, "deriveCustomQueryKey").firstOrNull()
}

/**
 * Map the platform [InputMode] to the engine `input_mode` string. Android's
 * enum has no TPS case (`"tps"` settings collapses to `TL` upstream via
 * `InputMode.fromPrefString`); the engine upgrades to the TPS family via
 * `contains_tps` on the raw input. Mirrors iOS
 * `RustEngineBridge+Phonetics.swift` `customSearchInputMode`.
 */
private fun customSearchInputMode(mode: InputMode): String =
    when (mode) {
        InputMode.POJ -> "poj"
        InputMode.ENGLISH -> "english"
        InputMode.TL -> "tl"
    }

// endregion
// region TPS (5 ops)

// Composing's derived display uses this to skip POJ/TL tone conversion.
fun RustEngineBridge.containsTps(text: String): Boolean {
    val payload = ContainsTps.newBuilder().setText(text).build()
    return boolDispatch({ it.containsTps = payload }, "containsTps")
}

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
    return stringDispatch({ it.tlNumericToTps = payload }, text, "tlNumericToTps", null)
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
    return stringDispatch({ it.tlDisplayToTps = payload }, text, "tlDisplayToTps", null)
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
    val resp = phoneticsDispatch({ it.tpsInputAdjust = payload }, "tpsInputAdjust", null)
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
    config: AppConfig?,
): PhoneticsResponse? {
    val phoneticsBuilder = PhoneticsRequest.newBuilder()
    methodSetter(phoneticsBuilder)
    val phoneticsRequest = phoneticsBuilder.build()
    val response = RustEngineBridge.dispatch(op) {
        setPhonetics(phoneticsRequest)
        if (config != null) {
            configSnapshot = config
        }
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
    config: AppConfig?,
): String {
    val resp = phoneticsDispatch(methodSetter, op, config) ?: return input
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
    val resp = phoneticsDispatch(methodSetter, op, null) ?: return false
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
private inline fun customSearchKeys(
    methodSetter: (PhoneticsRequest.Builder) -> Unit,
    op: String,
): List<CustomSearchKey> {
    val resp = phoneticsDispatch(methodSetter, op, null) ?: return emptyList()
    if (!resp.hasCustomSearchKeysResult()) {
        RustEngineBridge.recordFailure(op, "expected CustomSearchKeysResult")
        return emptyList()
    }
    val r: CustomSearchKeysResult = resp.customSearchKeysResult
    return r.keysList.map { CustomSearchKey(family = it.family, form = it.form, key = it.key) }
}

// endregion
