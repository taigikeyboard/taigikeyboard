// Lexicon read-path ops — extensions on RustEngineBridge (iOS counterpart:
// RustEngineBridge+Lexicon.swift). Read-path only — the mutable `user_association.db` SQLite half of
// NextWord persistence is out of scope; only the bundled `association.bin` read-only half goes
// through here. Sends through `RustEngineBridge.dispatch` (shared request-id counter, JNI hop,
// exception boundary and `recordFailure` sink). DTOs live on `RustEngineBridge` (`LexiconRow`,
// `LexiconInstallStats`, `LexiconInputMode`, `DictionaryToggles`, `DictionaryFilters`).

package com.siansiansu.taigikeyboard.engine

import com.siansiansu.taigikeyboard.engine.proto.DictionaryFiltersRequest
import com.siansiansu.taigikeyboard.engine.proto.DictionarySourceCode
import com.siansiansu.taigikeyboard.engine.proto.InputMode
import com.siansiansu.taigikeyboard.engine.proto.InstallRequest
import com.siansiansu.taigikeyboard.engine.proto.IsHanziRequest
import com.siansiansu.taigikeyboard.engine.proto.KautianSubcollToggles
import com.siansiansu.taigikeyboard.engine.proto.LexiconRequest
import com.siansiansu.taigikeyboard.engine.proto.LexiconResponse
import com.siansiansu.taigikeyboard.engine.proto.SearchByHanziRequest
import com.siansiansu.taigikeyboard.engine.proto.SearchWithSourcesRequest
import com.siansiansu.taigikeyboard.ime.dictionary.DictionarySource
import com.siansiansu.taigikeyboard.engine.proto.DictionaryToggles as ProtoDictionaryToggles
import com.siansiansu.taigikeyboard.engine.proto.TaigiWord as ProtoTaigiWord

// region Read path (5 ops)

/**
 * Install (or atomically reinstall) the lexicon engine state. Called
 * from `AppInitializer` after `copyAssetsIfNeeded` finishes; idempotent.
 * Returns `null` on failure (logged via `RustEngineBridge.diagnostics()`).
 *
 * @param syllableInventoryPath Absolute path to v3.5.8 Phase 2
 *   `syllables.fst` (TL syllable inventory FST). Empty string =
 *   skip-install; engine leaves `EngineState.syllable_inventory = None`
 *   and `composingFetchAtPos` returns empty candidates (graceful
 *   degrade). Required parameter to keep callers honest — silently
 *   omitting the file would make the v3.5.8 continuous-input feature
 *   appear "implemented" while returning zero candidates.
 */
fun RustEngineBridge.lexiconInstall(
    triePath: String,
    dictionaryBinPath: String,
    associationBinPath: String,
    dictionaryVersion: UInt,
    syllableInventoryPath: String,
): RustEngineBridge.LexiconInstallStats? {
    val payload = InstallRequest
        .newBuilder()
        .setTriePath(triePath)
        .setDictionaryBinPath(dictionaryBinPath)
        .setAssociationBinPath(associationBinPath)
        .setDictionaryVersion(dictionaryVersion.toInt())
        .setSyllableInventoryPath(syllableInventoryPath)
        .build()
    val resp = lexiconDispatch(LexiconRequest.newBuilder().setInstall(payload).build(), "lexiconInstall") ?: return null
    if (!resp.hasInstallResult()) {
        return null
    }
    val r = resp.installResult
    return RustEngineBridge.LexiconInstallStats(
        dictionaryRecordCount = r.dictionaryRecordCount.toULong(),
        prefixIndexEntryCount = r.prefixIndexEntryCount.toULong(),
    )
}

/**
 * Dictionary tab multi-source lookup. Engine classifies `input` as romanization or Hanji.
 */
