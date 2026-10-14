// The built-in tables a custom layout is made from: per base × script, the places of the half-width
// letter keyboard (as LayoutAssets merges it), and a catalog of every key a custom layout can place,
// in both punctuation widths. Puts an active custom layout's keys into the merged keyboard for
// LayoutManager. Android keeps its JSON tables (roadmap § Model); the iOS counterpart is the table
// half of Layout/CustomLayout.swift.

package com.siansiansu.taigikeyboard.ime.text.layout

import com.siansiansu.taigikeyboard.ime.text.key.KeyCode
import com.siansiansu.taigikeyboard.ime.text.key.KeyData
import com.siansiansu.taigikeyboard.ime.text.key.KeyVariation

class CustomLayoutTables(
    assets: LayoutAssets,
) {
    /** A base table's places below the digit row, in reading order. */
    class BaseTable(
        /** The half-width label of each character place. */
        val labels: List<String>,
        val functionKeys: List<FunctionKey>,
        /** The row of each function place. */
        val functionRows: List<Int>,
    )

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

    init {
        // Bases share files (PhahTaigi has one table for both scripts, modifiers ignore the script,
        // every base has the same digit row): parse each once. Tables never mutate a parsed key.
        val parsed = HashMap<Pair<LayoutType, String>, LayoutData?>()
        val load = { type: LayoutType, name: String -> parsed.getOrPut(type to name) { assets.load(type, name) } }

        val tables = mutableMapOf<Pair<String, LayoutScript>, BaseTable>()
        val catalog = mutableMapOf<String, Pair<KeyData, KeyData>>()

        fun addToCatalog(
            half: KeyData,
            full: KeyData,
        ) {
            catalog.putIfAbsent(half.label, half to full)
        }

        // The symbol page first: its `(` `)` carry full-width forms that MOE1's own keys leave out (a
        // base's own keys still win, see substitute).
        val symbolPage = load(LayoutType.SYMBOLS, "western_default")?.arrangement.orEmpty()
        val symbolPageFull = load(LayoutType.SYMBOLS, "western_fullwidth")?.arrangement.orEmpty()
        symbolPage.zip(symbolPageFull).forEach { (half, full) -> half.zip(full).forEach { (h, f) -> addToCatalog(h, f) } }

        for (base in KeyboardLayoutOptions.romanization.map { it.key }) {
            for (script in LayoutScript.entries) {
                // Below the digit row; row numbers count it.
                val halfRows = assets.letterKeyboard(base, script.inputMode, isFullWidthPunctuation = false, load).drop(1).map(::pieces)
                val fullRows = assets.letterKeyboard(base, script.inputMode, isFullWidthPunctuation = true, load).drop(1).map(::pieces)
                halfRows.flatten().zip(fullRows.flatten()).forEach { (h, f) ->
                    if (h is Piece.Character && f is Piece.Character) addToCatalog(h.key, f.key)
                }
                if (halfRows.isEmpty()) continue
                val functions = halfRows.withIndex().flatMap { (index, row) -> row.filterIsInstance<Piece.Function>().map { index + 1 to it.functionKey } }
                tables[base to script] =
                    BaseTable(
                        labels = halfRows.flatten().filterIsInstance<Piece.Character>().map { it.key.label },
                        functionKeys = functions.map { it.second },
                        functionRows = functions.map { it.first },
                    )
            }
        }
        this.tables = tables
        this.catalog = catalog
    }

    /** The table of [base] × [script]; null for TPS and unknown layouts. */
    fun baseTable(
        base: String,
        script: LayoutScript,
    ): BaseTable? = tables[base to script]

    /**
     * Whether [layout] can be shown: a romanization base, one entry per character place, every key
     * known, none twice, each row's function keys the base row's.
     */
    fun isValid(layout: CustomLayout): Boolean {
        val table = baseTable(layout.base, layout.script) ?: return false
        val placed = layout.keys.filterNotNull()
        return layout.keys.size == table.labels.size &&
            placed.all { it in catalog } &&
            placed.toSet().size == placed.size &&
            functionKeysByRow(table, layout.functionKeys) == functionKeysByRow(table, table.functionKeys)
    }

    /**
     * [arrangement] — the merged base table of [layout] in the current punctuation width, digit row
     * first — with [layout]'s keys in its places. A moved key is the whole [KeyData], so its inline
     * popup moves with it: the base's own key for that label first (MOE1 `(` stays half-width), else
     * the catalog's in the current width. Removed keys leave their row and a row left empty leaves
     * the keyboard. Globe, `@` and `/` are dropped; `@` / `/` come back on the bottom row (iOS
     * `CustomLayoutService.applyInputTypeKey`): right after a bottom-row `,`, which then hides in email
     * / URL fields, else right before space. The globe comes back in LayoutManager, after 123.
     */
    fun substitute(
        arrangement: ComputedLayoutDataArrangement,
        layout: CustomLayout,
        isFullWidthPunctuation: Boolean,
    ): ComputedLayoutDataArrangement {
        val table = baseTable(layout.base, layout.script) ?: return arrangement
        if (arrangement.size < 2) return arrangement
        val rows = arrangement.drop(1).map(::pieces)
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
        val result: ComputedLayoutDataArrangement = mutableListOf(arrangement.first())
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
    ): KeyData? = catalog[label]?.let { (half, full) -> (if (isFullWidthPunctuation) full else half).deepCopy() }

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
    }
}
