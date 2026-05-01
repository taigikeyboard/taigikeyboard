// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion
package com.siansiansu.taigikeyboard.ime.dictionary

import com.siansiansu.taigikeyboard.ime.core.settings.EngineSettings

/**
 * 辭典開關設定(從 EngineSettings 建立)
 *
 * Bitmask bit layout must match dictionary.bin / association.bin:
 *   0=kautian  1=taigitv  2=itaigi  3=sitbut  4=taihoa  5=taijit
 *   6=kungge   7=stti     8=khpoo   9=khiin   10=dev    11=lkk
 *   12=is_variant  13-15=reserved
 */
data class EnabledDictionaries(
    val kautian: Boolean, // 教育部臺灣台語常用詞辭典
    val taigitv: Boolean, // 台語新詞辭庫
    val itaigi: Boolean, // iTaigi 華台對照典
    val sitbut: Boolean, // 台灣植物名彙
    val taihoa: Boolean, // 台華線頂對照典
    val taijit: Boolean, // 台日大辭典
    val kungge: Boolean, // 台語工藝詞庫
    val stti: Boolean, // 學科術語辭典
    val khpoo: Boolean, // 腔口補充資料
    val variant: Boolean, // 異用字
    val khiin: Boolean, // 在來字
    val lkk: Boolean, // LKK漢羅合用建議用字
) {
    /** 是否全部開啟(10 個主要來源) */
    fun allEnabled(): Boolean = kautian && taigitv && itaigi && sitbut && taihoa && taijit && kungge && stti && khpoo && lkk

    /** 轉換為 dictionary bitmask(bits 0-11) */
    fun sourceBitmask(): Int {
        var mask = 0
        if (kautian) mask = mask or (1 shl 0)
        if (taigitv) mask = mask or (1 shl 1)
        if (itaigi) mask = mask or (1 shl 2)
        if (sitbut) mask = mask or (1 shl 3)
        if (taihoa) mask = mask or (1 shl 4)
        if (taijit) mask = mask or (1 shl 5)
        if (kungge) mask = mask or (1 shl 6)
        if (stti) mask = mask or (1 shl 7)
        if (khpoo) mask = mask or (1 shl 8)
        // khiin = bit 9 (handled separately in filter)
        // dev = bit 10 (always included)
        if (lkk) mask = mask or (1 shl 11)
        return mask
    }

    /** 轉換為 association bitmask(bits 0-8,對應 association.bin 的 9 個來源) */
    fun associationBitmask(): Int = sourceBitmask() and 0x1FF

    /**
     * 完整 dictionary.bin filter bitmask — 所有使用者切換(含 variant + khiin)
     * 都精確編碼進 bits 0-12。配合 `engine/lexicon/src/search.rs::build_filter`
     * 的 layout: bit 9=khiin、bit 10=dev (常開)、bit 12=variant。Tab3 +
     * autocomplete 都走此 mask;不要再用 `Int.MAX` / `UInt.MAX_VALUE` short-circuit
     * (會錯誤強制 enable variant + khiin)。
     */
    fun dictionaryFilterBitmask(): Int {
        var mask = sourceBitmask() // bits 0-8, 11
        if (khiin) mask = mask or (1 shl 9)
        mask = mask or (1 shl 10) // dev always included
        if (variant) mask = mask or (1 shl 12)
        return mask
    }

    /** association 的 9 個來源是否全部開啟 */
    fun allAssociationSourcesEnabled(): Boolean = kautian && taigitv && itaigi && sitbut && taihoa && taijit && kungge && stti && khpoo

    companion object {
        /**
         * Build from an [EngineSettings] live-read view. Each boolean is
         * read individually — matches iOS, which also does per-field
         * reads. A mid-iteration DataStore update could in theory produce
         * a split snapshot, but this mirrors the behavior iOS ships and
         * is consistent with the live-read contract in
         * `EngineSettingsProvider`.
         */
        fun fromSettings(settings: EngineSettings): EnabledDictionaries =
            EnabledDictionaries(
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
            )
    }
}