fun RustEngineBridge.searchWithSources(
    input: String,
    inputMode: RustEngineBridge.LexiconInputMode,
    limit: UInt,
    enabledSourcesBitmask: UInt,
): List<RustEngineBridge.LexiconRow> {
    val payload = SearchWithSourcesRequest
        .newBuilder()
        .setInput(input)
        .setInputMode(InputMode.forNumber(inputMode.protoValue) ?: InputMode.INPUT_MODE_UNSPECIFIED)
        .setLimit(limit.toInt())
        .setEnabledSourcesBitmask(enabledSourcesBitmask.toInt())
        .build()
    val resp = lexiconDispatch(LexiconRequest.newBuilder().setSearchWithSources(payload).build(), "searchWithSources")
        ?: return emptyList()
    if (!resp.hasSearchWithSourcesResult()) return emptyList()
    return resp.searchWithSourcesResult.rowsList.map(::taigiWordToRow)
}

/**
 * Dictionary tab hanzi-prefix lookup; `query` must be Hanji.
 */
fun RustEngineBridge.searchByHanzi(
    query: String,
    inputMode: RustEngineBridge.LexiconInputMode,
    limit: UInt,
    enabledSourcesBitmask: UInt,
): List<RustEngineBridge.LexiconRow> {
    val payload = SearchByHanziRequest
        .newBuilder()
        .setQuery(query)
        .setInputMode(InputMode.forNumber(inputMode.protoValue) ?: InputMode.INPUT_MODE_UNSPECIFIED)
        .setLimit(limit.toInt())
        .setEnabledSourcesBitmask(enabledSourcesBitmask.toInt())
        .build()
    val resp = lexiconDispatch(LexiconRequest.newBuilder().setSearchByHanzi(payload).build(), "searchByHanzi")
        ?: return emptyList()
    if (!resp.hasSearchByHanziResult()) return emptyList()
    return resp.searchByHanziResult.rowsList.map(::taigiWordToRow)
}

// endregion
// region Classification (v3.5.7)

/**
 * Resolve user's 12-toggle dictionary preferences into ready-to-send
 * filter bitmasks + enabled-source set. Single FFI hop replaces the
 * pre-v3.5.8 verbatim-mirrored `EnabledDictionaries` bit math.
 *
 * Call ONCE per query and pass the result down the search pipeline;
 * resolving again inside the Dictionary tab's badge filter would split the snapshot.
 */
fun RustEngineBridge.dictionaryFilters(toggles: RustEngineBridge.DictionaryToggles): RustEngineBridge.DictionaryFilters {
    val payload = DictionaryFiltersRequest
        .newBuilder()
        .setToggles(dictionaryTogglesProto(toggles))
        .build()
    // Binary skew fallback: when method 18 dispatch fails (e.g. Kotlin
    // updated but Rust .so not rebuilt) but methods 12-17 still work,
    // the dev-only fallback would silently strip user-enabled dictionaries.
    // Mirror Rust `engine/lexicon/src/dictionary_filters.rs::compute_filters`
    // here so search call paths continue to honor user toggles.
    // Codex PR #210 r3182714295.
    val resp = lexiconDispatch(LexiconRequest.newBuilder().setDictionaryFilters(payload).build(), "dictionaryFilters")
    if (resp == null || !resp.hasDictionaryFiltersResult()) {
        return platformFallbackFilters(toggles)
    }
    val r = resp.dictionaryFiltersResult
    return RustEngineBridge.DictionaryFilters(
        dictionaryFilterBitmask = r.dictionaryFilterBitmask.toUInt(),
        assocLookupBitmask = r.assocLookupBitmask.toUInt(),
        enabledSources = r.enabledSourceCodesList
            .mapNotNull(::platformDictionarySource)
            .toSet(),
    )
}

