// 鍵盤佈局管理器 — 從 assets/layouts/ 讀 JSON,合併 main + modifier + extension 三層,
// 注入 extended popups,輸出 ComputedLayoutData 給 KeyboardImeRoot 使用。
// Moshi 解析快取 + LayoutType / KeyType / KeyVariation Adapter 集中於此。

package com.siansiansu.taigikeyboard.ime.text.layout

import android.content.Context
import com.siansiansu.taigikeyboard.ime.core.CompositionRoot
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.logging.debug
import com.siansiansu.taigikeyboard.ime.core.Subtype
import com.siansiansu.taigikeyboard.ime.text.key.KeyCode
import com.siansiansu.taigikeyboard.ime.text.key.KeyData
import com.siansiansu.taigikeyboard.ime.text.key.KeyType
import com.siansiansu.taigikeyboard.ime.text.key.KeyTypeAdapter
import com.siansiansu.taigikeyboard.ime.text.key.KeyVariation
import com.siansiansu.taigikeyboard.ime.text.key.KeyVariationAdapter
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardMode
import com.squareup.moshi.JsonAdapter
import com.squareup.moshi.Moshi
import com.squareup.moshi.Types
import com.squareup.moshi.kotlin.reflect.KotlinJsonAdapterFactory
private typealias LTN = Pair<LayoutType, String>

