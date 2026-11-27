// The built-in tables a custom layout is made from: per base × script, the places of the half-width
// letter keyboard (as LayoutAssets merges it), and a catalog of every key a custom layout can place,
// in both punctuation widths, plus the built-in long-press lists and the characters a list can take.
// Puts an active custom layout's keys and long-press lists into the merged keyboard for LayoutManager.
// Android keeps its JSON tables (roadmap § Model); the iOS counterpart is the table half of
// Layout/CustomLayout.swift.

package com.siansiansu.taigikeyboard.ime.text.layout

import com.siansiansu.taigikeyboard.engine.ToneVariations
import com.siansiansu.taigikeyboard.ime.text.key.KeyCode
import com.siansiansu.taigikeyboard.ime.text.key.KeyData
import com.siansiansu.taigikeyboard.ime.text.key.KeyType
import com.siansiansu.taigikeyboard.ime.text.key.KeyVariation
import java.text.Normalizer

/**
 * One key of a drawn board ([LayoutKeyGrid]): its face, its place (null on the digit row), and whether
 * it is a removed key's place, which the editor can fill again (a removed character key has an empty
 * label; the hidden emoji key keeps its face, so its place keeps its width).
 */
data class GridKey(
    val face: Face,
    val place: LayoutPlace?,
    val isRemoved: Boolean = false,
) {
    sealed interface Face {
        data class Character(
            val label: String,
        ) : Face

        data class Function(
            val key: FunctionKey,
        ) : Face
    }
}