/** Proto form of the user's dictionary toggles, shared by [dictionaryFilters] and `nextwordPredictNext`. */
internal fun dictionaryTogglesProto(toggles: RustEngineBridge.DictionaryToggles): ProtoDictionaryToggles =
    ProtoDictionaryToggles
        .newBuilder()
        .setKautian(toggles.kautian)
        .setTaigitv(toggles.taigitv)
        .setItaigi(toggles.itaigi)
        .setSitbut(toggles.sitbut)
        .setTaihoa(toggles.taihoa)
        .setTaijit(toggles.taijit)
        .setKungge(toggles.kungge)
        .setStti(toggles.stti)
        .setKhpoo(toggles.khpoo)
        .setVariant(toggles.variant)
        .setKhiin(toggles.khiin)
        .setLkk(toggles.lkk)
        .setDev(toggles.dev)
        // Always set the subcollection message (Android ships the toggles)
        // so the engine runs the gate; absence would signal legacy all-on
        // (DD5). Mirrors iOS RustEngineBridge togglesProto mapping.
        .setKautianSubcoll(
            KautianSubcollToggles
                .newBuilder()
                .setAccentLukang(toggles.kautianSubcoll.lukang)
                .setAccentSansia(toggles.kautianSubcoll.sansia)
                .setAccentTaipak(toggles.kautianSubcoll.taipak)
                .setAccentGilan(toggles.kautianSubcoll.gilan)
                .setAccentTainan(toggles.kautianSubcoll.tainan)
                .setAccentKaohsiung(toggles.kautianSubcoll.kaohsiung)
                .setAccentKinmen(toggles.kautianSubcoll.kinmen)
                .setAccentMakung(toggles.kautianSubcoll.makung)
                .setAccentSintik(toggles.kautianSubcoll.sintik)
                .setAccentTaichung(toggles.kautianSubcoll.taichung)
                .setNameAppendix(toggles.kautianSubcoll.nameAppendix)
                .build(),
        ).build()

/**
 * Dictionary tab short-circuit predicate. True iff `text` contains any CJK
 * codepoint (Unified + Extensions A-E). See
 * `INVARIANT_LEX_INPUT_CLASSIFICATION_HANZI_RANGE`.
 * Engine-side check fixes the pre-v3.5.7 Kotlin `Char.code` (16-bit) miss on Ext B-E.
 */
fun RustEngineBridge.isHanzi(text: String): Boolean {
    val payload = IsHanziRequest.newBuilder().setText(text).build()
    val resp = lexiconDispatch(LexiconRequest.newBuilder().setIsHanzi(payload).build(), "isHanzi") ?: return false
    if (!resp.hasIsHanziResult()) return false
    return resp.isHanziResult.isHanzi
}

/**
 * Fallback only for platform/Rust binary skew where method 18 is absent.
 * Rust `engine/lexicon/src/dictionary_filters.rs::compute_filters` is
 * authoritative; keep this bit layout in sync with
 * `engine/protos/proto/lexicon.proto`. Bit positions pinned by the
 * 6 inline Rust golden tests. Mirrors iOS
 * `RustEngineBridge.platformFallbackFilters` — must drift together.
 */
