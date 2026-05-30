// 中文: Lexicon 讀路徑橋:將 install / search / searchByHanzi / assocLookup /
// 中文: classifyInput / isHanzi / dictionaryFilters 等 op 包成 Kotlin API,
// 中文: 共用 RustEngineBridge.dispatchRaw 做 JNI roundtrip。對應 iOS RustEngineBridge+Lexicon.swift。

package com.siansiansu.taigikeyboard.engine

import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.engine.proto.AssocLookupRequest
import com.siansiansu.taigikeyboard.engine.proto.ClassifyInputRequest
import com.siansiansu.taigikeyboard.engine.proto.DictionaryFiltersRequest
import com.siansiansu.taigikeyboard.engine.proto.DictionarySourceCode
import com.siansiansu.taigikeyboard.engine.proto.ErrorCode
import com.siansiansu.taigikeyboard.engine.proto.InputMode
import com.siansiansu.taigikeyboard.engine.proto.InputType
import com.siansiansu.taigikeyboard.engine.proto.InstallRequest
import com.siansiansu.taigikeyboard.engine.proto.IsHanziRequest
import com.siansiansu.taigikeyboard.engine.proto.KautianSubcollToggles
import com.siansiansu.taigikeyboard.engine.proto.LexiconRequest
import com.siansiansu.taigikeyboard.engine.proto.LexiconResponse
import com.siansiansu.taigikeyboard.engine.proto.ProcessCandidatesRequest
import com.siansiansu.taigikeyboard.engine.proto.Request
import com.siansiansu.taigikeyboard.engine.proto.Response
import com.siansiansu.taigikeyboard.engine.proto.SearchByHanziRequest
import com.siansiansu.taigikeyboard.engine.proto.SearchRequest
import com.siansiansu.taigikeyboard.engine.proto.SearchWithSourcesRequest
import com.siansiansu.taigikeyboard.ime.core.settings.EngineSettings
import com.siansiansu.taigikeyboard.ime.dictionary.DictionarySource
import com.siansiansu.taigikeyboard.ime.dictionary.FrequencyData
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord
import com.siansiansu.taigikeyboard.engine.proto.DictionaryToggles as ProtoDictionaryToggles
import com.siansiansu.taigikeyboard.engine.proto.ScoreBreakdown as ProtoScoreBreakdown
import com.siansiansu.taigikeyboard.engine.proto.TaigiWord as ProtoTaigiWord
import com.siansiansu.taigikeyboard.ime.dictionary.InputType as DictInputType

/**
 * Lexicon read-path bridge. Top-level object (NOT a member of
 * `RustEngineBridge`) — keeps the lexicon LOC + functional cohesion in
 * one place at the cost of cross-class call inconsistency with the
 * composing / nextword / phonetics slices.
 *
 * Reuses `RustEngineBridge.dispatchRaw` for the JNI roundtrip (single
 * symbol bound to `RustEngineBridge`) and `nextRequestIdInternal` for
 * unique IDs across bridges.
 *
 * Read-path only — the mutable `user_association.db` SQLite half of
 * NextWord persistence is out of scope for this bridge; only the
 * bundled `association.bin` read-only half goes through here.
 *
 * 中文: 唯讀路徑 — `user_association.db` 由 Android 平台 SQLite 處理(設計如此),
 *       此橋只走 `association.bin` 唯讀資料(bundle 字典+ngram)。
 */
object LexiconBridge {
    private const val TAG = "LexiconBridge"

    /** Bridge-synthesized companion to proto `TaigiWord`. */
    data class Row(
        val id: Long,
        val roman: String,
        val hanzi: String?,
        val lengthScore: Int?,
        val sourceBitmask: UInt?,
    )

    /** Bridge-synthesized companion to proto `LexiconAssocEntry`. */
    data class AssocEntry(
        val previousWord: String,
        val candidateWord: String,
        val candidateTl: String,
        val count: UInt,
    )

    /** Engine install diagnostic counts (for dogfood logging). */
    data class InstallStats(
        val dictionaryRecordCount: ULong,
        val prefixIndexEntryCount: ULong,
    )

