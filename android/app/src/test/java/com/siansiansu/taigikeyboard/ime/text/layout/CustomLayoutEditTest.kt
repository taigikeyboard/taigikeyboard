// Unit tests for the custom-layout editor model — the editing operations on CustomLayout and the
// tables' fresh layouts, spare-tray pools and drawn board rows — over the real layout assets.
// Mirrors iOS CustomLayoutTests (editing part).

package com.siansiansu.taigikeyboard.ime.text.layout

import com.siansiansu.taigikeyboard.ime.text.key.KeyCode
import com.siansiansu.taigikeyboard.ime.text.key.KeyData
import com.siansiansu.taigikeyboard.ime.text.key.KeyVariation
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutTestAssets.substituted
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutTestAssets.tables
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutTestAssets.unchanged
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class CustomLayoutEditTest {
    private val bases = KeyboardLayoutOptions.romanization.map { it.key }

    // trace (qwerty_tl + mod/default_halfwidth): rows below the digit row are
    //   q…p = Key 0–9 · a…l = Key 10–18 · shift(F0) z…m(Key 19–25) delete(F1) ·
    //   123(F2) [globe] [@] ,(Key 26) [/] emoji(F3) space(F4) -(Key 27) 文/A×3(F5) enter(F6)
    private val lohankha = unchanged("qwerty", LayoutScript.TL)
    private val lohankhaTable = tables.baseTable("qwerty", LayoutScript.TL)!!

    private fun key(index: Int) = LayoutPlace.Key(index)

    private fun function(index: Int) = LayoutPlace.Function(index)

    @Test
    fun `editor rows number every place in reading order`() {
        val rows = tables.editorRows(lohankha)
        assertEquals(5, rows.size)
        assertTrue(rows[0].all { it.place == null })
        assertEquals((0..9).map(::key), rows[1].map { it.place })
        assertEquals((10..18).map(::key), rows[2].map { it.place })
        assertEquals(listOf(function(0)) + (19..25).map(::key) + function(1), rows[3].map { it.place })
        assertEquals(listOf(function(2), key(26), function(3), function(4), key(27), function(5), function(6)), rows[4].map { it.place })
        assertEquals(
            listOf(
                GridKey.Face.Function(FunctionKey.NUMERIC),
                GridKey.Face.Character(","),
                GridKey.Face.Function(FunctionKey.EMOJI),
                GridKey.Face.Function(FunctionKey.SPACE),
                GridKey.Face.Character("-"),
                GridKey.Face.Function(FunctionKey.TRANSLATE),
                GridKey.Face.Function(FunctionKey.RETURN),
            ),
            rows[4].map { it.face },
        )
    }

    @Test
    fun `editor rows match the keyboard substitution on every base and script`() {
        for (base in bases) {
            for (script in LayoutScript.entries) {
                val fresh = unchanged(base, script)
                val table = tables.baseTable(base, script)!!
                // Every character key moved (reversed), shift ↔ delete swapped.
                val edited = fresh.copy(keys = fresh.keys.reversed()).swapped(function(0), function(1), table)
                assertEquals("$base $script", keyboardFaces(substituted(edited)), boardFaces(tables.editorRows(edited)))
            }
        }
    }

    @Test
    fun `preview rows match the keyboard with removed keys and no emoji key`() {
        // trace: Key 10–18 is the whole `a…l` row, so it leaves the keyboard.
        val edited = (10..18).fold(lohankha.removed(function(3))) { layout, index -> layout.removed(key(index)) }
        assertEquals(keyboardFaces(substituted(edited)), boardFaces(tables.previewRows(edited)))
        assertEquals(4, tables.previewRows(edited).size)
        assertEquals(5, tables.editorRows(edited).size)
        assertTrue(tables.editorRows(edited)[2].all { it.isRemoved })
    }

    @Test
    fun `character keys swap anywhere, function keys only within their row`() {
        assertTrue(lohankha.canSwap(key(0), key(27), lohankhaTable))
        assertFalse(lohankha.canSwap(key(0), key(0), lohankhaTable))
        // shift ↔ delete (row 3), 123 ↔ enter (row 4)
        assertTrue(lohankha.canSwap(function(0), function(1), lohankhaTable))
        assertTrue(lohankha.canSwap(function(2), function(6), lohankhaTable))
        // shift ↔ enter crosses rows; a function key never trades with a character key.
        assertFalse(lohankha.canSwap(function(0), function(6), lohankhaTable))
        assertFalse(lohankha.canSwap(key(19), function(0), lohankhaTable))

        val swapped = lohankha.swapped(key(0), key(10), lohankhaTable)
        assertEquals("a", swapped.keys[0])
        assertEquals("q", swapped.keys[10])
        assertEquals(lohankha, lohankha.swapped(function(0), function(6), lohankhaTable))
        assertEquals(
            listOf(FunctionKey.BACKSPACE, FunctionKey.SHIFT),
            lohankha.swapped(function(0), function(1), lohankhaTable).functionKeys.take(2),
        )
        assertTrue(tables.isValid(lohankha.swapped(function(2), function(6), lohankhaTable)))
    }

    @Test
    fun `a spare replaces a key, which returns to the tray`() {
        val replaced = lohankha.replaced(key(0), "ph")
        assertEquals("ph", replaced.keys[0])
        assertTrue("q" in replaced.spares(tables.trayLetters))
        assertFalse("ph" in replaced.spares(tables.trayLetters))
        assertTrue(tables.isValid(replaced))
        // A key on the board is never placed twice; function places take no spare.
        assertEquals(lohankha, lohankha.replaced(key(0), "a"))
        assertEquals(lohankha, lohankha.replaced(function(0), "ph"))
    }

    @Test
    fun `a removed key leaves a place the editor can fill`() {
        val removed = lohankha.removed(key(0))
        assertNull(removed.keys[0])
        assertEquals(setOf<LayoutPlace>(key(0)), removed.removedPlaces)
        assertFalse(removed.canRemove(key(0)))
        assertTrue("q" in removed.spares(tables.trayLetters))
        assertTrue(tables.editorRows(removed)[1][0].isRemoved)
        assertEquals(9, tables.previewRows(removed)[1].size)
        assertEquals("q", removed.replaced(key(0), "q").keys[0])
        // Only the emoji key leaves among the function keys.
        assertFalse(lohankha.canRemove(function(2)))
        assertEquals(lohankha, lohankha.removed(function(2)))
    }

    @Test
    fun `the hidden emoji key keeps its face, moves within its row and comes back where it is`() {
        assertEquals(function(3), lohankha.emojiPlace)
        val hidden = lohankha.removed(function(3))
        assertFalse(hidden.hasEmojiKey)
        assertEquals(setOf<LayoutPlace>(function(3)), hidden.removedPlaces)

        val moved = hidden.swapped(function(3), function(6), lohankhaTable)
        assertEquals(function(6), moved.emojiPlace)
        val last = tables.editorRows(moved)[4].last()
        assertEquals(GridKey(GridKey.Face.Function(FunctionKey.EMOJI), function(6), isRemoved = true), last)
        // trace: enter now holds F3 (after emoji's old neighbour `,`); the hidden emoji at F6 leaves the row.
        assertEquals(
            listOf(FunctionKey.NUMERIC, FunctionKey.RETURN, FunctionKey.SPACE, FunctionKey.TRANSLATE),
            tables.previewRows(moved)[4].mapNotNull { (it.face as? GridKey.Face.Function)?.key },
        )

        val restored = moved.restoreEmojiKey()
        assertTrue(restored.removedPlaces.isEmpty())
        assertEquals(GridKey(GridKey.Face.Function(FunctionKey.EMOJI), function(6)), tables.editorRows(restored)[4].last())
    }

    @Test
    fun `fresh layouts are valid and unchanged on every base and script`() {
        for (base in bases) {
            for (script in LayoutScript.entries) {
                val fresh = tables.fresh("id", "Name", base, script)!!
                assertTrue("$base $script", tables.isValid(fresh))
                assertTrue("$base $script", tables.isUnchanged(fresh))
                assertFalse("$base $script", tables.isUnchanged(fresh.removed(key(0))))
            }
        }
        assertNull(tables.fresh("id", "Name", "tps", LayoutScript.TL))
    }

    @Test
    fun `tray letters then symbols, each key once`() {
        val letters = tables.trayLetters
        val symbols = tables.traySymbols
        // a–z, o͘, `-` lead; every multi-letter romanization key follows, whatever the base.
        assertEquals(('a'..'z').map(Char::toString) + listOf("o͘", "-"), letters.take(28))
        assertTrue(listOf("ph", "ts", "tsh", "oo", "th", "kh", "ng", "nn", "ch", "chh").all { it in letters })
        // The finals close the letters, most frequent first.
        assertEquals(
            listOf("an", "ia", "in", "au", "ong", "ian", "un", "ai", "iu", "ann", "ang", "ui", "inn", "iau", "iong", "er", "ir"),
            letters.takeLast(17),
        )
        assertEquals(letters.size, letters.toSet().size)
        assertEquals(symbols.size, symbols.toSet().size)
        assertTrue(letters.none { it in symbols })
        // trace: western_default has 10 + 10 + 10 + 7 = 37 keys; its `-` goes with the letters.
        assertEquals(36, symbols.size)
        assertTrue(listOf(",", ".", "?", "!", "(", ")", "@", "~").all { it in symbols })
    }

    /** Each row below the digit row as the editor shows it: character labels and function keys. */
    private fun boardFaces(rows: List<List<GridKey>>): List<List<Any>> =
        rows.drop(1).map { row ->
            row.map { key ->
                when (val face = key.face) {
                    is GridKey.Face.Character -> face.label
                    is GridKey.Face.Function -> face.key
                }
            }
        }

    /**
     * The substituted keyboard below the digit row in the same terms: globe, `@` and `/` dropped (not
     * places), the three 文/A copies as one key.
     */
    private fun keyboardFaces(rows: List<List<KeyData>>): List<List<Any>> =
        rows.drop(1).map { row ->
            row
                .filterNot { it.code == KeyCode.LANGUAGE_SWITCH || it.variation == KeyVariation.EMAIL_ADDRESS || it.variation == KeyVariation.URI }
                .map { key -> FunctionKey.ofCode(key.code) ?: key.label }
                .fold(mutableListOf<Any>()) { faces, face ->
                    if (!(face == FunctionKey.TRANSLATE && faces.lastOrNull() == FunctionKey.TRANSLATE)) faces += face
                    faces
                }
        }
}
