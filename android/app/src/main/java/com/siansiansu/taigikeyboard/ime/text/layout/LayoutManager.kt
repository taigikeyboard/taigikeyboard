
package com.siansiansu.taigikeyboard.ime.text.layout

import android.content.Context
import com.squareup.moshi.JsonAdapter
import com.squareup.moshi.Moshi
import com.squareup.moshi.Types
import com.squareup.moshi.kotlin.reflect.KotlinJsonAdapterFactory
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.Subtype
import com.siansiansu.taigikeyboard.ime.text.key.KeyData
import com.siansiansu.taigikeyboard.ime.text.key.KeyTypeAdapter
import com.siansiansu.taigikeyboard.ime.text.key.KeyVariation
import com.siansiansu.taigikeyboard.ime.text.key.KeyVariationAdapter
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardMode
private typealias LTN = Pair<LayoutType, String>

class LayoutManager(
    private val context: Context,
    private val prefs: PrefHelper
) {

    /**
     * Loads the layout for the specified type and name.
     *
     * @returns the [LayoutData] or null.
     */
    private fun loadLayout(ltn: LTN?) = loadLayout(ltn?.first, ltn?.second)
    private fun loadLayout(type: LayoutType?, name: String?): LayoutData? {
        if (type == null || name == null) {
            return null
        }
        val rawJsonData: String = try {
            context.assets.open("ime/text/$type/$name.json").bufferedReader().use { it.readText() }
        } catch (e: Exception) {
            android.util.Log.e("LayoutManager", "Failed to load layout $type/$name", e)
            null
        } ?: return null
        val moshi = Moshi.Builder()
            .add(KotlinJsonAdapterFactory())
            .add(LayoutTypeAdapter())
            .add(KeyTypeAdapter())
            .add(KeyVariationAdapter())
            .build()
        val layoutAdapter = moshi.adapter(LayoutData::class.java)
        val layoutData = layoutAdapter.fromJson(rawJsonData)
        if (layoutData != null && name?.contains("phah_taigi") == true) {
            android.util.Log.d("LayoutManager", "Loaded phahTaigi layout: ${layoutData.name}, rows=${layoutData.arrangement.size}")
            layoutData.arrangement.forEachIndexed { rowIndex, row ->
                android.util.Log.d("LayoutManager", "  Row $rowIndex: ${row.size} keys - ${row.map { it.label }.joinToString(" ")}")
            }
        }
        return layoutData
    }

    private fun loadExtendedPopups(subtype: Subtype): Map<String, List<KeyData>> {
        val lang = subtype.locale.language

        // 檢查是否為台語佈局並載入對應的聲調 popup
        val inputMode = prefs.inputMode
        val taigiMap = when {
            // 根據 inputMode 載入對應的台語 popup
            lang == "nan" || subtype.layout == "qwerty_poj" || subtype.layout == "qwerty_tl" -> {
                when (inputMode) {
                    "poj" -> loadExtendedPopupsInternal("ime/text/characters/extended_popups/taigi_poj.json")
                    "tl" -> loadExtendedPopupsInternal("ime/text/characters/extended_popups/taigi_tl.json")
                    else -> null
                }
            }
            else -> null
        }

        // If Taigi popup loaded successfully, return it; otherwise return empty map
        return taigiMap ?: mapOf()
    }

    private fun loadExtendedPopupsInternal(path: String): Map<String, List<KeyData>>? {
        val rawJsonData: String = try {
            context.assets.open(path).bufferedReader().use { it.readText() }
        } catch (e: Exception) {
            null
        } ?: return null
        val moshi = Moshi.Builder()
            .add(KotlinJsonAdapterFactory())
            .add(KeyTypeAdapter())
            .build()
        val mapAdaptor: JsonAdapter<Map<String, List<KeyData>>> =
            moshi.adapter(
                Types.newParameterizedType(
                    Map::class.java,
                    String::class.java,
                    Types.newParameterizedType(
                        List::class.java,
                        KeyData::class.java
                    )
                )
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
        extension: LTN? = null
    ): ComputedLayoutData {
        val computedArrangement: ComputedLayoutDataArrangement = mutableListOf()

        val mainLayout = loadLayout(main)
        val modifierLayout =  loadLayout(modifier)
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

        // TODO: rewrite this part
        if (keyboardMode == KeyboardMode.CHARACTERS) {
            val extendedPopups = loadExtendedPopups(subtype)
            for (computedRow in computedArrangement) {
                for (keyData in computedRow) {
                    if (keyData.variation != KeyVariation.ALL) {
                        if (keyData.variation == KeyVariation.NORMAL ||
                            keyData.variation == KeyVariation.PASSWORD) {
                            if (extendedPopups.containsKey(keyData.label + "~normal")) {
                                keyData.popup.addAll(extendedPopups[keyData.label + "~normal"] ?: listOf())
                            }
                        }
                        if (keyData.variation == KeyVariation.EMAIL_ADDRESS ||
                            keyData.variation == KeyVariation.URI) {
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
            mainLayout?.name ?: "computed",  // 使用 mainLayout 的名稱
            mainLayout?.direction ?: "ltr",
            computedArrangement
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
        overrideIsTranslateSwapped: Boolean? = null
    ): ComputedLayoutData {
        var main: LTN? = null
        var modifier: LTN? = null
        var extension: LTN? = null

        // 根據 isTranslateSwapped 決定使用半形或全形標點的佈局
        // 優先使用傳入的 override 值，避免 DataStore 非同步讀取導致的時序問題
        val isTranslateSwapped = overrideIsTranslateSwapped ?: prefs.isTranslateSwapped
        val modSuffix = if (isTranslateSwapped) "fullwidth" else "halfwidth"
        val symbolsSuffix = if (isTranslateSwapped) "fullwidth" else "default"

        when (keyboardMode) {
            KeyboardMode.CHARACTERS -> {
                // 選擇佈局：優先檢查 phahTaigiLayoutEnabled
                val layoutName = if (prefs.phahTaigiLayoutEnabled) {
                    // phahTaigi 佈局：根據 isTranslateSwapped 選擇全形/半形
                    val suffix = if (isTranslateSwapped) "fullwidth" else "halfwidth"
                    "qwerty_phah_taigi_$suffix"
                } else {
                    // 原有邏輯：根據 inputMode 選擇 poj/tl
                    when (prefs.inputMode) {
                        "poj" -> "qwerty_poj"
                        "tl" -> "qwerty_tl"
                        else -> "qwerty_poj"
                    }
                }
                android.util.Log.d("LayoutManager", "Loading layout: $layoutName (phahTaigi=${prefs.phahTaigiLayoutEnabled}, isTranslateSwapped=$isTranslateSwapped)")
                main = LTN(LayoutType.CHARACTERS, layoutName)
                // phahTaigi 使用專用的 modifier（移除底部 "-" 按鍵）
                val modifierName = if (prefs.phahTaigiLayoutEnabled) {
                    "phah_taigi_$modSuffix"
                } else {
                    "default_$modSuffix"
                }
                modifier = LTN(LayoutType.CHARACTERS_MOD, modifierName)
                extension = LTN(LayoutType.EXTENSION, "number_row")
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
                extension = LTN(LayoutType.EXTENSION, "number_row")
            }
            KeyboardMode.SYMBOLS2 -> {
                main = LTN(LayoutType.SYMBOLS2, "western_$symbolsSuffix")
                modifier = LTN(LayoutType.SYMBOLS2_MOD, "default_$modSuffix")
            }
            KeyboardMode.CLIPBOARD -> {
                // CLIPBOARD mode doesn't use layout files, return empty layout
            }
        }

        return mergeLayouts(keyboardMode, subtype, main, modifier, extension)
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
        overrideIsTranslateSwapped: Boolean? = null
    ): ComputedLayoutData {
        return computeLayoutFor(keyboardMode, subtype, overrideIsTranslateSwapped)
    }
}