    /** Lexicon engine `inputType` enum (mirrors proto `InputType`). */
    enum class LexiconInputType(
        val protoValue: Int,
    ) {
        UNSPECIFIED(0),
        ROMAN_NO_TONE(1),
        ROMAN_WITH_TONE(2),
        HANZI(3),
    }

    /** Lexicon engine `inputMode` enum (mirrors proto `InputMode`). */
    enum class LexiconInputMode(
        val protoValue: Int,
    ) {
        UNSPECIFIED(0),
        TL(1),
        POJ(2),
        TPS(3),
    }

    /**
     * Snapshot of the user's dictionary preference state. Field order mirrors
     * `engine/protos/proto/lexicon.proto::DictionaryToggles` (12 source toggles
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
         * Android always populates this (the app ships the toggles), so the
         * `kautian_subcoll` proto message is always present and the engine
         * always runs the subcollection gate. Field order mirrors config.yaml
         * `dialect_columns` / proto `KautianSubcollToggles`.
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
                    ),
                )
        }
    }

    /**
     * Output of `dictionaryFilters` — ready-to-send bitmasks plus the
     * decoded enabled-source set for Dictionary tab retag. Replaces verbatim
     * platform `EnabledDictionaries` bit math (deleted in v3.5.8 slice).
     *
     * `assocLookupBitmask` carries the `UInt.MAX_VALUE` sentinel when all 9
     * association sources are on — preserves the documented
     * `lexicon.proto:166-173` shortcut. Caller forwards directly to
     * `assocLookup(enabledSourcesBitmask = ...)`.
     */
    data class DictionaryFilters(
        val dictionaryFilterBitmask: UInt,
        val assocLookupBitmask: UInt,
        val enabledSources: Set<DictionarySource>,
    )

    /**
     * Install (or atomically reinstall) the lexicon engine state. Called
     * from `AppInitializer` after `copyAssetsIfNeeded` finishes; idempotent.
     * Returns `null` on failure (logged via `RustEngineBridge.diagnostics()`).
     *
     * 中文: 安裝/重灌 lexicon 引擎(冪等)— 驗 trie/dictionary/association 三檔路徑後 mmap;
     *       失敗回 null,診斷打到 RustEngineBridge.diagnostics()。
     *
     * @param syllableInventoryPath Absolute path to v3.5.8 Phase 2
     *   `syllables.fst` (TL syllable inventory FST). Empty string =
     *   skip-install; engine leaves `EngineState.syllable_inventory = None`
     *   and `composingFetchAtPos` returns empty candidates (graceful
     *   degrade). Required parameter to keep callers honest — silently
     *   omitting the file would make the v3.5.8 continuous-input feature
     *   appear "implemented" while returning zero candidates.
     */
    fun install(
        triePath: String,
        dictionaryBinPath: String,
        associationBinPath: String,
        dictionaryVersion: UInt,
        syllableInventoryPath: String,
    ): InstallStats? {
        val payload = InstallRequest
            .newBuilder()
            .setTriePath(triePath)
            .setDictionaryBinPath(dictionaryBinPath)
            .setAssociationBinPath(associationBinPath)
            .setDictionaryVersion(dictionaryVersion.toInt())
            .setSyllableInventoryPath(syllableInventoryPath)
            .build()
        val resp = dispatch(LexiconRequest.newBuilder().setInstall(payload).build()) ?: return null
        if (!resp.hasInstallResult()) {
            return null
        }
        val r = resp.installResult
        return InstallStats(
            dictionaryRecordCount = r.dictionaryRecordCount.toULong(),
            prefixIndexEntryCount = r.prefixIndexEntryCount.toULong(),
        )
    }

