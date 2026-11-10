// Reads keyboard layout JSON from assets/ime/text/ and merges the extension + main + modifier layers
// into one arrangement. Shared by LayoutManager (the keyboard) and CustomLayoutTables (the tables a
// custom layout is made from), so both see the same rows. Owns the Moshi instance with the
// LayoutType / KeyType / KeyVariation adapters and the letter-keyboard file names.

package com.siansiansu.taigikeyboard.ime.text.layout

import android.content.Context
import com.siansiansu.taigikeyboard.ime.text.key.KeyData
import com.siansiansu.taigikeyboard.ime.text.key.KeyTypeAdapter
import com.siansiansu.taigikeyboard.ime.text.key.KeyVariationAdapter
import com.squareup.moshi.Moshi
import com.squareup.moshi.kotlin.reflect.KotlinJsonAdapterFactory

/** Layout JSON over [readText], which returns a file's text or null when it cannot be read. */
class LayoutAssets(
    private val readText: (path: String) -> String?,
) {
    /** Parses `ime/text/<type>/<name>.json`; every call returns fresh, mutable [KeyData]. */
    fun load(
        type: LayoutType,
        name: String,
    ): LayoutData? = readText("ime/text/$type/$name.json")?.let { layoutAdapter.fromJson(it) }

    /**
     * Merges the layers into one arrangement:
     *   e e e e e e e e e e      e = extension
     *   c c c c c c c c c c      c = main
     *    c c c c c c c c c       m = mod
     *   m c c c c c c c c m
     *   m m m m m m m m m m
     * The last main row is wrapped by the first and last key of the modifier's first row.
     */
    fun merge(
        main: LayoutData?,
        modifier: LayoutData?,
        extension: LayoutData?,
    ): ComputedLayoutDataArrangement {
        val arrangement: ComputedLayoutDataArrangement = mutableListOf()

        extension?.arrangement?.firstOrNull()?.let { arrangement.add(it.toMutableList()) }

        if (main != null && modifier != null) {
            for (mainRowI in main.arrangement.indices) {
                val mainRow = main.arrangement[mainRowI]
                if (mainRowI + 1 < main.arrangement.size) {
                    arrangement.add(mainRow.toMutableList())
                } else {
                    val mergedRow = mutableListOf<KeyData>()
                    val firstModRow = modifier.arrangement.firstOrNull()
                    val firstModKey = firstModRow?.firstOrNull()
                    if (firstModKey != null) {
                        mergedRow.add(firstModKey)
                    }
                    mergedRow.addAll(mainRow)
                    val lastModKey = firstModRow?.lastOrNull()
                    if (lastModKey != null && firstModKey != lastModKey) {
                        mergedRow.add(lastModKey)
                    }
                    arrangement.add(mergedRow)
                }
            }
            for (modRowI in 1 until modifier.arrangement.size) {
                arrangement.add(modifier.arrangement[modRowI].toMutableList())
            }
        } else if (main != null) {
            main.arrangement.forEach { arrangement.add(it.toMutableList()) }
        } else if (modifier != null) {
            modifier.arrangement.forEach { arrangement.add(it.toMutableList()) }
        }
        return arrangement
    }

    /** The letter keyboard of [layoutType] in [inputMode] (TL / POJ): digit row, letter rows, modifier rows. */
    fun letterKeyboard(
        layoutType: String,
        inputMode: String,
        isFullWidthPunctuation: Boolean,
        load: (LayoutType, String) -> LayoutData? = ::load,
    ): ComputedLayoutDataArrangement =
        merge(
            load(LayoutType.CHARACTERS, characterLayoutName(layoutType, inputMode, isFullWidthPunctuation)),
            load(LayoutType.CHARACTERS_MOD, characterModifierName(layoutType, inputMode, isFullWidthPunctuation)),
            load(LayoutType.EXTENSION, "number_row"),
        )

    companion object {
        /** Thread-safe and reusable. */
        val moshi: Moshi =
            Moshi
                .Builder()
                .add(KotlinJsonAdapterFactory())
                .add(LayoutTypeAdapter())
                .add(KeyTypeAdapter())
                .add(KeyVariationAdapter())
                .build()

        private val layoutAdapter = moshi.adapter(LayoutData::class.java)

        /** Reads the app's bundled assets; an unreadable file is null, reported to [onError]. */
        fun forContext(
            context: Context,
            onError: (path: String, error: Exception) -> Unit,
        ): LayoutAssets =
            LayoutAssets { path ->
                try {
                    context.assets
                        .open(path)
                        .bufferedReader()
                        .use { it.readText() }
                } catch (e: Exception) {
                    onError(path, e)
                    null
                }
            }

        /** The `characters/` table of [layoutType] in [inputMode]. */
        fun characterLayoutName(
            layoutType: String,
            inputMode: String,
            isFullWidthPunctuation: Boolean,
        ): String {
            val modSuffix = if (isFullWidthPunctuation) "fullwidth" else "halfwidth"
            val moeSuffix = if (isFullWidthPunctuation) "_fullwidth" else ""
            return when {
                inputMode == "english" -> "qwerty_english"
                inputMode == "tps" -> "tps"
                else ->
                    when (layoutType) {
                        "phahTaigi" -> "qwerty_phah_taigi_$modSuffix"
                        "moe1" -> if (inputMode == "poj") "qwerty_moe1_poj$moeSuffix" else "qwerty_moe1$moeSuffix"
                        "moe2" -> if (inputMode == "poj") "qwerty_moe2_poj$moeSuffix" else "qwerty_moe2$moeSuffix"
                        "tps" -> "tps"
                        // "qwerty", and any value a hand edit could leave behind.
                        else -> if (inputMode == "poj") "qwerty_poj" else "qwerty_tl"
                    }
            }
        }

        /** The `characters/mod/` table of [layoutType] in [inputMode]. */
        fun characterModifierName(
            layoutType: String,
            inputMode: String,
            isFullWidthPunctuation: Boolean,
        ): String {
            val modSuffix = if (isFullWidthPunctuation) "fullwidth" else "halfwidth"
            return when {
                inputMode == "english" -> "english"
                inputMode == "tps" -> "tps_halfwidth"
                layoutType == "phahTaigi" -> "phah_taigi_$modSuffix"
                layoutType == "moe1" -> "moe1_$modSuffix"
                layoutType == "moe2" -> "moe2_$modSuffix"
                layoutType == "tps" -> "tps_halfwidth"
                else -> "default_$modSuffix"
            }
        }
    }
}
