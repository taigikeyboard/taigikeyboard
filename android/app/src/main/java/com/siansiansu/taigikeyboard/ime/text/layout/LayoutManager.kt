// Keyboard layout manager: picks the layout files for a keyboard mode, merges them (LayoutAssets), puts an
// active custom layout's keys in (CustomLayoutTables), injects extended popups, and emits
// ComputedLayoutData for KeyboardImeRoot.

package com.siansiansu.taigikeyboard.ime.text.layout

import android.content.Context
import com.siansiansu.taigikeyboard.ime.core.CompositionRoot
import com.siansiansu.taigikeyboard.ime.core.logging.debug
import com.siansiansu.taigikeyboard.ime.settings.PrefHelper
import com.siansiansu.taigikeyboard.ime.text.key.KeyCode
import com.siansiansu.taigikeyboard.ime.text.key.KeyData
import com.siansiansu.taigikeyboard.ime.text.key.KeyType
import com.siansiansu.taigikeyboard.ime.text.key.KeyVariation
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardMode
import com.squareup.moshi.JsonAdapter
import com.squareup.moshi.Types
private typealias LTN = Pair<LayoutType, String>

class LayoutManager(
    private val context: Context,
    private val prefs: PrefHelper,
) {
    private val compositionRoot = CompositionRoot.shared(context)
    private val logger = compositionRoot.logger

    private val assets = compositionRoot.layoutAssets

    companion object {
        private const val TAG = "LayoutManager"
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
        val layoutData = assets.load(type, name)
        if (logger.isDebugEnabled && layoutData != null && name.contains("phah_taigi")) {
            logger.d(TAG, "[LAYOUT] Loaded phahTaigi layout: ${layoutData.name}, rows=${layoutData.arrangement.size}")
            layoutData.arrangement.forEachIndexed { rowIndex, row ->
                logger.d(TAG, "[LAYOUT]   Row $rowIndex: ${row.size} keys - ${row.map { it.label }.joinToString(" ")}")
            }
        }
        return layoutData
    }

    private fun loadExtendedPopups(): Map<String, List<KeyData>> {
        val taigiMap =
            when (prefs.inputMode) {
                "poj" -> loadExtendedPopupsInternal("ime/text/characters/extended_popups/taigi_poj.json")
                "tl" -> loadExtendedPopupsInternal("ime/text/characters/extended_popups/taigi_tl.json")
                else -> null
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
            LayoutAssets.moshi.adapter(
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
     * Merges the specified layouts (LTNs) ([LayoutAssets.merge]) and returns the computed layout,
     * with [custom]'s keys in place of the base's before extended popups are added, so the
     * popups follow each key's label.
     *
     * @param keyboardMode The keyboard mode for the returning [ComputedLayoutData].
     * @param main The main layout type and name.
     * @param modifier The modifier (mod) layout type and name.
     * @param extension The extension layout type and name.
     * @param custom The active custom layout, for the letter keyboard; [main] is its base table.
     * @param isFullWidthPunctuation The punctuation width [main] and [modifier] were picked for.
     * @returns a [ComputedLayoutData] object, regardless of the specified LTNs or errors.
     */
    private fun mergeLayouts(
        keyboardMode: KeyboardMode,
        main: LTN? = null,
        modifier: LTN? = null,
        extension: LTN? = null,
        custom: CustomLayout? = null,
        isFullWidthPunctuation: Boolean = false,
    ): ComputedLayoutData {
        val mainLayout = loadLayout(main)
        val merged = assets.merge(mainLayout, loadLayout(modifier), loadLayout(extension))
        val computedArrangement =
            if (custom == null) merged else compositionRoot.customLayoutTables.substitute(merged, custom, isFullWidthPunctuation)

        // TODO: consolidate the KeyVariation→popup-suffix mapping
        // (NORMAL/PASSWORD → "~normal", EMAIL_ADDRESS/URI → "~uri") to
        // remove the nested if/else duplication below.
        if (keyboardMode == KeyboardMode.CHARACTERS) {
            val extendedPopups = loadExtendedPopups()
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
            mainLayout?.name ?: "computed",
            mainLayout?.direction ?: "ltr",
            computedArrangement,
        )
    }

    /**
     * Computes a layout for [keyboardMode] and returns it.
     *
     * @param keyboardMode The keyboard mode for which the layout should be computed.
     * @param overrideIsFullWidthPunctuation Optional override for isFullWidthPunctuation; takes precedence over the prefs value.
     */
    private fun computeLayoutFor(
        keyboardMode: KeyboardMode,
        overrideIsFullWidthPunctuation: Boolean? = null,
        overrideInputMode: String? = null,
    ): ComputedLayoutData {
        var main: LTN? = null
        var modifier: LTN? = null
        var extension: LTN? = null

        // Prefer the passed-in override over prefs: an async DataStore read can otherwise lag the punctuation
        // (halfwidth / fullwidth) layout choice by a frame.
        val isFullWidthPunctuation = overrideIsFullWidthPunctuation ?: prefs.isFullWidthPunctuation
        val inputMode = overrideInputMode ?: prefs.inputMode
        val modSuffix = if (isFullWidthPunctuation) "fullwidth" else "halfwidth"
        val symbolsSuffix = if (isFullWidthPunctuation) "fullwidth" else "default"
        // English and TPS keep their own tables; a custom layout only replaces a romanization one.
        val custom = if (keyboardMode == KeyboardMode.CHARACTERS && (inputMode == "tl" || inputMode == "poj")) prefs.activeCustomLayout else null

        when (keyboardMode) {
            KeyboardMode.CHARACTERS -> {
                val layoutType = prefs.keyboardLayoutType
                // A custom layout keeps the table it was made from (its script) in TL and POJ alike.
                val layoutName = LayoutAssets.characterLayoutName(layoutType, custom?.script?.inputMode ?: inputMode, isFullWidthPunctuation)
                logger.debug(TAG) {
                    "[LAYOUT] Loading layout: $layoutName (inputMode=$inputMode, layoutType=$layoutType, custom=${custom?.id}, isFullWidthPunctuation=$isFullWidthPunctuation)"
                }
                main = LTN(LayoutType.CHARACTERS, layoutName)
                modifier = LTN(LayoutType.CHARACTERS_MOD, LayoutAssets.characterModifierName(layoutType, inputMode, isFullWidthPunctuation))
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
                // No number_row extension: the symbols layout already carries the digit row.
            }

            KeyboardMode.SYMBOLS2 -> {
                main = LTN(LayoutType.SYMBOLS2, "western_$symbolsSuffix")
                modifier = LTN(LayoutType.SYMBOLS2_MOD, "default_$modSuffix")
            }

            KeyboardMode.CLIPBOARD -> {
                // CLIPBOARD mode doesn't use layout files, return empty layout
            }
        }

        val result = mergeLayouts(keyboardMode, main, modifier, extension, custom, isFullWidthPunctuation)

        // 文/A is dropped where it could flip nothing — Romanization Only (always half-width)
        // and TPS (always full-width) — from every mode's rows (characters +
        // symbols mods all carry it) and SPACE (flexGrow 1) takes the freed width.
        if (!prefs.candidateDisplayMode.allowsSwapToggle || prefs.isTpsLayout) {
            result.arrangement.forEach { row -> row.removeAll { it.code == KeyCode.TRANSLATE } }
        }

        // Globe key toggle: skip English and TPS layouts (TPS has more keys, no room for globe)
        if (inputMode != "english" && inputMode != "tps" && keyboardMode == KeyboardMode.CHARACTERS) {
            if (prefs.isGlobeKeyEnabled) {
                // TL / POJ on the TPS layout use the globe-less tps_halfwidth mod — inject one
                // right after the symbols key, where every other mod carries it (iOS: bottom-row index 1)
                val bottomRow = result.arrangement.lastOrNull()
                if (bottomRow != null && bottomRow.none { it.code == KeyCode.LANGUAGE_SWITCH }) {
                    val symbolsIndex = bottomRow.indexOfFirst { it.code == KeyCode.VIEW_SYMBOLS }
                    bottomRow.add(
                        symbolsIndex + 1,
                        KeyData(
                            code = KeyCode.LANGUAGE_SWITCH,
                            label = "language_switch",
                            type = KeyType.SYSTEM_GUI,
                        ),
                    )
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
     * Fetches the computed layout for the given [keyboardMode].
     * This function computes the layout synchronously and returns it directly.
     *
     * @param keyboardMode The keyboard mode for which the layout should be computed.
     * @param overrideIsFullWidthPunctuation Optional override for isFullWidthPunctuation; takes precedence over the prefs value.
     * @return The computed layout data.
     */
    fun fetchComputedLayout(
        keyboardMode: KeyboardMode,
        overrideIsFullWidthPunctuation: Boolean? = null,
        overrideInputMode: String? = null,
    ): ComputedLayoutData = computeLayoutFor(keyboardMode, overrideIsFullWidthPunctuation, overrideInputMode)

    /**
     * Fetches a layout for preview mode, always using Taigi mode (never English).
     * This matches iOS KeyboardPreviewPanel behavior.
     */
    fun fetchComputedLayoutForPreview(keyboardMode: KeyboardMode): ComputedLayoutData {
        // Force Taigi mode — if user's inputMode is "english", override to "tl"
        val previewInputMode = if (prefs.inputMode == "english") "tl" else prefs.inputMode
        return computeLayoutFor(
            keyboardMode,
            overrideIsFullWidthPunctuation = false,
            overrideInputMode = previewInputMode,
        )
    }
}