    /**
     * IME autocomplete entry. Hanzi `inputType` returns `[]` per D-8 hard
     * guard (pinned by INVARIANT_LEX_HANZI_GUARD; commit 12 adds the
     * platform parity test).
     *
     * 中文: IME autocomplete 進入點;inputType==Hanzi 直接回 []。Engine 內部走 phonetics::normalize_input + trie 查詢。
     */
    fun search(
        input: String,
        inputType: LexiconInputType,
        inputMode: LexiconInputMode,
        limit: UInt,
        tpsOrMappedToER: Boolean,
        enabledSourcesBitmask: UInt,
    ): List<Row> {
        val payload = SearchRequest
            .newBuilder()
            .setInput(input)
            .setInputType(InputType.forNumber(inputType.protoValue) ?: InputType.INPUT_TYPE_UNSPECIFIED)
            .setInputMode(InputMode.forNumber(inputMode.protoValue) ?: InputMode.INPUT_MODE_UNSPECIFIED)
            .setLimit(limit.toInt())
            .setTpsOrMappedToEr(tpsOrMappedToER)
            .setEnabledSourcesBitmask(enabledSourcesBitmask.toInt())
            .build()
        val resp = dispatch(LexiconRequest.newBuilder().setSearch(payload).build()) ?: return emptyList()
        if (!resp.hasSearchResult()) return emptyList()
        return resp.searchResult.rowsList.map(::taigiWordToRow)
    }

    /**
     * Dictionary tab multi-source lookup.
     *
     * 中文: Tab3 多來源查詢 — input 可為羅馬字或漢字,engine 內自行分類;sources bitmask 由平台端 toggle 結果決定。
     */
    fun searchWithSources(
        input: String,
        inputMode: LexiconInputMode,
        limit: UInt,
        enabledSourcesBitmask: UInt,
    ): List<Row> {
        val payload = SearchWithSourcesRequest
            .newBuilder()
            .setInput(input)
            .setInputMode(InputMode.forNumber(inputMode.protoValue) ?: InputMode.INPUT_MODE_UNSPECIFIED)
            .setLimit(limit.toInt())
            .setEnabledSourcesBitmask(enabledSourcesBitmask.toInt())
            .build()
        val resp = dispatch(LexiconRequest.newBuilder().setSearchWithSources(payload).build()) ?: return emptyList()
        if (!resp.hasSearchWithSourcesResult()) return emptyList()
        return resp.searchWithSourcesResult.rowsList.map(::taigiWordToRow)
    }

    /**
     * Dictionary tab hanzi-prefix lookup.
     *
     * 中文: Tab3 漢字前綴查詢 — query 必為漢字。供 Tab3 漢字 short-circuit 路徑使用。
     */
    fun searchByHanzi(
        query: String,
        inputMode: LexiconInputMode,
        limit: UInt,
        enabledSourcesBitmask: UInt,
    ): List<Row> {
        val payload = SearchByHanziRequest
            .newBuilder()
            .setQuery(query)
            .setInputMode(InputMode.forNumber(inputMode.protoValue) ?: InputMode.INPUT_MODE_UNSPECIFIED)
            .setLimit(limit.toInt())
            .setEnabledSourcesBitmask(enabledSourcesBitmask.toInt())
            .build()
        val resp = dispatch(LexiconRequest.newBuilder().setSearchByHanzi(payload).build()) ?: return emptyList()
        if (!resp.hasSearchByHanziResult()) return emptyList()
        return resp.searchByHanziResult.rowsList.map(::taigiWordToRow)
    }

    /**
     * Bundled-bigram lookup. Called by `NextWordService.predict` for dict rows.
     *
     * 中文: 內建 bigram 查詢(association.bin)— previousWord → 後續候選清單。NextWordService.predict 用來補 dict 來源預測。
     */
    fun assocLookup(
        previousWord: String,
        limit: UInt,
        enabledSourcesBitmask: UInt,
    ): List<AssocEntry> {
        val payload = AssocLookupRequest
            .newBuilder()
            .setPreviousWord(previousWord)
            .setLimit(limit.toInt())
            .setEnabledSourcesBitmask(enabledSourcesBitmask.toInt())
            .build()
        val resp = dispatch(LexiconRequest.newBuilder().setAssocLookup(payload).build()) ?: return emptyList()
        if (!resp.hasAssocLookupResult()) return emptyList()
        return resp.assocLookupResult.entriesList.map { e ->
            AssocEntry(
                previousWord = e.previousWord,
                candidateWord = e.candidateWord,
                candidateTl = e.candidateTl,
                count = e.count.toUInt(),
            )
        }
    }

    // region Classification (v3.5.7)