class LayoutManager(
    private val context: Context,
    private val prefs: PrefHelper,
) {
    private val logger = CompositionRoot.shared(context).logger

    companion object {
        private const val TAG = "LayoutManager"

        /** Cached Moshi instance (thread-safe, reusable) */
        private val moshi: Moshi =
            Moshi
                .Builder()
                .add(KotlinJsonAdapterFactory())
                .add(LayoutTypeAdapter())
                .add(KeyTypeAdapter())
                .add(KeyVariationAdapter())
                .build()
    }

    /**
     * Loads the layout for the specified type and name.
     *
     * @returns the [LayoutData] or null.
     */
    private fun loadLayout(ltn: LTN?) = loadLayout(ltn?.first, ltn?.second)

    private fun loadLayout(
        type: LayoutType?,
        name: String?,
    ): LayoutData? {
        if (type == null || name == null) {
            return null
        }
        val rawJsonData: String =
            try {
                context.assets
                    .open("ime/text/$type/$name.json")
                    .bufferedReader()
                    .use { it.readText() }
            } catch (e: Exception) {
                logger.e(TAG, "[LAYOUT] Failed to load layout $type/$name", e)
                null
            } ?: return null
        val layoutAdapter = moshi.adapter(LayoutData::class.java)
        val layoutData = layoutAdapter.fromJson(rawJsonData)
        if (logger.isDebugEnabled && layoutData != null && name?.contains("phah_taigi") == true) {
            logger.d(TAG, "[LAYOUT] Loaded phahTaigi layout: ${layoutData.name}, rows=${layoutData.arrangement.size}")
            layoutData.arrangement.forEachIndexed { rowIndex, row ->
                logger.d(TAG, "[LAYOUT]   Row $rowIndex: ${row.size} keys - ${row.map { it.label }.joinToString(" ")}")
            }
        }
        return layoutData
    }

    private fun loadExtendedPopups(subtype: Subtype): Map<String, List<KeyData>> {
        val inputMode = prefs.inputMode

        // English / TPS mode：不載入台語 popup
        if (inputMode == "english" || inputMode == "tps") {
            return mapOf()
        }

        val lang = subtype.locale.language

        // 檢查是否為台語佈局並載入對應的聲調 popup
        val taigiMap =
            when {
                // 根據 inputMode 載入對應的台語 popup
                lang == "nan" || subtype.layout == "qwerty_poj" || subtype.layout == "qwerty_tl" -> {
                    when (inputMode) {
                        "poj" -> loadExtendedPopupsInternal("ime/text/characters/extended_popups/taigi_poj.json")
                        "tl" -> loadExtendedPopupsInternal("ime/text/characters/extended_popups/taigi_tl.json")
                        else -> null
                    }
                }

                else -> {
                    null
                }
            }

        // If Taigi popup loaded successfully, return it; otherwise return empty map
        return taigiMap ?: mapOf()
    }

    private fun loadExtendedPopupsInternal(path: String): Map<String, List<KeyData>>? {
        val rawJsonData: String =
            try {
                context.assets
                    .open(path)
                    .bufferedReader()
                    .use { it.readText() }
            } catch (e: Exception) {
                null
            } ?: return null
        val mapAdaptor: JsonAdapter<Map<String, List<KeyData>>> =
            moshi.adapter(
                Types.newParameterizedType(
                    Map::class.java,
                    String::class.java,
                    Types.newParameterizedType(
                        List::class.java,
                        KeyData::class.java,
                    ),
                ),
            )
        return mapAdaptor.fromJson(rawJsonData)
    }

    /**
     * Merges the specified layouts (LTNs) and returns the computed layout.
     * The computed layout may looks like this:
     *   e e e e e e e e e e      e = extension
     *   c c c c c c c c c c      c = main
     *    c c c c c c c c c       m = mod
     *   m c c c c c c c c m
     *   m m m m m m m m m m
     *
     * @param keyboardMode The keyboard mode for the returning [ComputedLayoutData].
     * @param subtype The subtype used for populating the extended popups.
     * @param main The main layout type and name.
     * @param modifier The modifier (mod) layout type and name.
     * @param extension The extension layout type and name.
     * @returns a [ComputedLayoutData] object, regardless of the specified LTNs or errors.
     */
    private fun mergeLayouts(
        keyboardMode: KeyboardMode,
        subtype: Subtype,
        main: LTN? = null,
        modifier: LTN? = null,
        extension: LTN? = null,
    ): ComputedLayoutData {
        val computedArrangement: ComputedLayoutDataArrangement = mutableListOf()

        val mainLayout = loadLayout(main)
        val modifierLayout = loadLayout(modifier)
        val extensionLayout = loadLayout(extension)

        if (extensionLayout != null) {
            val row = extensionLayout.arrangement.firstOrNull()
            if (row != null) {
                computedArrangement.add(row.toMutableList())
            }
        }

        if (mainLayout != null && modifierLayout != null) {
            for (mainRowI in mainLayout.arrangement.indices) {
                val mainRow = mainLayout.arrangement[mainRowI]
                if (mainRowI + 1 < mainLayout.arrangement.size) {
                    computedArrangement.add(mainRow.toMutableList())
                } else {
                    // merge main and mod here
                    val mergedRow = mutableListOf<KeyData>()
                    val firstModRow = modifierLayout.arrangement.firstOrNull()
                    val firstModKey = firstModRow?.firstOrNull()
                    if (firstModKey != null) {
                        mergedRow.add(firstModKey)
                    }
                    mergedRow.addAll(mainRow)
                    val lastModKey = firstModRow?.lastOrNull()
                    if (lastModKey != null && firstModKey != lastModKey) {
                        mergedRow.add(lastModKey)
                    }
                    computedArrangement.add(mergedRow)
                }
            }
            for (modRowI in 1 until modifierLayout.arrangement.size) {
                val modRow = modifierLayout.arrangement[modRowI]
                computedArrangement.add(modRow.toMutableList())
            }
        } else if (mainLayout != null && modifierLayout == null) {
            for (mainRow in mainLayout.arrangement) {
                computedArrangement.add(mainRow.toMutableList())
            }
        } else if (mainLayout == null && modifierLayout != null) {
            for (modRow in modifierLayout.arrangement) {
                computedArrangement.add(modRow.toMutableList())
            }
        }

        // TODO: consolidate the KeyVariation→popup-suffix mapping
        // (NORMAL/PASSWORD → "~normal", EMAIL_ADDRESS/URI → "~uri") to
        // remove the nested if/else duplication below.
        if (keyboardMode == KeyboardMode.CHARACTERS) {
            val extendedPopups = loadExtendedPopups(subtype)
            for (computedRow in computedArrangement) {
                for (keyData in computedRow) {
                    if (keyData.variation != KeyVariation.ALL) {
                        if (keyData.variation == KeyVariation.NORMAL ||
                            keyData.variation == KeyVariation.PASSWORD
                        ) {
                            if (extendedPopups.containsKey(keyData.label + "~normal")) {
                                keyData.popup.addAll(extendedPopups[keyData.label + "~normal"] ?: listOf())
                            }
                        }
                        if (keyData.variation == KeyVariation.EMAIL_ADDRESS ||
                            keyData.variation == KeyVariation.URI
                        ) {
                            if (extendedPopups.containsKey(keyData.label + "~uri")) {
                                keyData.popup.addAll(extendedPopups[keyData.label + "~uri"] ?: listOf())
                            }
                        }
                    }
                    if (extendedPopups.containsKey(keyData.label)) {
                        keyData.popup.addAll(extendedPopups[keyData.label] ?: listOf())
                    }
                }
            }
        }

        return ComputedLayoutData(
            keyboardMode,
            mainLayout?.name ?: "computed", // 使用 mainLayout 的名稱
            mainLayout?.direction ?: "ltr",
            computedArrangement,
        )
    }

    /**
     * Computes a layout for [keyboardMode] based on the given [subtype] and returns it.
     *
     * TODO: used layouts for symbols should be dynamically selected based on subtype
     *
     * @param keyboardMode The keyboard mode for which the layout should be computed.
     * @param subtype The subtype which localizes the computed layout.
     * @param overrideIsTranslateSwapped Optional override for isTranslateSwapped, 優先於 prefs 的值
     */
    private fun computeLayoutFor(
        keyboardMode: KeyboardMode,
        subtype: Subtype,
        overrideIsTranslateSwapped: Boolean? = null,
        overrideInputMode: String? = null,
    ): ComputedLayoutData {
        var main: LTN? = null
        var modifier: LTN? = null
        var extension: LTN? = null

        // 根據 isTranslateSwapped 決定使用半形或全形標點的佈局
        // 優先使用傳入的 override 值，避免 DataStore 非同步讀取導致的時序問題
        val isTranslateSwapped = overrideIsTranslateSwapped ?: prefs.isTranslateSwapped
        val inputMode = overrideInputMode ?: prefs.inputMode
        val modSuffix = if (isTranslateSwapped) "fullwidth" else "halfwidth"
        val symbolsSuffix = if (isTranslateSwapped) "fullwidth" else "default"

        when (keyboardMode) {
            KeyboardMode.CHARACTERS -> {
                // 選擇佈局：English mode > keyboardLayoutType
                val layoutName =
                    when {
                        inputMode == "english" -> {
                            "qwerty_english"
                        }

                        inputMode == "tps" -> {
                            "tps"
                        }

                        else -> {
                            when (prefs.keyboardLayoutType) {
                                "phahTaigi" -> {
                                    // phahTaigi 佈局：根據 isTranslateSwapped 選擇全形/半形
                                    val suffix = if (isTranslateSwapped) "fullwidth" else "halfwidth"
                                    "qwerty_phah_taigi_$suffix"
                                }

                                "moe1" -> {
                                    val suffix = if (isTranslateSwapped) "_fullwidth" else ""
                                    when (inputMode) {
                                        "poj" -> "qwerty_moe1_poj$suffix"
                                        else -> "qwerty_moe1$suffix"
                                    }
                                }

                                "moe2" -> {
                                    val suffix = if (isTranslateSwapped) "_fullwidth" else ""
                                    when (inputMode) {
                                        "poj" -> "qwerty_moe2_poj$suffix"
                                        else -> "qwerty_moe2$suffix"
                                    }
                                }

                                "tps" -> {
                                    "tps"
                                }

                                "qwerty" -> {
                                    // 原有邏輯：根據 inputMode 選擇 poj/tl
                                    when (inputMode) {
                                        "poj" -> "qwerty_poj"
                                        "tl" -> "qwerty_tl"
                                        else -> "qwerty_tl"
                                    }
                                }

                                else -> {
                                    // 向後相容：使用舊的 phahTaigiLayoutEnabled
                                    if (prefs.phahTaigiLayoutEnabled) {
                                        val suffix = if (isTranslateSwapped) "fullwidth" else "halfwidth"
                                        "qwerty_phah_taigi_$suffix"
                                    } else {
                                        when (inputMode) {
                                            "poj" -> "qwerty_poj"
                                            "tl" -> "qwerty_tl"
                                            else -> "qwerty_tl"
                                        }
                                    }
                                }
                            }
                        }
                    }
                logger.debug(TAG) {
                    "[LAYOUT] Loading layout: $layoutName (inputMode=$inputMode, layoutType=${prefs.keyboardLayoutType}, isTranslateSwapped=$isTranslateSwapped)"
                }
                main = LTN(LayoutType.CHARACTERS, layoutName)
                // 根據模式選擇 modifier
                val modifierName =
                    when {
                        inputMode == "english" -> "english"
                        inputMode == "tps" -> "tps_halfwidth"
                        prefs.keyboardLayoutType == "phahTaigi" -> "phah_taigi_$modSuffix"
                        prefs.keyboardLayoutType == "moe1" -> "moe1_$modSuffix"
                        prefs.keyboardLayoutType == "moe2" -> "moe2_$modSuffix"
                        prefs.keyboardLayoutType == "tps" -> "tps_halfwidth"
                        prefs.phahTaigiLayoutEnabled -> "phah_taigi_$modSuffix"
                        else -> "default_$modSuffix"
                    }
                modifier = LTN(LayoutType.CHARACTERS_MOD, modifierName)
                // TPS layout already has enough rows — skip number row
                if (inputMode != "tps" && prefs.keyboardLayoutType != "tps") {
                    extension = LTN(LayoutType.EXTENSION, "number_row")
                }
            }

            KeyboardMode.NUMERIC -> {
                main = LTN(LayoutType.NUMERIC, "default")
            }

            KeyboardMode.NUMERIC_ADVANCED -> {
                main = LTN(LayoutType.NUMERIC_ADVANCED, "default")
            }

            KeyboardMode.PHONE -> {
                main = LTN(LayoutType.PHONE, "default")
            }

            KeyboardMode.PHONE2 -> {
                main = LTN(LayoutType.PHONE2, "default")
            }

            KeyboardMode.SYMBOLS -> {
                main = LTN(LayoutType.SYMBOLS, "western_$symbolsSuffix")
                modifier = LTN(LayoutType.SYMBOLS_MOD, "default_$modSuffix")
                // 不需要 number_row extension，因為 symbols layout 已經包含數字列
            }

            KeyboardMode.SYMBOLS2 -> {
                main = LTN(LayoutType.SYMBOLS2, "western_$symbolsSuffix")
                modifier = LTN(LayoutType.SYMBOLS2_MOD, "default_$modSuffix")
            }

            KeyboardMode.CLIPBOARD -> {
                // CLIPBOARD mode doesn't use layout files, return empty layout
            }
        }

        val result = mergeLayouts(keyboardMode, subtype, main, modifier, extension)

        // Globe key toggle: skip English and TPS layouts (TPS has more keys, no room for globe)
        if (inputMode != "english" && inputMode != "tps" && keyboardMode == KeyboardMode.CHARACTERS) {
            if (prefs.isGlobeKeyEnabled) {
                // TPS layouts lack globe key in JSON — inject one after space in bottom row
                val bottomRow = result.arrangement.lastOrNull()
                if (bottomRow != null && bottomRow.none { it.code == KeyCode.LANGUAGE_SWITCH }) {
                    val spaceIndex = bottomRow.indexOfFirst { it.code == 32 }
                    if (spaceIndex >= 0) {
                        bottomRow.add(
                            spaceIndex + 1,
                            KeyData(
                                code = KeyCode.LANGUAGE_SWITCH,
                                label = "language_switch",
                                type = KeyType.SYSTEM_GUI,
                            ),
                        )
                    }
                }
            } else {
                // Remove globe key from all non-English character layouts
                for (row in result.arrangement) {
                    row.removeAll { it.code == KeyCode.LANGUAGE_SWITCH }
                }
            }
        }

        return result
    }

    /**
     * Fetches the computed layout for the given [keyboardMode]/[subtype] combo.
     * This function computes the layout synchronously and returns it directly.
     *
     * @param keyboardMode The keyboard mode for which the layout should be computed.
     * @param subtype The subtype which localizes the computed layout.
     * @param overrideIsTranslateSwapped Optional override for isTranslateSwapped, 優先於 prefs 的值
     * @return The computed layout data.
     */
    fun fetchComputedLayout(
        keyboardMode: KeyboardMode,
        subtype: Subtype,
        overrideIsTranslateSwapped: Boolean? = null,
        overrideInputMode: String? = null,
    ): ComputedLayoutData = computeLayoutFor(keyboardMode, subtype, overrideIsTranslateSwapped, overrideInputMode)

    /**
     * Fetches a layout for preview mode, always using Taigi mode (never English).
     * This matches iOS KeyboardPreviewPanel behavior.
     */
    fun fetchComputedLayoutForPreview(
        keyboardMode: KeyboardMode,
        subtype: Subtype,
    ): ComputedLayoutData {
        // Force Taigi mode — if user's inputMode is "english", override to "tl"
        val previewInputMode = if (prefs.inputMode == "english") "tl" else prefs.inputMode
        return computeLayoutFor(
            keyboardMode,
            subtype,
            overrideIsTranslateSwapped = false,
            overrideInputMode = previewInputMode,
        )
    }
}