/** Over the layout [assets] and the engine's tone-mark table [toneVariations] (JVM tests pass their own). */
class CustomLayoutTables(
    private val assets: LayoutAssets,
    private val toneVariations: () -> ToneVariations,
) {
    /** A base table's places below the digit row (if any), in reading order. */
    class BaseTable(
        /** The half-width label of each character place. */
        val labels: List<String>,
        val functionKeys: List<FunctionKey>,
        /** The row of each function place. */
        val functionRows: List<Int>,
        /** The digit row's labels; empty for a table without one (TPS). */
        val digits: List<String>,
        /** The places of each row below the digit row (globe, `@` and `/` are not places). */
        val placeRows: List<List<LayoutPlace>>,
        /** The long-press entries of each half-width character key, by label (tone marks not included). */
        val keyPopups: Map<String, List<String>>,
    ) {
        /** How many rows of the merged keyboard come before the places: the digit row, if any. */
        val digitRowCount: Int
            get() = if (digits.isEmpty()) 0 else 1
    }

    /** One place of a merged row: what a custom layout fills, or what the keyboard puts back itself. */
    private sealed interface Piece {
        data class Character(
            val key: KeyData,
        ) : Piece

        /** TRANSLATE is one place over its three variation copies (email / normal / URL fields). */
        data class Function(
            val functionKey: FunctionKey,
            val keys: List<KeyData>,
        ) : Piece

        /** `@` / `/`, shown in email / URL fields only. */
        data class InputType(
            val key: KeyData,
        ) : Piece

        data object Globe : Piece
    }

    private val tables: Map<Pair<String, LayoutScript>, BaseTable>

    /** Half-width label → (half-width key, full-width key). Never handed out without a deep copy. */
    private val catalog: Map<String, Pair<KeyData, KeyData>>

    /**
     * Letters offered in the spare tray: a–z, o͘, ⁿ, `ee`, `-`, then every multi-letter key of the romanization
     * tables (MOE2 `ph`, `tsh`, POJ `ch` …), whatever the base, then [TRAY_FINALS]. iOS
     * `CustomLayout.trayLetters`.
     */
    val trayLetters: List<String>

    /** Punctuation and symbols offered in the spare tray: the common symbol page (the 123 key). */
    val traySymbols: List<String>

    /** Half labels of both symbol pages: what any layout, TPS included, can put on its board. */
    private val symbolPageLabels: Set<String>

    /**
     * Glyphs a TPS layout can put on its board: the TPS table's (its `,` is a symbol-page key), then each
     * key's long-press variants (ㆴ on ㄅ …) in table order; never a digit (digits stay long-press
     * entries) or a Latin letter. Kept out of [catalog], so a romanization layout never takes one. iOS
     * `CustomLayout.tpsGlyphs`.
     */
    private val tpsGlyphs: List<String>

    private val tpsGlyphSet: Set<String>

    /**
     * Tone-marked letters for long-press lists, one row per lowercase base (Shift uppercases them on
     * the keyboard): a e i o oo o͘ u m n ng first, then any other base; each row the engine's universal
     * list (both scripts' marks, both tone-9 marks). iOS `CustomLayout.toneRows`.
     */
    val toneRows: List<List<String>> by lazy {
        val universal = toneVariations().universal
        val toneBases = universal.keys.filter { it == it.lowercase() }.toSet()
        val leadingBases = listOf("a", "e", "i", "o", "oo", "o\u0358", "u", "m", "n", "ng").filter { it in toneBases }
        (leadingBases + (toneBases - leadingBases.toSet()).sorted()).map { base ->
            // ⁿ rides the `n` list but is no tone mark: it sits with the letters.
            universal[base].orEmpty().filterNot { it == TRAY_NASAL_MARKER }
        }
    }

    /**
     * Symbols for long-press lists: both symbol pages in both widths, then every entry of the built-in
     * long-press lists; half-width forms first. iOS `CustomLayout.calloutSymbols`.
     */
    val calloutSymbols: List<String> by lazy {
        val symbolPages =
            listOf(LayoutType.SYMBOLS, LayoutType.SYMBOLS2).flatMap { type ->
                listOf("western_default", "western_fullwidth").flatMap { name ->
                    assets
                        .load(type, name)
                        ?.arrangement
                        .orEmpty()
                        .flatten()
                }
            }
        val builtInPopups = (symbolPages + catalog.values.flatMap { it.toList() }).flatMap(::popupEntries)
        halfWidthFirst(composedUnique(symbolPages.mapNotNull(::entry) + builtInPopups).filterNot { it in trayLetters || it in DRAWN_AS_EMOJI })
    }

    // The long-press tables above and below serve the editor only, so the keyboard never parses them.

    init {
        // Bases share files (PhahTaigi has one table for both scripts, modifiers ignore the script,
        // every romanization base has the same digit row): parse each once. Tables never mutate a parsed key.
        val parsed = HashMap<Pair<LayoutType, String>, LayoutData?>()
        val load = { type: LayoutType, name: String -> parsed.getOrPut(type to name) { assets.load(type, name) } }

        val tables = mutableMapOf<Pair<String, LayoutScript>, BaseTable>()
        val catalog = mutableMapOf<String, Pair<KeyData, KeyData>>()
        // Full-width symbol label \u2192 its half-width label, from the symbol page pairs (`\uff0c` \u2192 `,`).
        val halfWidthLabels = HashMap<String, String>()

        fun addToCatalog(
            half: KeyData,
            full: KeyData,
        ) {
            catalog.putIfAbsent(half.label, half to full)
        }

        fun addSymbolPage(type: LayoutType): List<List<KeyData>> {
            val halfPage = load(type, "western_default")?.arrangement.orEmpty()
            val fullPage = load(type, "western_fullwidth")?.arrangement.orEmpty()
            halfPage.zip(fullPage).forEach { (half, full) ->
                half.zip(full).forEach { (h, f) ->
                    addToCatalog(h, f)
                    halfWidthLabels.putIfAbsent(f.label, h.label)
                }
            }
            return halfPage
        }

        // The symbol page first: its `(` `)` carry full-width forms that MOE1's own keys leave out (a
        // base's own keys still win, see substitute).
        val symbolPage = addSymbolPage(LayoutType.SYMBOLS)

        val romanizationLabels = mutableListOf<String>()
        for (base in KeyboardLayoutOptions.romanization.map { it.key }) {
            for (script in LayoutScript.entries) {
                val halfKeyboard = assets.letterKeyboard(base, script.inputMode, isFullWidthPunctuation = false, load)
                val fullKeyboard = assets.letterKeyboard(base, script.inputMode, isFullWidthPunctuation = true, load)
                val digitRowCount = digitRowCount(base, script.inputMode)
                halfKeyboard.drop(digitRowCount).flatMap(::pieces).zip(fullKeyboard.drop(digitRowCount).flatMap(::pieces)).forEach { (h, f) ->
                    if (h is Piece.Character && f is Piece.Character) addToCatalog(h.key, f.key)
                }
                val table = baseTableOf(halfKeyboard, digitRowCount) { it } ?: continue
                romanizationLabels += table.labels
                tables[base to script] = table
            }
        }
        // TPS has one table (always full-width punctuation), filed under TL (CustomLayout's script is
        // unused for it); its places carry half-width labels, so a record reads `,` as on iOS.
        val tps = CustomLayout.TPS_BASE
        val tpsTable =
            baseTableOf(assets.letterKeyboard(tps, tps, isFullWidthPunctuation = true, load), digitRowCount(tps, tps)) { label ->
                halfWidthLabels[label] ?: label
            }
        tpsTable?.let { tables[CustomLayout.TPS_BASE to LayoutScript.TL] = it }
        this.tables = tables
        // After the tables, so a key they share keeps its form.
        val secondSymbolPage = addSymbolPage(LayoutType.SYMBOLS2)
        // No built-in table has \u207f, `ee` or the finals: each is a plain key, in both widths.
        (listOf(TRAY_NASAL_MARKER) + TRAY_VOWELS + TRAY_FINALS).forEach { addToCatalog(KeyData(0, it), KeyData(0, it)) }
        this.catalog = catalog

        val symbolLabels = symbolPage.flatten().map { it.label }
        val single = ('a'..'z').map(Char::toString) + listOf("o\u0358", TRAY_NASAL_MARKER) + TRAY_VOWELS + listOf("-")
        trayLetters = (single + romanizationLabels.filterNot { it in symbolLabels } + TRAY_FINALS).distinct().filter { it in catalog }
        traySymbols = symbolLabels.filterNot { it in trayLetters }.distinct()
        symbolPageLabels = (symbolLabels + secondSymbolPage.flatten().map { it.label }).toSet()

        val tpsBoardGlyphs = tpsTable?.labels.orEmpty().filterNot { it in symbolLabels }
        val tpsPopups = tpsTable?.keyPopups.orEmpty()
        val tpsVariants = tpsBoardGlyphs.flatMap { glyph -> tpsPopups[glyph].orEmpty().filter { it.first().code > 0x7F } }
        tpsGlyphs = (tpsBoardGlyphs + tpsVariants).distinct()
        tpsGlyphSet = tpsGlyphs.toSet()
    }

    /**
     * Characters a TPS key's long-press list can take beyond the symbols: every TPS glyph, then the
     * digits the built-in first-row lists hold (\u31a0 \u2192 1 \u2026). iOS `CustomLayout.tpsCalloutCharacters`.
     */
    private val tpsCalloutCharacters: List<String> by lazy { tpsGlyphs + "1234567890".map(Char::toString) }

    /** [entries] spelled as the tray's chips are (NFC), to tell which chips a list already holds. */
    fun trayForms(entries: List<String>): Set<String> = composedUnique(entries).toSet()

    /**
     * Whether [spare] can go on [place] of [layout]: a character place and a key placeable on its base
     * ([isPlaceable]) not on the board. A full-width form and a long-press-only symbol never are. iOS
     * `CustomLayout.canReplace`.
     */
    fun canReplace(
        layout: CustomLayout,
        place: LayoutPlace,
        spare: String,
    ): Boolean = place is LayoutPlace.Key && isPlaceable(layout, spare) && spare !in layout.keys

    /**
     * Whether a key labelled [label] can sit on [layout]'s board: a TPS layout takes TPS glyphs and
     * symbol-page keys, a romanization layout any catalog key. iOS `CustomLayout.isPlaceable`.
     */
    private fun isPlaceable(
        layout: CustomLayout,
        label: String,
    ): Boolean = if (layout.isTps) label in tpsGlyphSet || label in symbolPageLabels else label in catalog

    /** Spare letters offered in the tray, multi-letter keys included; on a TPS layout, spare TPS glyphs. */
    fun trayLetters(layout: CustomLayout): List<String> = layout.spares(if (layout.isTps) tpsGlyphs else trayLetters)

    /**
     * A TPS layout's long-press group in the tray: the TPS glyphs on the board and the digits (the spare
     * glyphs already show with the letters). Empty for a romanization layout. iOS `CustomLayout.trayTpsCallouts`.
     */
    fun trayTpsCallouts(layout: CustomLayout): List<String> {
        if (!layout.isTps) return emptyList()
        val spare = trayLetters(layout).toSet()
        return tpsCalloutCharacters.filterNot { it in spare }
    }

    /**
     * The built-in long-press list of the key labelled [label] on [layout]'s base and script: the base's
     * own key for that label (else the catalog's half-width key), then its tone marks
     * ([CustomLayout.toneMarks]: a universal layout's are both scripts'). A TPS glyph with variants
     * lists itself first, as the keyboard shows it (`tpsPopupWithBaseGlyph`; iOS `TPSCallouts.calloutChars`).
     */
    fun defaultCallouts(
        layout: CustomLayout,
        label: String,
    ): List<String> {
        val ownPopups = baseTable(layout.base, layout.script)?.keyPopups?.get(label) ?: catalog[label]?.first?.let(::popupEntries).orEmpty()
        if (layout.isTps && label in tpsGlyphSet && ownPopups.isNotEmpty()) return listOf(label) + ownPopups
        return ownPopups + layout.toneMarks(toneVariations())[label].orEmpty()
    }

    /**
     * The board labels of the keys [layout] gives their own long-press list: each half-width label and
     * its full-width form (TPS and full-width punctuation draw `，` for `,`), so their built-in hints hide.
     */
    fun customCalloutLabels(layout: CustomLayout): Set<String> = layout.callouts.keys.flatMapTo(HashSet()) { label -> listOfNotNull(label, catalog[label]?.second?.label) }

    /** The long-press list the key labelled [label] shows: [layout]'s own, else the built-in one. */
    fun callouts(
        layout: CustomLayout,
        label: String,
    ): List<String> = layout.callouts[label] ?: defaultCallouts(layout, label)

    /** [layout] with [entries] as the list of the key labelled [label] ([CustomLayout.withCallouts]). */
    fun withCallouts(
        layout: CustomLayout,
        label: String,
        entries: List<String>,
    ): CustomLayout = layout.withCallouts(label, entries, defaultCallouts(layout, label))

    /** The tray's symbol group for [layout]: spare symbols for the board, then [calloutSymbols], each once, half-width first. */
    fun traySymbols(layout: CustomLayout): List<String> = halfWidthFirst((layout.spares(traySymbols) + calloutSymbols).distinct())

    /**
     * [arrangement] — the keyboard [substitute] made from [layout], extended popups added — with each
     * board key's long-press list replaced by [layout]'s own, where it has one. An empty list leaves the
     * key without a long-press. [isFullWidthPunctuation] gives each entry the form the board would type
     * (`,` → `，`): the board's key with that label first (MOE1 `(` stays half-width), else the catalog
     * key; each entry once. Half-width keeps the list as stored, so a stored `，` stays full-width. iOS
     * `CustomLayout.typedCallouts`.
     */
    fun applyCallouts(
        arrangement: ComputedLayoutDataArrangement,
        layout: CustomLayout,
        isFullWidthPunctuation: Boolean,
    ) {
        if (layout.callouts.isEmpty()) return
        val placed = layout.keys.filterNotNullTo(LinkedHashSet())
        // A full-width key goes by its half-width label, as the list is stored. Placed keys only, the
        // first in reading order winning (iOS `halfWidthLabel`): `‧` and `•` share the full-width form `·`.
        val halfWidthLabels = HashMap<String, String>()
        placed.forEach { label -> catalog[label]?.let { (_, full) -> halfWidthLabels.putIfAbsent(full.label, label) } }
        val digitRowCount = baseTable(layout.base, layout.script)?.digitRowCount ?: return
        val boardKeys =
            arrangement.drop(digitRowCount).flatMap(::pieces).mapNotNull { piece ->
                val key = (piece as? Piece.Character)?.key ?: return@mapNotNull null
                val label = key.label.takeIf { it in placed } ?: halfWidthLabels[key.label] ?: return@mapNotNull null
                label to key
            }
        // What the board types for each half-width label right now: the key's own form at full width.
        val boardForms = if (isFullWidthPunctuation) boardKeys.associate { (label, key) -> label to key.label } else emptyMap()

        fun typed(entries: List<String>): List<String> =
            if (!isFullWidthPunctuation) {
                entries
            } else {
                entries.map { text -> boardForms[text] ?: catalog[text]?.second?.let(::entry) ?: text }.distinct()
            }
        for ((label, key) in boardKeys) {
            layout.callouts[label]?.let { entries -> key.popup = typed(entries).mapTo(mutableListOf(), KeyData::ofEntry) }
        }
    }

    /** A fresh layout: the [base] × [script] table unchanged; null for unknown layouts. */
    fun fresh(
        id: String,
        name: String,
        base: String,
        script: LayoutScript,
    ): CustomLayout? =
        baseTable(base, script)?.let { table ->
            CustomLayout(id, name, base, script, table.labels, table.functionKeys, hasEmojiKey = true)
        }

    /**
     * [layout] shown where [use] says. Picking one script lays the keys as arranged on that script's
     * table row by row. Most bases have the same places in both scripts; where a row has more (QWERTY's
     * Pe̍h-ōe-jī `o͘`), removed keys fill its end, and where it has fewer, its last keys go back to the
     * tray with their long-press lists. A TPS layout stays TPS, and a romanization one never becomes
     * one. Mirrors iOS `CustomLayout.use`.
     */
    fun withUse(
        layout: CustomLayout,
        use: LayoutUse,
    ): CustomLayout {
        if (layout.isTps || use == LayoutUse.TPS) return layout
        val script = use.script ?: return layout.copy(isUniversal = true)
        val oldCounts = characterCountsPerRow(baseTable(layout.base, layout.script) ?: return layout)
        val newCounts = characterCountsPerRow(baseTable(layout.base, script) ?: return layout)
        var start = 0
        val keys =
            buildList {
                oldCounts.zip(newCounts).forEach { (oldCount, newCount) ->
                    addAll(layout.keys.subList(start, start + minOf(oldCount, newCount)))
                    repeat(newCount - oldCount) { add(null) }
                    start += oldCount
                }
            }
        val kept = keys.filterNotNull().toSet()
        return layout.copy(script = script, isUniversal = false, keys = keys, callouts = layout.callouts.filterKeys { it in kept })
    }

    private fun characterCountsPerRow(table: BaseTable): List<Int> = table.placeRows.map { row -> row.count { it is LayoutPlace.Key } }

    /**
     * The board the editor draws: the digit row (no places) if the base has one, then every place of
     * [layout], a removed key and the hidden emoji key included ([CustomLayout.removedPlaces]).
     */
    fun editorRows(layout: CustomLayout): List<List<GridKey>> {
        val table = baseTable(layout.base, layout.script) ?: return emptyList()
        val removed = layout.removedPlaces
        val placeRows =
            table.placeRows.map { row ->
                row.map { place -> GridKey(layout.face(place), place, isRemoved = place in removed) }
            }
        if (table.digits.isEmpty()) return placeRows
        return listOf(table.digits.map { GridKey(GridKey.Face.Character(it), place = null) }) + placeRows
    }

    /** The board as the keyboard shows it (card previews): removed keys leave their row, an empty row goes too. */
    fun previewRows(layout: CustomLayout): List<List<GridKey>> = editorRows(layout).map { row -> row.filterNot { it.isRemoved } }.filter { it.isNotEmpty() }

    /** The table of [base] × [script] (TPS has one table, whatever the script); null for unknown layouts. */
    fun baseTable(
        base: String,
        script: LayoutScript,
    ): BaseTable? = tables[base to if (base == CustomLayout.TPS_BASE) LayoutScript.TL else script]

    /**
     * Whether [layout] can be shown: a known base, one entry per character place, every key placeable
     * on that base ([isPlaceable]), none twice, each row's function keys the base row's.
     */
    fun isValid(layout: CustomLayout): Boolean {
        val table = baseTable(layout.base, layout.script) ?: return false
        val placed = layout.keys.filterNotNull()
        return layout.keys.size == table.labels.size &&
            placed.all { isPlaceable(layout, it) } &&
            placed.toSet().size == placed.size &&
            functionKeysByRow(table, layout.functionKeys) == functionKeysByRow(table, table.functionKeys)
    }

    /**
     * [arrangement] — the merged base table of [layout] in the current punctuation width, digit row
     * (if any) first — with [layout]'s keys in its places. A moved key is the whole [KeyData], so its
     * inline popup moves with it: the base's own key for that label first (MOE1 `(` stays half-width),
     * else the catalog's in the current width (a TPS variant is a plain glyph key). Removed keys leave
     * their row and a row left empty leaves the keyboard. Globe, `@` and `/` are dropped; `@` / `/`
     * come back on the bottom row (iOS `CustomLayoutService.applyInputTypeKey`): right after a
     * bottom-row `,`, which then hides in email / URL fields, else right before space. A base without
     * them (TPS) keeps its `,` everywhere. The globe comes back in LayoutManager, after 123.
     */
    fun substitute(
        arrangement: ComputedLayoutDataArrangement,
        layout: CustomLayout,
        isFullWidthPunctuation: Boolean,
    ): ComputedLayoutDataArrangement {
        val table = baseTable(layout.base, layout.script) ?: return arrangement
        if (arrangement.size <= table.digitRowCount) return arrangement
        val rows = arrangement.drop(table.digitRowCount).map(::pieces)
        val characterCount = rows.sumOf { row -> row.count { it is Piece.Character } }
        if (characterCount != table.labels.size || layout.keys.size != characterCount || layout.functionKeys.size != table.functionKeys.size) {
            return arrangement
        }

        // The base's own keys, by half-width label and function key, read before any is placed.
        val ownKeys = HashMap<String, KeyData>()
        val ownFunctionKeys = HashMap<FunctionKey, List<KeyData>>()
        val inputTypeKeys = mutableListOf<KeyData>()
        var place = 0
        for (piece in rows.flatten()) {
            when (piece) {
                is Piece.Character -> ownKeys[table.labels[place++]] = piece.key
                is Piece.Function -> ownFunctionKeys[piece.functionKey] = piece.keys
                is Piece.InputType -> inputTypeKeys += piece.key
                Piece.Globe -> Unit
            }
        }

        var nextKey = 0
        var nextFunction = 0
        var comma: KeyData? = null
        val result: ComputedLayoutDataArrangement = arrangement.take(table.digitRowCount).toMutableList()
        for (row in rows) {
            val keys = mutableListOf<KeyData>()
            for (piece in row) {
                when (piece) {
                    is Piece.Character -> {
                        val label = layout.keys[nextKey++] ?: continue
                        val key = ownKeys[label] ?: catalogKey(label, isFullWidthPunctuation) ?: continue
                        keys += key.copy(variation = KeyVariation.ALL).also { if (label == ",") comma = it }
                    }

                    is Piece.Function -> {
                        val functionKey = layout.functionKeys[nextFunction++]
                        if (functionKey != FunctionKey.EMOJI || layout.hasEmojiKey) keys += ownFunctionKeys[functionKey].orEmpty()
                    }

                    is Piece.InputType, Piece.Globe -> Unit
                }
            }
            if (keys.isNotEmpty()) result += keys
        }

        if (inputTypeKeys.isEmpty()) return result
        val bottomRow = result.last()
        val commaIndex = bottomRow.indexOfFirst { it === comma }
        if (commaIndex >= 0) {
            bottomRow[commaIndex] = bottomRow[commaIndex].copy(variation = KeyVariation.NORMAL)
            bottomRow.addAll(commaIndex + 1, inputTypeKeys)
        } else {
            bottomRow.addAll(bottomRow.indexOfFirst { it.code == KeyCode.SPACE }.coerceAtLeast(0), inputTypeKeys)
        }
        return result
    }

    private fun catalogKey(
        label: String,
        isFullWidthPunctuation: Boolean,
    ): KeyData? =
        catalog[label]?.let { (half, full) -> (if (isFullWidthPunctuation) full else half).deepCopy() }
            // A TPS variant (ㆴ) has no key of its own in any table: code 0 types its label, as tps.json's glyphs.
            ?: label.takeIf { it in tpsGlyphSet }?.let { KeyData(0, it) }

    /** Each row's function keys, order ignored — what a reorder must keep. Null when the count is off. */
    private fun functionKeysByRow(
        table: BaseTable,
        keys: List<FunctionKey>,
    ): Map<Int, List<String>>? =
        if (keys.size != table.functionRows.size) {
            null
        } else {
            table.functionRows
                .zip(keys)
                .groupBy({ it.first }, { it.second.storageValue })
                .mapValues { it.value.sorted() }
        }

    private companion object {
        /** `ee` [ɛ], spelled the same in TL and POJ. iOS `CustomLayout.trayVowels`. */
        val TRAY_VOWELS = listOf("ee")

        /**
         * The nasal marker ⁿ (POJ `siaⁿ`). No built-in table has it on the board (only the `n` long-press);
         * Shift turns it into ᴺ in the engine. iOS `CustomLayout.trayNasalMarker`.
         */
        const val TRAY_NASAL_MARKER = "ⁿ"

        /**
         * Finals spelled the same in TL and POJ, most frequent first (share of dictionary syllables whose
         * final contains them). iOS `CustomLayout.trayFinals`.
         */
        val TRAY_FINALS =
            listOf("an", "ia", "in", "au", "ong", "ian", "un", "ai", "iu", "ann", "ang", "ui", "inn", "iau", "er", "ir", "or")

        /**
         * Built-in long-press symbols that draw as colour emoji in the tray; left out of it, as on iOS.
         * Listed, not detected: `Character.isEmoji` needs API 35.
         */
        val DRAWN_AS_EMOJI = setOf("▶", "◀")

        /**
         * 1 when the letter keyboard of [layoutType] in [inputMode] opens with a digit row
         * ([LayoutAssets.hasDigitRow]), which stays the base's; 0 for TPS. iOS `CustomLayout.firstEditableRow`.
         */
        fun digitRowCount(
            layoutType: String,
            inputMode: String,
        ): Int = if (LayoutAssets.hasDigitRow(layoutType, inputMode)) 1 else 0

        /**
         * The places of the half-width letter keyboard [halfKeyboard] below its first [digitRowCount]
         * rows, each character place labelled [halfWidthLabel] of its key's label; null when it has no
         * rows below the digit row.
         */
        fun baseTableOf(
            halfKeyboard: List<List<KeyData>>,
            digitRowCount: Int,
            halfWidthLabel: (String) -> String,
        ): BaseTable? {
            val rows = halfKeyboard.drop(digitRowCount).map(::pieces)
            if (rows.isEmpty()) return null
            val characterKeys = rows.flatten().filterIsInstance<Piece.Character>().map { it.key }
            // Row numbers count the digit row.
            val functions =
                rows.withIndex().flatMap { (index, row) -> row.filterIsInstance<Piece.Function>().map { index + digitRowCount to it.functionKey } }
            var nextKey = 0
            var nextFunction = 0
            return BaseTable(
                labels = characterKeys.map { halfWidthLabel(it.label) },
                functionKeys = functions.map { it.second },
                functionRows = functions.map { it.first },
                digits = halfKeyboard.take(digitRowCount).flatten().map { it.label },
                placeRows =
                    rows.map { row ->
                        row.mapNotNull { piece ->
                            when (piece) {
                                is Piece.Character -> LayoutPlace.Key(nextKey++)
                                is Piece.Function -> LayoutPlace.Function(nextFunction++)
                                is Piece.InputType, Piece.Globe -> null
                            }
                        }
                    },
                keyPopups = characterKeys.associate { halfWidthLabel(it.label) to popupEntries(it) },
            )
        }

        /** [row] as places; consecutive TRANSLATE copies make one. */
        fun pieces(row: List<KeyData>): List<Piece> {
            val pieces = mutableListOf<Piece>()
            for (key in row) {
                val functionKey = FunctionKey.ofCode(key.code)
                val previous = pieces.lastOrNull()
                when {
                    functionKey == FunctionKey.TRANSLATE && previous is Piece.Function && previous.functionKey == FunctionKey.TRANSLATE ->
                        pieces[pieces.lastIndex] = previous.copy(keys = previous.keys + key)
                    functionKey != null -> pieces += Piece.Function(functionKey, listOf(key))
                    key.code == KeyCode.LANGUAGE_SWITCH -> pieces += Piece.Globe
                    key.variation == KeyVariation.EMAIL_ADDRESS || key.variation == KeyVariation.URI -> pieces += Piece.InputType(key)
                    else -> pieces += Piece.Character(key)
                }
            }
            return pieces
        }

        fun KeyData.deepCopy(): KeyData = copy(popup = popup.mapTo(mutableListOf()) { it.deepCopy() })

        /** The text a character key types: its label, else its code's character; null for a function key. */
        fun entry(key: KeyData): String? {
            if (key.type != KeyType.CHARACTER || key.code < 0) return null
            return key.label.ifEmpty { if (key.code > 0) String(Character.toChars(key.code)) else "" }.ifEmpty { null }
        }

        fun popupEntries(key: KeyData): List<String> = key.popup.mapNotNull(::entry)

        /** [chars] in NFC, each once (TL and POJ spell some marks in different forms). */
        fun composedUnique(chars: List<String>): List<String> = chars.map { Normalizer.normalize(it, Normalizer.Form.NFC) }.distinct()

        /**
         * [chars] with CJK punctuation and full-width forms (U+2E80 and up, as the full-width punctuation
         * mode types) moved after the rest, order otherwise kept.
         */
        fun halfWidthFirst(chars: List<String>): List<String> = chars.partition { !isFullWidthSymbol(it) }.let { it.first + it.second }
    }
}

/** Whether [text] is CJK punctuation or a full-width form (U+2E80 and up). */
fun isFullWidthSymbol(text: String): Boolean = text.codePoints().anyMatch { it >= 0x2E80 }
