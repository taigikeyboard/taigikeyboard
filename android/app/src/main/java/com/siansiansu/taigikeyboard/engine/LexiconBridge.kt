package com.siansiansu.taigikeyboard.engine

import com.siansiansu.taigikeyboard.engine.proto.AssocLookupRequest
import com.siansiansu.taigikeyboard.engine.proto.ClassifyInputRequest
import com.siansiansu.taigikeyboard.engine.proto.InputMode
import com.siansiansu.taigikeyboard.engine.proto.InputType
import com.siansiansu.taigikeyboard.engine.proto.InstallRequest
import com.siansiansu.taigikeyboard.engine.proto.IsHanziRequest
import com.siansiansu.taigikeyboard.engine.proto.LexiconRequest
import com.siansiansu.taigikeyboard.engine.proto.LexiconResponse
import com.siansiansu.taigikeyboard.engine.proto.Request
import com.siansiansu.taigikeyboard.engine.proto.Response
import com.siansiansu.taigikeyboard.engine.proto.SearchByHanziRequest
import com.siansiansu.taigikeyboard.engine.proto.SearchRequest
import com.siansiansu.taigikeyboard.engine.proto.SearchWithSourcesRequest
import com.siansiansu.taigikeyboard.engine.proto.TaigiWord
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
    enum class LexiconInputType(val protoValue: Int) {
        UNSPECIFIED(0),
        ROMAN_NO_TONE(1),
        ROMAN_WITH_TONE(2),
        HANZI(3),
    }

    /** Lexicon engine `inputMode` enum (mirrors proto `InputMode`). */
    enum class LexiconInputMode(val protoValue: Int) {
        UNSPECIFIED(0),
        TL(1),
        POJ(2),
        TPS(3),
    }

    /**
     * Install (or atomically reinstall) the lexicon engine state. Called
     * from `AppInitializer` after `copyAssetsIfNeeded` finishes; idempotent.
     * Returns `null` on failure (logged via `RustEngineBridge.diagnostics()`).
     */
    fun install(
        triePath: String,
        dictionaryBinPath: String,
        associationBinPath: String,
        dictionaryVersion: UInt,
    ): InstallStats? {
        val payload = InstallRequest.newBuilder()
            .setTriePath(triePath)
            .setDictionaryBinPath(dictionaryBinPath)
            .setAssociationBinPath(associationBinPath)
            .setDictionaryVersion(dictionaryVersion.toInt())
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
     */
    fun search(
        input: String,
        inputType: LexiconInputType,
        inputMode: LexiconInputMode,
        limit: UInt,
        tpsOrMappedToER: Boolean,
        enabledSourcesBitmask: UInt,
    ): List<Row> {
        val payload = SearchRequest.newBuilder()
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

    /** Tab3 multi-source dictionary lookup. */
    fun searchWithSources(
        input: String,
        inputMode: LexiconInputMode,
        limit: UInt,
        enabledSourcesBitmask: UInt,
    ): List<Row> {
        val payload = SearchWithSourcesRequest.newBuilder()
            .setInput(input)
            .setInputMode(InputMode.forNumber(inputMode.protoValue) ?: InputMode.INPUT_MODE_UNSPECIFIED)
            .setLimit(limit.toInt())
            .setEnabledSourcesBitmask(enabledSourcesBitmask.toInt())
            .build()
        val resp = dispatch(LexiconRequest.newBuilder().setSearchWithSources(payload).build()) ?: return emptyList()
        if (!resp.hasSearchWithSourcesResult()) return emptyList()
        return resp.searchWithSourcesResult.rowsList.map(::taigiWordToRow)
    }

    /** Tab3 hanzi-prefix dictionary lookup. */
    fun searchByHanzi(
        query: String,
        inputMode: LexiconInputMode,
        limit: UInt,
        enabledSourcesBitmask: UInt,
    ): List<Row> {
        val payload = SearchByHanziRequest.newBuilder()
            .setQuery(query)
            .setInputMode(InputMode.forNumber(inputMode.protoValue) ?: InputMode.INPUT_MODE_UNSPECIFIED)
            .setLimit(limit.toInt())
            .setEnabledSourcesBitmask(enabledSourcesBitmask.toInt())
            .build()
        val resp = dispatch(LexiconRequest.newBuilder().setSearchByHanzi(payload).build()) ?: return emptyList()
        if (!resp.hasSearchByHanziResult()) return emptyList()
        return resp.searchByHanziResult.rowsList.map(::taigiWordToRow)
    }

    /** Bundled-bigram lookup. Called by `NextWordService.predict` for dict rows. */
    fun assocLookup(
        previousWord: String,
        limit: UInt,
        enabledSourcesBitmask: UInt,
    ): List<AssocEntry> {
        val payload = AssocLookupRequest.newBuilder()
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

    /** Classifier output — `inputType` is the platform `DictInputType`. */
    data class ClassificationResult(
        val inputType: DictInputType,
        val searchKey: String,
    )

    /**
     * Classify `raw` into `(InputType, search_key)`. Single FFI hop replaces
     * the per-keystroke ladder of `hasToneMarks` / `containsTPS` /
     * `tpsToTL` calls. See `INVARIANT_LEX_INPUT_CLASSIFICATION_PRECEDENCE`.
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
     * Tab3 short-circuit predicate. True iff `text` contains any CJK
     * codepoint (Unified + Extensions A-E). See
     * `INVARIANT_LEX_INPUT_CLASSIFICATION_HANZI_RANGE`.
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
            InputType.INPUT_TYPE_HANZI           -> DictInputType.Hanzi
            InputType.INPUT_TYPE_ROMAN_WITH_TONE -> DictInputType.RomanWithTone
            else                                 -> DictInputType.RomanWithoutTone
        }

    // endregion Classification

    private fun dispatch(lexiconRequest: LexiconRequest): LexiconResponse? {
        val request = Request.newBuilder()
            .setId(RustEngineBridge.nextRequestIdInternal())
            .setLexicon(lexiconRequest)
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
        return if (response.hasLexicon()) response.lexicon else null
    }

    private fun taigiWordToRow(proto: TaigiWord): Row {
        return Row(
            id = proto.id,
            roman = proto.roman,
            hanzi = if (proto.hasHanji()) proto.hanji else null,
            lengthScore = if (proto.hasLengthScore()) proto.lengthScore else null,
            sourceBitmask = if (proto.hasSourceBitmask()) proto.sourceBitmask.toUInt() else null,
        )
    }
}
