// Lexicon read-path ops — extensions on RustEngineBridge (iOS counterpart:
// RustEngineBridge+Lexicon.swift). Read-path only — the mutable `user_association.db` SQLite half of
// NextWord persistence is out of scope; only the bundled `association.bin` read-only half goes
// through here. Sends through `RustEngineBridge.dispatch` (shared request-id counter, JNI hop,
// exception boundary and `recordFailure` sink). DTOs (`LexiconInstallStats`, `DictionaryToggles`)
// are defined below.

package com.siansiansu.taigikeyboard.engine

import com.siansiansu.taigikeyboard.engine.proto.DictionarySearchRequest
import com.siansiansu.taigikeyboard.engine.proto.DictionarySearchRow
import com.siansiansu.taigikeyboard.engine.proto.DictionarySourceCode
import com.siansiansu.taigikeyboard.engine.proto.InstallRequest
import com.siansiansu.taigikeyboard.engine.proto.KautianSubcollectionToggles
import com.siansiansu.taigikeyboard.engine.proto.LexiconRequest
import com.siansiansu.taigikeyboard.engine.proto.LexiconResponse
import com.siansiansu.taigikeyboard.ime.dictionary.DictionarySource
import com.siansiansu.taigikeyboard.ime.settings.EngineSettings
import com.siansiansu.taigikeyboard.engine.proto.DictionarySourceToggles as ProtoDictionarySourceToggles

// region Lexicon DTOs

/** Engine install diagnostic counts (for dogfood logging). */
data class LexiconInstallStats(
    val dictionaryRecordCount: ULong,
    val prefixIndexEntryCount: ULong,
)

/**
 * Snapshot of the user's dictionary preference state. Field order mirrors
 * `engine/protos/proto/lexicon.proto::DictionarySourceToggles` (12 source toggles
 * + nested [KautianSubcoll]). Build via `from(settings)`; never construct
 * piecemeal at search call sites — that splits the snapshot.
 */
data class DictionaryToggles(
    val kautian: Boolean,
    val taigitv: Boolean,
    val itaigi: Boolean,
    val sitbut: Boolean,
    val taihoa: Boolean,
    val taijit: Boolean,
    val kungge: Boolean,
    val stti: Boolean,
    val khpoo: Boolean,
    val variant: Boolean,
    val khiin: Boolean,
    val lkk: Boolean,
    val dev: Boolean,
    val kautianSubcoll: KautianSubcoll,
) {
    /**
     * kautian subcollection enable state (10 accents + name appendix).
     * Always sent; contract in `lexicon.proto` `DictionarySourceToggles`. Field order mirrors config.yaml
     * `dialect_columns` / proto `KautianSubcollectionToggles`.
     * Mirrors iOS `RustEngineBridge.DictionaryToggles.KautianSubcoll`.
     */
    data class KautianSubcoll(
        val lukang: Boolean,
        val sansia: Boolean,
        val taipak: Boolean,
        val gilan: Boolean,
        val tainan: Boolean,
        val kaohsiung: Boolean,
        val kinmen: Boolean,
        val makung: Boolean,
        val sintik: Boolean,
        val taichung: Boolean,
        val nameAppendix: Boolean,
        val altReading: Boolean,
    )

    companion object {
        fun from(settings: EngineSettings): DictionaryToggles =
            DictionaryToggles(
                kautian = settings.isMoeDictEnabled,
                taigitv = settings.isNewwordDictEnabled,
                itaigi = settings.isITaigiDictEnabled,
                sitbut = settings.isTaiwanPlantDictEnabled,
                taihoa = settings.isTaiHuaDictEnabled,
                taijit = settings.isTaiwanJapanDictEnabled,
                kungge = settings.isKunggeDictEnabled,
                stti = settings.isSttiDictEnabled,
                khpoo = settings.isKhpooDictEnabled,
                variant = settings.isVariantEnabled,
                khiin = settings.isKhiinEnabled,
                lkk = settings.isLkkDictEnabled,
                dev = settings.isDevDictEnabled,
                kautianSubcoll = KautianSubcoll(
                    lukang = settings.isKautianAccentLukangEnabled,
                    sansia = settings.isKautianAccentSansiaEnabled,
                    taipak = settings.isKautianAccentTaipakEnabled,
                    gilan = settings.isKautianAccentGilanEnabled,
                    tainan = settings.isKautianAccentTainanEnabled,
                    kaohsiung = settings.isKautianAccentKaohsiungEnabled,
                    kinmen = settings.isKautianAccentKinmenEnabled,
                    makung = settings.isKautianAccentMakungEnabled,
                    sintik = settings.isKautianAccentSintikEnabled,
                    taichung = settings.isKautianAccentTaichungEnabled,
                    nameAppendix = settings.isKautianNameAppendixEnabled,
                    altReading = settings.isKautianAltReadingEnabled,
                ),
            )
    }
}

// endregion

// region Read path

/**
 * Install (or atomically reinstall) the lexicon engine state. Called
 * from `AppInitializer` after `copyAssetsIfNeeded` finishes; idempotent.
 * Returns `null` on failure (logged via `RustEngineBridge.recordFailure`).
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
): LexiconInstallStats? {
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
    return LexiconInstallStats(
        dictionaryRecordCount = r.dictionaryRecordCount.toULong(),
        prefixIndexEntryCount = r.prefixIndexEntryCount.toULong(),
    )
}

/**
 * The Manage Dictionaries search in one call (`DictionarySearch`): the engine picks the Hanji or
 * romanization lookup, filters by [toggles], renders the romanization and puts the custom entries
 * first. [inputMode] is the stored settings spelling. `null` = the engine failed (recorded).
 * Blocks while the user data is still opening — call it off the main thread.
 */
fun RustEngineBridge.dictionarySearch(
    query: String,
    inputMode: String,
    toggles: DictionaryToggles,
    customDictionaryEnabled: Boolean,
    limit: Int,
): List<DictionarySearchRow>? {
    val payload = DictionarySearchRequest
        .newBuilder()
        .setQuery(query)
        .setInputMode(inputMode)
        .setToggles(dictionaryTogglesProto(toggles))
        .setCustomDictionaryEnabled(customDictionaryEnabled)
        .setLimit(limit.coerceAtLeast(0))
        .build()
    val resp = lexiconDispatch(LexiconRequest.newBuilder().setDictionarySearch(payload).build(), "dictionarySearch")
        ?: return null
    if (!resp.hasDictionarySearchResult()) {
        RustEngineBridge.recordFailure("dictionarySearch", "missing dictionary_search result")
        return null
    }
    return resp.dictionarySearchResult.rowsList
}

// endregion

// region Helpers

/** Proto form of the user's dictionary toggles, shared by [dictionarySearch], `continuousAppConfig` and `nextwordPredictNext`. */
internal fun dictionaryTogglesProto(toggles: DictionaryToggles): ProtoDictionarySourceToggles =
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
        // Always sent; contract in `lexicon.proto` `DictionarySourceToggles`. Mirrors iOS RustEngineBridge togglesProto mapping.
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
 * Map proto `DictionarySourceCode` to the platform `DictionarySource`
 * enum. Explicit `when` (no `ordinal` reliance — Kotlin enum has no
 * stable numeric value; codes are wire-stable per
 * `lexicon.proto::DictionarySourceCode`). Unspecified / unrecognised
 * codes return `null` and the caller drops them. Mirrors iOS
 * `RustEngineBridge.platformDictionarySource` — must drift together.
 */
internal fun platformDictionarySource(code: DictionarySourceCode): DictionarySource? =
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

// endregion Helpers

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