    /**
     * Classifier output — `inputType` is the platform `DictInputType`.
     * C-1 (v3.5.9 D) retired the TPS→TL pre-conversion; `searchKey` is
     * now an identity passthrough of the raw input. The `tps:` FST
     * family is queried directly via `SearchRequest.inputMode = Tps`.
     */
    data class ClassificationResult(
        val inputType: DictInputType,
        val searchKey: String,
    )

    /**
     * Classify `raw` into `(InputType, search_key)`. Single FFI hop —
     * `lexicon::classify_input` keeps tone / TPS detection inside Rust,
     * replacing the platform-side per-keystroke ladder that previously
     * chained multiple phonetics ops per keypress. See
     * `INVARIANT_LEX_INPUT_CLASSIFICATION_PRECEDENCE`.
     *
     * 中文: 把 raw 分類成 (InputType, searchKey) 二元組;v3.5.7 後改成單次 FFI,取代過去每按鍵都串多個 phonetics op 的階梯邏輯。
     */
    fun classifyInput(raw: String): ClassificationResult {
        val payload = ClassifyInputRequest.newBuilder().setRaw(raw).build()
        val resp = dispatch(LexiconRequest.newBuilder().setClassifyInput(payload).build())
            ?: return ClassificationResult(DictInputType.RomanWithoutTone, raw)
        if (!resp.hasClassifyInputResult()) {
            return ClassificationResult(DictInputType.RomanWithoutTone, raw)
        }
        val r = resp.classifyInputResult
        return ClassificationResult(
            inputType = platformInputType(r.inputType),
            searchKey = r.searchKey,
        )
    }

