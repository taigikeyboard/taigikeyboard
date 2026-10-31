// The built-in tables a custom layout is made from: per base × script, the places of the half-width
// letter keyboard (as LayoutAssets merges it), and a catalog of every key a custom layout can place,
// in both punctuation widths, plus the built-in long-press lists and the characters a list can take.
// Puts an active custom layout's keys and long-press lists into the merged keyboard for LayoutManager.
// Android keeps its JSON tables (roadmap § Model); the iOS counterpart is the table half of
// Layout/CustomLayout.swift.

package com.siansiansu.taigikeyboard.ime.text.layout

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

class CustomLayoutTables(
    private val assets: LayoutAssets,
) {
    /** A base table's places below the digit row, in reading order. */
    class BaseTable(
        /** The half-width label of each character place. */
        val labels: List<String>,
        val functionKeys: List<FunctionKey>,
        /** The row of each function place. */
        val functionRows: List<Int>,
        /** The digit row's labels. */
        val digits: List<String>,
        /** The places of each row below the digit row (globe, `@` and `/` are not places). */
        val placeRows: List<List<LayoutPlace>>,
        /** The long-press entries of each half-width character key, by label (tone marks not included). */
        val keyPopups: Map<String, List<String>>,
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

    /**
     * Letters offered in the spare tray: a–z, o͘, ⁿ, `ee`, `-`, then every multi-letter key of the romanization
     * tables (MOE2 `ph`, `tsh`, POJ `ch` …), whatever the base, then [TRAY_FINALS]. iOS
     * `CustomLayout.trayLetters`.
     */
    val trayLetters: List<String>

    /** Punctuation and symbols offered in the spare tray: the common symbol page (the 123 key). */
    val traySymbols: List<String>

    /**
     * Tone-marked letters for long-press lists, one row per lowercase base (Shift uppercases them on
     * the keyboard): a e i o oo o͘ u m n ng first, then any other base; each row TL's marks, then the
     * marks only POJ has. iOS `CustomLayout.toneRows`.
     */
    val toneRows: List<List<String>> by lazy {
        val toneBases = toneMarks.values
            .flatMap { it.keys }
            .filter { it == it.lowercase() }
            .toSet()
        val leadingBases = listOf("a", "e", "i", "o", "oo", "o\u0358", "u", "m", "n", "ng").filter { it in toneBases }
        (leadingBases + (toneBases - leadingBases.toSet()).sorted()).map { base ->
            // ⁿ rides the `n` list but is no tone mark: it sits with the letters.
            composedUnique(toneMarks.values.flatMap { it[base].orEmpty() }).filterNot { it == TRAY_NASAL_MARKER }
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

    /** Full-width key label → its half-width label, from [catalog]. */
    private val halfWidthLabels: Map<String, String>

    // The long-press tables above and below serve the editor only, so the keyboard never parses them.

    /** Tone-mark long-press entries by script, then by key label. */
    private val toneMarks: Map<LayoutScript, Map<String, List<String>>> by lazy {
        LayoutScript.entries.associateWith { script ->
            assets.extendedPopups(script.inputMode).mapValues { (_, keys) -> keys.mapNotNull(::entry) }
        }
    }

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

        val romanizationLabels = mutableListOf<String>()
        for (base in KeyboardLayoutOptions.romanization.map { it.key }) {
            for (script in LayoutScript.entries) {
                val halfKeyboard = assets.letterKeyboard(base, script.inputMode, isFullWidthPunctuation = false, load)
                // Below the digit row; row numbers count it.
                val halfRows = halfKeyboard.drop(1).map(::pieces)
                val fullRows = assets.letterKeyboard(base, script.inputMode, isFullWidthPunctuation = true, load).drop(1).map(::pieces)
                halfRows.flatten().zip(fullRows.flatten()).forEach { (h, f) ->
                    if (h is Piece.Character && f is Piece.Character) addToCatalog(h.key, f.key)
                }
                if (halfRows.isEmpty()) continue
                val characterKeys = halfRows.flatten().filterIsInstance<Piece.Character>().map { it.key }
                val labels = characterKeys.map { it.label }
                romanizationLabels += labels
                val functions = halfRows.withIndex().flatMap { (index, row) -> row.filterIsInstance<Piece.Function>().map { index + 1 to it.functionKey } }
                var nextKey = 0
                var nextFunction = 0
                tables[base to script] =
                    BaseTable(
                        labels = labels,
                        functionKeys = functions.map { it.second },
                        functionRows = functions.map { it.first },
                        digits = halfKeyboard.first().map { it.label },
                        placeRows =
                            halfRows.map { row ->
                                row.mapNotNull { piece ->
                                    when (piece) {
                                        is Piece.Character -> LayoutPlace.Key(nextKey++)
                                        is Piece.Function -> LayoutPlace.Function(nextFunction++)
                                        is Piece.InputType, Piece.Globe -> null
                                    }
                                }
                            },
                        keyPopups = characterKeys.associate { it.label to popupEntries(it) },
                    )
            }
        }
        this.tables = tables
        // No built-in table has \u207f, `ee` or the finals: each is a plain key, in both widths.
        (listOf(TRAY_NASAL_MARKER) + TRAY_VOWELS + TRAY_FINALS).forEach { addToCatalog(KeyData(0, it), KeyData(0, it)) }
        this.catalog = catalog
        halfWidthLabels = HashMap<String, String>().apply { catalog.forEach { (half, keys) -> putIfAbsent(keys.second.label, half) } }

        val symbolLabels = symbolPage.flatten().map { it.label }
        val single = ('a'..'z').map(Char::toString) + listOf("o\u0358", TRAY_NASAL_MARKER) + TRAY_VOWELS + listOf("-")
        trayLetters = (single + romanizationLabels.filterNot { it in symbolLabels } + TRAY_FINALS).distinct().filter { it in catalog }
        traySymbols = symbolLabels.filterNot { it in trayLetters }.distinct()
    }

    /** [entries] spelled as the tray's chips are (NFC), to tell which chips a list already holds. */
    fun trayForms(entries: List<String>): Set<String> = composedUnique(entries).toSet()

    /** Whether [label] can go on a board key: the long-press-only symbols cannot. */
    fun isPlaceable(label: String): Boolean = label in catalog

    /**
     * The built-in long-press list of the key labelled [label] on [layout]'s base and script: the base's
     * own key for that label (else the catalog's half-width key), then its tone marks.
     */
    fun defaultCallouts(
        layout: CustomLayout,
        label: String,
    ): List<String> {
        val ownPopups = baseTable(layout.base, layout.script)?.keyPopups?.get(label) ?: catalog[label]?.first?.let(::popupEntries).orEmpty()
        return ownPopups + toneMarks[layout.script]?.get(label).orEmpty()
    }

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
     * key without a long-press.
     */
    fun applyCallouts(
        arrangement: ComputedLayoutDataArrangement,
        layout: CustomLayout,
    ) {
        if (layout.callouts.isEmpty()) return
        val placed = layout.keys.filterNotNullTo(HashSet())
        for (piece in arrangement.drop(1).flatMap(::pieces)) {
            val key = (piece as? Piece.Character)?.key ?: continue
            // A full-width key goes by its half-width label, as the list is stored.
            val label = key.label.takeIf { it in placed } ?: halfWidthLabels[key.label]?.takeIf { it in placed } ?: continue
            layout.callouts[label]?.let { entries -> key.popup = entries.mapTo(mutableListOf(), ::calloutKey) }
        }
    }

    /** A fresh layout: the [base] × [script] table unchanged; null for TPS and unknown layouts. */
    fun fresh(
        id: String,
        name: String,
        base: String,
        script: LayoutScript,
    ): CustomLayout? =
        baseTable(base, script)?.let { table ->
            CustomLayout(id, name, base, script, table.labels, table.functionKeys, hasEmojiKey = true)
        }

    /** Whether [layout]'s board still matches its base table. */
    fun isUnchanged(layout: CustomLayout): Boolean = layout == fresh(layout.id, layout.name, layout.base, layout.script)

    /**
     * The board the editor draws: the digit row (no places), then every place of [layout], a removed
     * key and the hidden emoji key included ([CustomLayout.removedPlaces]).
     */
    fun editorRows(layout: CustomLayout): List<List<GridKey>> {
        val table = baseTable(layout.base, layout.script) ?: return emptyList()
        val removed = layout.removedPlaces
        val placeRows =
            table.placeRows.map { row ->
                row.map { place -> GridKey(layout.face(place), place, isRemoved = place in removed) }
            }
        return listOf(table.digits.map { GridKey(GridKey.Face.Character(it), place = null) }) + placeRows
    }

    /** The board as the keyboard shows it (card previews): removed keys leave their row, an empty row goes too. */
    fun previewRows(layout: CustomLayout): List<List<GridKey>> = editorRows(layout).map { row -> row.filterNot { it.isRemoved } }.filter { it.isNotEmpty() }

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
            listOf("an", "ia", "in", "au", "ong", "ian", "un", "ai", "iu", "ann", "ang", "ui", "inn", "iau", "er", "ir")

        /**
         * Built-in long-press symbols that draw as colour emoji in the tray; left out of it, as on iOS.
         * Listed, not detected: `Character.isEmoji` needs API 35.
         */
        val DRAWN_AS_EMOJI = setOf("▶", "◀")

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

        /** A long-press entry as a key: one code point types itself; longer text has code 0, as the tray's finals do. */
        fun calloutKey(entry: String): KeyData = KeyData(if (entry.codePointCount(0, entry.length) == 1) entry.codePointAt(0) else 0, entry)

        /** [chars] in NFC, each once (TL and POJ spell some marks in different forms). */
        fun composedUnique(chars: List<String>): List<String> = chars.map { Normalizer.normalize(it, Normalizer.Form.NFC) }.distinct()

        /**
         * [chars] with CJK punctuation and full-width forms (U+2E80 and up, as the full-width punctuation
         * mode types) moved after the rest, order otherwise kept.
         */
        fun halfWidthFirst(chars: List<String>): List<String> = chars.partition { text -> text.codePoints().noneMatch { it >= 0x2E80 } }.let { it.first + it.second }
    }
}