private fun platformFallbackFilters(toggles: RustEngineBridge.DictionaryToggles): RustEngineBridge.DictionaryFilters {
    var dictMask = 0u
    if (toggles.kautian) dictMask = dictMask or (1u shl 0)
    if (toggles.taigitv) dictMask = dictMask or (1u shl 1)
    if (toggles.itaigi) dictMask = dictMask or (1u shl 2)
    if (toggles.sitbut) dictMask = dictMask or (1u shl 3)
    if (toggles.taihoa) dictMask = dictMask or (1u shl 4)
    if (toggles.taijit) dictMask = dictMask or (1u shl 5)
    if (toggles.kungge) dictMask = dictMask or (1u shl 6)
    if (toggles.stti) dictMask = dictMask or (1u shl 7)
    if (toggles.khpoo) dictMask = dictMask or (1u shl 8)
    if (toggles.khiin) dictMask = dictMask or (1u shl 9)
    if (toggles.dev) dictMask = dictMask or (1u shl 10)
    if (toggles.lkk) dictMask = dictMask or (1u shl 11)
    if (toggles.variant) dictMask = dictMask or (1u shl 12)
    dictMask = dictMask or encodeKautianSubcollWire(toggles)

    val allAssocOn = toggles.kautian &&
        toggles.taigitv &&
        toggles.itaigi &&
        toggles.sitbut &&
        toggles.taihoa &&
        toggles.taijit &&
        toggles.kungge &&
        toggles.stti &&
        toggles.khpoo
    val assocMask: UInt = if (allAssocOn) UInt.MAX_VALUE else (dictMask and 0x1FFu)

    val enabled = mutableSetOf(DictionarySource.CUSTOM)
    if (toggles.dev) enabled.add(DictionarySource.DEV)
    if (toggles.kautian) enabled.add(DictionarySource.KAUTIAN)
    if (toggles.taigitv) enabled.add(DictionarySource.TAIGITV)
    if (toggles.itaigi) enabled.add(DictionarySource.ITAIGI)
    if (toggles.sitbut) enabled.add(DictionarySource.SITBUT)
    if (toggles.taihoa) enabled.add(DictionarySource.TAIHOA)
    if (toggles.taijit) enabled.add(DictionarySource.TAIJIT)
    if (toggles.kungge) enabled.add(DictionarySource.KUNGGE)
    if (toggles.stti) enabled.add(DictionarySource.STTI)
    if (toggles.khpoo) enabled.add(DictionarySource.KHPOO)
    if (toggles.khiin) enabled.add(DictionarySource.KHIIN)
    if (toggles.lkk) enabled.add(DictionarySource.LKK)
    return RustEngineBridge.DictionaryFilters(
        dictionaryFilterBitmask = dictMask,
        assocLookupBitmask = assocMask,
        enabledSources = enabled,
    )
}

/**
 * kautian subcollection wire ENCODE — fallback-only mirror of Rust
 * `engine/lexicon/src/dictionary_filters.rs::encode_kautian_subcoll_wire`.
 * Returns the wire high region (bit 13 active + bits 14..=25 enable mask)
 * when the kautian master is on; `0` otherwise (kautian rows drop via the
 * source-OR anyway). Keeps fallback behaviour identical to the engine so a
 * binary-skew session does not silently revert subcollection toggles. The
 * `main` subtag bit is set unconditionally when the master is on (主條目 is
 * not a user toggle). Mirrors iOS `RustEngineBridge.encodeKautianSubcollWire`.
 *
 * Pure top-level function (no `RustEngineBridge` receiver) so the JVM unit
 * test `KautianSubcollWireEncodeTest` can call it without triggering
 * `System.loadLibrary("rust_taigi")`.
 *
 * CROSS-PLATFORM INVARIANT — bit positions mirror
 * `engine/lexicon/src/dictionary_reader.rs` (`KAUTIAN_SUBTAG_*` /
 * `WIRE_KAUTIAN_SUBCOLL_*`). Drift causes silent subcollection-filter divergence.
 */
internal fun encodeKautianSubcollWire(toggles: RustEngineBridge.DictionaryToggles): UInt {
    if (!toggles.kautian) return 0u
    val sub = toggles.kautianSubcoll
    // Accent order MUST match config.yaml `dialect_columns` (subtag bit = 1 + index).
    val accents = listOf(
        sub.lukang,
        sub.sansia,
        sub.taipak,
        sub.gilan,
        sub.tainan,
        sub.kaohsiung,
        sub.kinmen,
        sub.makung,
        sub.sintik,
        sub.taichung,
    )
    var subtag = 1 shl KAUTIAN_SUBTAG_MAIN_BIT // main always on when master on
    accents.forEachIndexed { index, enabled ->
        if (enabled) subtag = subtag or (1 shl (KAUTIAN_SUBTAG_ACCENT_SHIFT + index))
    }
    if (sub.nameAppendix) subtag = subtag or (1 shl KAUTIAN_SUBTAG_NAME_BIT)
    return WIRE_KAUTIAN_SUBCOLL_ACTIVE_BIT or (subtag.toUInt() shl WIRE_KAUTIAN_SUBCOLL_SHIFT)
}