    /**
     * Resolve user's 12-toggle dictionary preferences into ready-to-send
     * filter bitmasks + enabled-source set. Single FFI hop replaces the
     * pre-v3.5.8 verbatim-mirrored `EnabledDictionaries` bit math.
     *
     * Call ONCE per query and pass the result down the search pipeline;
     * resolving again inside the Dictionary tab's badge filter would split the snapshot.
     *
     * 中文: 把使用者 12 個字典 toggle 解析成 (dictionaryFilterBitmask, assocLookupBitmask, enabledSources)。
     *       每次查詢「呼叫一次」,結果傳遞到整個 search 管線;Tab3 badge filter 不可重新解析(會把 snapshot 切兩份)。
     *       FFI 失敗時 fallback 跑平台側對齊版 compute_filters,避免 dev 環境 Rust .so 未重 build 時誤失能。
     */
    fun dictionaryFilters(toggles: DictionaryToggles): DictionaryFilters {
        val protoToggles = ProtoDictionaryToggles
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
            )
            .build()
        val payload = DictionaryFiltersRequest
            .newBuilder()
            .setToggles(protoToggles)
            .build()
        // Binary skew fallback: when method 18 dispatch fails (e.g. Kotlin
        // updated but Rust .so not rebuilt) but methods 12-17 still work,
        // the dev-only fallback would silently strip user-enabled dictionaries.
        // Mirror Rust `engine/lexicon/src/dictionary_filters.rs::compute_filters`
        // here so search/assoc call paths continue to honor user toggles.
        // Codex PR #210 r3182714295.
        val resp = dispatch(LexiconRequest.newBuilder().setDictionaryFilters(payload).build())
        if (resp == null || !resp.hasDictionaryFiltersResult()) {
            return platformFallbackFilters(toggles)
        }
        val r = resp.dictionaryFiltersResult
        return DictionaryFilters(
            dictionaryFilterBitmask = r.dictionaryFilterBitmask.toUInt(),
            assocLookupBitmask = r.assocLookupBitmask.toUInt(),
            enabledSources = r.enabledSourceCodesList
                .mapNotNull(::platformDictionarySource)
                .toSet(),
        )
    }

    /**
     * Dictionary tab short-circuit predicate. True iff `text` contains any CJK
     * codepoint (Unified + Extensions A-E). See
     * `INVARIANT_LEX_INPUT_CLASSIFICATION_HANZI_RANGE`.
     *
     * 中文: Tab3 漢字短路徑判斷 — text 含 CJK Unified + Ext A-E 任一字即 true;
     *       修正 v3.5.7 前 Kotlin Char.code(16-bit)漏判 Ext B/C/D/E 的舊 bug。
     */
    fun isHanzi(text: String): Boolean {
        val payload = IsHanziRequest.newBuilder().setText(text).build()
        val resp = dispatch(LexiconRequest.newBuilder().setIsHanzi(payload).build()) ?: return false
        if (!resp.hasIsHanziResult()) return false
        return resp.isHanziResult.isHanzi
    }

    /**
     * Map proto `InputType` to the platform `DictInputType`. Unspecified /
     * unrecognised values fall back to `RomanWithoutTone` (matches the
     * safe-fallback contract used by the dispatch error paths).
     * Mirrors iOS `RustEngineBridge.platformInputType` — must drift
     * together.
     */
    private fun platformInputType(proto: InputType): DictInputType =
        when (proto) {
            InputType.INPUT_TYPE_HANZI -> DictInputType.Hanzi
            InputType.INPUT_TYPE_ROMAN_WITH_TONE -> DictInputType.RomanWithTone
            else -> DictInputType.RomanWithoutTone
        }

    /**
     * Fallback only for platform/Rust binary skew where method 18 is absent.
     * Rust `engine/lexicon/src/dictionary_filters.rs::compute_filters` is
     * authoritative; keep this bit layout in sync with
     * `engine/protos/proto/lexicon.proto`. Bit positions pinned by the
     * 6 inline Rust golden tests. Mirrors iOS
     * `RustEngineBridge.platformFallbackFilters` — must drift together.
     */
    private fun platformFallbackFilters(toggles: DictionaryToggles): DictionaryFilters {
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

        val allAssocOn = toggles.kautian && toggles.taigitv && toggles.itaigi &&
            toggles.sitbut && toggles.taihoa && toggles.taijit &&
            toggles.kungge && toggles.stti && toggles.khpoo
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
        return DictionaryFilters(
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
     * CROSS-PLATFORM INVARIANT — bit positions mirror
     * `engine/lexicon/src/dictionary_reader.rs` (`KAUTIAN_SUBTAG_*` /
     * `WIRE_KAUTIAN_SUBCOLL_*`). Drift causes silent subcollection-filter divergence.
     */
    internal fun encodeKautianSubcollWire(toggles: DictionaryToggles): UInt {
        if (!toggles.kautian) return 0u
        val sub = toggles.kautianSubcoll
        // Accent order MUST match config.yaml `dialect_columns` (subtag bit = 1 + index).
        val accents = listOf(
            sub.lukang, sub.sansia, sub.taipak, sub.gilan, sub.tainan,
            sub.kaohsiung, sub.kinmen, sub.makung, sub.sintik, sub.taichung,
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

    private fun dispatch(lexiconRequest: LexiconRequest): LexiconResponse? {
        val request = Request
            .newBuilder()
            .setId(RustEngineBridge.nextRequestIdInternal())
            .setLexicon(lexiconRequest)
            .build()
        val responseBytes = try {
            RustEngineBridge.dispatchRaw(request.toByteArray())
        } catch (t: Throwable) {
            RustEngineBridge.backend.w(TAG, "dispatch failed", t)
            return null
        }
        val response = try {
            Response.parseFrom(responseBytes)
        } catch (t: Throwable) {
            RustEngineBridge.backend.w(TAG, "response parse failed", t)
            return null
        }
        if (response.errorValue != 0) {
            RustEngineBridge.backend.w(TAG, "engine returned error: ${response.error}")
            return null
        }
        return if (response.hasLexicon()) response.lexicon else null
    }

    private fun taigiWordToRow(proto: ProtoTaigiWord): Row =
        Row(
            id = proto.id,
            roman = proto.roman,
            hanzi = if (proto.hasHanji()) proto.hanji else null,
            lengthScore = if (proto.hasLengthScore()) proto.lengthScore else null,
            sourceBitmask = if (proto.hasSourceBitmask()) proto.sourceBitmask.toUInt() else null,
        )

    // region Lexicon ranking (1 op) — production caller for `engine/ranking/`

    /**
     * Production caller for the Rust ranking pipeline. Single FFI
     * round-trip runs dedup → score → sort → (TPS-gated) display-dedup
     * atomically inside `engine/ranking/`. Mirrors iOS
     * `RustEngineBridge.processCandidates`.
     *
     * `tpsDedupEnabled` is platform-decided per audit § 3 — pass
     * `settings?.inputMode == "tps"` from the call site. Engine never
     * derives it from any config field.
     *
     * `nowMs` is caller-supplied for deterministic recency-window math
     * in tests; production passes `System.currentTimeMillis()`.
     *
     * In `BuildConfig.DEBUG` builds, requests + emits the per-candidate
     * `ScoreBreakdown` so dogfood traces include the score arithmetic.
     * Release builds skip the breakdown (zero serialization overhead).
     *
     * 中文: 排序生產入口 — 單次 FFI 跑完 dedup→score→sort→(TPS 模式)display-dedup;
     *       tpsDedupEnabled 由平台端決定(讀 settings.inputMode == "tps"),Engine 不自行推。
     */
    fun processCandidates(
        raw: List<TaigiWord>,
        normalizedInput: String,
        tpsDedupEnabled: Boolean,
        frequencyData: Map<String, FrequencyData>,
        nowMs: Long,
    ): List<TaigiWord> {
        val detailed = processCandidatesDetailed(
            raw = raw,
            normalizedInput = normalizedInput,
            tpsDedupEnabled = tpsDedupEnabled,
            frequencyData = frequencyData,
            nowMs = nowMs,
            includeBreakdown = BuildConfig.DEBUG,
        )
        if (BuildConfig.DEBUG && detailed.breakdowns.size == detailed.ranked.size) {
            for (i in detailed.ranked.indices) {
                val word = detailed.ranked[i]
                val b = detailed.breakdowns[i]
                RustEngineBridge.backend.d(
                    "RustEngineBridge",
                    "[SCORE] input='$normalizedInput' | ${word.roman} ${word.hanzi ?: ""}: " +
                        "user=${b.userFreqScore} recency=${b.recencyBonus} exact=${b.exactBonus} " +
                        "close=${b.closenessBonus} base=${b.baseFreqScore} " +
                        "completion=${b.completionPenalty} total=${b.total}",
                )
            }
        }
        return detailed.ranked
    }

    /**
     * Test seam — same FFI call as [processCandidates], plus access to the
     * per-candidate [RustEngineBridge.ScoreBreakdown] payload. Production
     * code stays on [processCandidates] which discards the breakdown after
     * debug logging.
     *
     * NOTE: `src/test/` JVM tests cannot exercise this seam because
     * `System.loadLibrary("rust_taigi")` fails on host JVM. Bridge
     * parity is verified by the Rust workspace tests + iOS XCTest
     * (links the xcframework) + Android instrumented dogfood.
     *
     * 中文: 測試用入口,可取出每筆候選的 ScoreBreakdown(六項分數);production 走 processCandidates 即可。
     */
    fun processCandidatesDetailed(
        raw: List<TaigiWord>,
        normalizedInput: String,
        tpsDedupEnabled: Boolean,
        frequencyData: Map<String, FrequencyData>,
        nowMs: Long,
        includeBreakdown: Boolean,
    ): RustEngineBridge.CandidateRanking {
        val payloadBuilder = ProcessCandidatesRequest
            .newBuilder()
            .setNormalizedInput(normalizedInput)
            .setTpsDedupEnabled(tpsDedupEnabled)
            .setNowMs(nowMs)
            .setIncludeBreakdown(includeBreakdown)
        for (word in raw) {
            payloadBuilder.addRaw(taigiWordToProto(word))
        }
        payloadBuilder.addAllFreq(RustEngineBridge.frequencyDataToProtoEntries(frequencyData))
        val resp = rankingDispatch(
            lexiconRequest = LexiconRequest.newBuilder().setProcessCandidates(payloadBuilder.build()).build(),
            op = "processCandidates",
        )
        if (resp == null) {
            return RustEngineBridge.CandidateRanking(
                ranked = fallbackRanked(raw),
                breakdowns = emptyList(),
            )
        }
        if (!resp.hasProcessCandidatesResult()) {
            RustEngineBridge.recordFailure("processCandidates", "missing process_candidates_result")
            return RustEngineBridge.CandidateRanking(
                ranked = fallbackRanked(raw),
                breakdowns = emptyList(),
            )
        }
        val result = resp.processCandidatesResult
        val ranked = result.rankedList.map(::taigiWordFromProto)
        val breakdowns = result.breakdownList.map(::scoreBreakdownFromProto)
        return RustEngineBridge.CandidateRanking(ranked = ranked, breakdowns = breakdowns)
    }

    /**
     * Ranking-pipeline dispatch — mirrors the pre-split
     * `RustEngineBridge.lexiconDispatch(op)`: `sendRawBytes` JNI hop and
     * `RustEngineBridge.recordFailure(op, ...)` on every failure mode
     * (`response decode failed`, non-OK `engine returned X` with code,
     * `missing lexicon payload`). Distinct from this class's broader
     * [dispatch] helper which routes legacy lexicon read-path errors
     * through `backend.w` only — that pre-existing behaviour is preserved
     * for `install` / `search*` / `assocLookup` / `classifyInput` /
     * `dictionaryFilters` / `isHanzi`, but the ranking public API must
     * keep its op-named diagnostics for telemetry parity with iOS
     * `RustEngineBridge.lexiconDispatch`.
     */
    private fun rankingDispatch(
        lexiconRequest: LexiconRequest,
        op: String,
    ): LexiconResponse? {
        val request = Request
            .newBuilder()
            .setId(RustEngineBridge.nextRequestIdInternal())
            .setLexicon(lexiconRequest)
            .build()
        val response = RustEngineBridge.sendRawBytes(request.toByteArray())
        if (response == null) {
            RustEngineBridge.recordFailure(op, "response decode failed")
            return null
        }
        if (response.error != ErrorCode.OK) {
            RustEngineBridge.recordFailure(op, "engine returned ${response.error}", response.error.number)
            return null
        }
        if (!response.hasLexicon()) {
            RustEngineBridge.recordFailure(op, "missing lexicon payload")
            return null
        }
        return response.lexicon
    }

    /**
     * Raw-list fallback on the FFI error path. When the Rust lexicon
     * dispatch fails (encode / decode error, non-OK engine response,
     * or missing payload variant), return the input list unchanged.
     *
     * Simplified in the v3.5.3 follow-up (PR #192): previously this
     * delegated to Kotlin-side dedup helpers (since removed) as a
     * defense-in-depth dedup. That silently masked Rust dispatch bugs
     * by producing a near-correct candidate list.
     */
    private fun fallbackRanked(raw: List<TaigiWord>): List<TaigiWord> = raw

    private fun taigiWordToProto(word: TaigiWord): ProtoTaigiWord {
        val builder = ProtoTaigiWord
            .newBuilder()
            .setId(word.id.toLong())
            .setRoman(word.roman)
        word.hanzi?.let { builder.setHanji(it) }
        word.lengthScore?.let { builder.setLengthScore(it) }
        word.sourceBitmask?.let { builder.setSourceBitmask(it) }
        return builder.build()
    }

    private fun taigiWordFromProto(proto: ProtoTaigiWord): TaigiWord =
        TaigiWord(
            id = proto.id.toInt(),
            roman = proto.roman,
            hanzi = if (proto.hasHanji()) proto.hanji else null,
            lengthScore = if (proto.hasLengthScore()) proto.lengthScore else null,
            sourceBitmask = if (proto.hasSourceBitmask()) proto.sourceBitmask else null,
        )

    private fun scoreBreakdownFromProto(proto: ProtoScoreBreakdown): RustEngineBridge.ScoreBreakdown =
        RustEngineBridge.ScoreBreakdown(
            userFreqScore = proto.userFreqScore,
            recencyBonus = proto.recencyBonus,
            exactBonus = proto.exactBonus,
            completionPenalty = proto.completionPenalty,
            closenessBonus = proto.closenessBonus,
            baseFreqScore = proto.baseFreqScore,
        )

    // endregion Lexicon ranking
}
