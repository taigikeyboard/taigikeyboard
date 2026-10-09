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
import com.siansiansu.taigikeyboard.engine.proto.IsHanjiRequest
import com.siansiansu.taigikeyboard.engine.proto.KautianSubcollectionToggles
import com.siansiansu.taigikeyboard.engine.proto.LexiconRequest
import com.siansiansu.taigikeyboard.engine.proto.LexiconResponse
import com.siansiansu.taigikeyboard.engine.proto.SearchByHanjiRequest
import com.siansiansu.taigikeyboard.engine.proto.SearchWithSourcesRequest
import com.siansiansu.taigikeyboard.ime.dictionary.DictionarySource
import com.siansiansu.taigikeyboard.engine.proto.DictionarySourceToggles as ProtoDictionarySourceToggles
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
 * Dictionary-search multi-source lookup. Engine classifies `input` as romanization or Hanji.
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
 * Dictionary-search hanji-prefix lookup; `query` must be Hanji.
 */
fun RustEngineBridge.searchByHanji(
    query: String,
    inputMode: RustEngineBridge.LexiconInputMode,
    limit: UInt,
    enabledSourcesBitmask: UInt,
): List<RustEngineBridge.LexiconRow> {
    val payload = SearchByHanjiRequest
        .newBuilder()
        .setQuery(query)
        .setInputMode(InputMode.forNumber(inputMode.protoValue) ?: InputMode.INPUT_MODE_UNSPECIFIED)
        .setLimit(limit.toInt())
        .setEnabledSourcesBitmask(enabledSourcesBitmask.toInt())
        .build()
    val resp = lexiconDispatch(LexiconRequest.newBuilder().setSearchByHanji(payload).build(), "searchByHanji")
        ?: return emptyList()
    if (!resp.hasSearchByHanjiResult()) return emptyList()
    return resp.searchByHanjiResult.rowsList.map(::taigiWordToRow)
}

// endregion
// region Classification (v3.5.7)

/**
 * Resolve user's 12-toggle dictionary preferences into ready-to-send
 * filter bitmasks + enabled-source set. Single FFI hop replaces the
 * pre-v3.5.8 verbatim-mirrored `EnabledDictionaries` bit math.
 *
 * Call ONCE per query and pass the result down the search pipeline;
 * resolving again inside the dictionary search's badge filter would split the snapshot.
 */
fun RustEngineBridge.dictionaryFilters(toggles: RustEngineBridge.DictionaryToggles): RustEngineBridge.DictionaryFilters {
    val payload = DictionaryFiltersRequest
        .newBuilder()
        .setToggles(dictionaryTogglesProto(toggles))
        .build()
    // The bit layout belongs to Rust (`compute_filters`); no platform
    // mirror. `lexiconDispatch` records its own failures.
    val resp = lexiconDispatch(LexiconRequest.newBuilder().setDictionaryFilters(payload).build(), "dictionaryFilters")
        ?: return RustEngineBridge.DictionaryFilters.ALL_SOURCES_ENABLED
    if (!resp.hasDictionaryFiltersResult()) {
        RustEngineBridge.recordFailure("dictionaryFilters", "missing dictionary_filters result")
        return RustEngineBridge.DictionaryFilters.ALL_SOURCES_ENABLED
    }
    val r = resp.dictionaryFiltersResult
    return RustEngineBridge.DictionaryFilters(
        dictionaryFilterBitmask = r.dictionaryFilterBitmask.toUInt(),
        enabledSources = r.enabledSourceCodesList
            .mapNotNull(::platformDictionarySource)
            .toSet(),
    )
}

/** Proto form of the user's dictionary toggles, shared by [dictionaryFilters], `composingFetchAtPos` and `nextwordPredictNext`. */
internal fun dictionaryTogglesProto(toggles: RustEngineBridge.DictionaryToggles): ProtoDictionarySourceToggles =
    ProtoDictionarySourceToggles
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
        .setKautianSubcollections(
            KautianSubcollectionToggles
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
                .setAltReading(toggles.kautianSubcoll.altReading)
                .build(),
        ).build()

/**
 * Dictionary-search short-circuit predicate. True iff `text` contains any CJK
 * ideograph in the engine's `CJK_RANGES` (the `dictionary/common/cjk.py`
 * table). See
 * `INVARIANT_LEX_INPUT_CLASSIFICATION_HANJI_RANGE`.
 * Engine-side check fixes the pre-v3.5.7 Kotlin `Char.code` (16-bit) miss on Ext B-E.
 */
fun RustEngineBridge.isHanji(text: String): Boolean {
    val payload = IsHanjiRequest.newBuilder().setText(text).build()
    val resp = lexiconDispatch(LexiconRequest.newBuilder().setIsHanji(payload).build(), "isHanji") ?: return false
    if (!resp.hasIsHanjiResult()) return false
    return resp.isHanjiResult.isHanji
}

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
        hanji = if (proto.hasHanji()) proto.hanji else null,
        sources = proto.sourcesList.mapNotNull(::platformDictionarySource),
    )