// kautian subtag bit layout (mirrors Rust dictionary_reader.rs KAUTIAN_SUBTAG_*).
private const val KAUTIAN_SUBTAG_MAIN_BIT = 0
private const val KAUTIAN_SUBTAG_ACCENT_SHIFT = 1
private const val KAUTIAN_SUBTAG_NAME_BIT = 11

// wire high region (mirrors Rust dictionary_reader.rs WIRE_KAUTIAN_SUBCOLL_*).
private val WIRE_KAUTIAN_SUBCOLL_ACTIVE_BIT: UInt = 1u shl 13
private const val WIRE_KAUTIAN_SUBCOLL_SHIFT = 14

/**
 * Map proto `DictionarySourceCode` to the platform `DictionarySource`
 * enum. Explicit `when` (no `ordinal` reliance — Kotlin enum has no
 * stable numeric value; codes are wire-stable per
 * `lexicon.proto::DictionarySourceCode`). Unspecified / unrecognised
 * codes return `null` and the caller drops them. Mirrors iOS
 * `RustEngineBridge.platformDictionarySource` — must drift together.
 */
private fun platformDictionarySource(code: DictionarySourceCode): DictionarySource? =
    when (code) {
        DictionarySourceCode.DICT_SOURCE_KAUTIAN -> DictionarySource.KAUTIAN

        DictionarySourceCode.DICT_SOURCE_TAIGITV -> DictionarySource.TAIGITV

        DictionarySourceCode.DICT_SOURCE_ITAIGI -> DictionarySource.ITAIGI

        DictionarySourceCode.DICT_SOURCE_SITBUT -> DictionarySource.SITBUT

        DictionarySourceCode.DICT_SOURCE_TAIHOA -> DictionarySource.TAIHOA

        DictionarySourceCode.DICT_SOURCE_TAIJIT -> DictionarySource.TAIJIT

        DictionarySourceCode.DICT_SOURCE_KUNGGE -> DictionarySource.KUNGGE

        DictionarySourceCode.DICT_SOURCE_STTI -> DictionarySource.STTI

        DictionarySourceCode.DICT_SOURCE_KHPOO -> DictionarySource.KHPOO

        DictionarySourceCode.DICT_SOURCE_KHIIN -> DictionarySource.KHIIN

        DictionarySourceCode.DICT_SOURCE_LKK -> DictionarySource.LKK

        DictionarySourceCode.DICT_SOURCE_DEV -> DictionarySource.DEV

        DictionarySourceCode.DICT_SOURCE_CUSTOM -> DictionarySource.CUSTOM

        DictionarySourceCode.DICT_SOURCE_UNSPECIFIED,
        DictionarySourceCode.UNRECOGNIZED,
        -> null
    }

// endregion Classification

private fun lexiconDispatch(
    lexiconRequest: LexiconRequest,
    op: String,
): LexiconResponse? {
    val response = RustEngineBridge.dispatch(op) { setLexicon(lexiconRequest) } ?: return null
    if (!response.hasLexicon()) {
        RustEngineBridge.recordFailure(op, "missing lexicon payload")
        return null
    }
    return response.lexicon
}

private fun taigiWordToRow(proto: ProtoTaigiWord): RustEngineBridge.LexiconRow =
    RustEngineBridge.LexiconRow(
        id = proto.id,
        roman = proto.roman,
        hanzi = if (proto.hasHanji()) proto.hanji else null,
        lengthScore = if (proto.hasLengthScore()) proto.lengthScore else null,
        sourceBitmask = if (proto.hasSourceBitmask()) proto.sourceBitmask.toUInt() else null,
    )
