// Unit tests for the pure CustomLayoutSelection write plans. Mirrors iOS SharedSettings custom-layout
// selection tests (built-in clears, TPS input mode suspends, delete keeps the base).

package com.siansiansu.taigikeyboard.ime.settings

import com.siansiansu.taigikeyboard.ime.text.layout.CustomLayout
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutScript
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutTestAssets
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Test

class CustomLayoutSelectionTest {
    private val layoutA = LayoutTestAssets.unchanged("moe1", LayoutScript.TL, id = "a")
    private val layoutB = LayoutTestAssets.unchanged("moe1", LayoutScript.POJ, id = "b")

    @Test
    fun `a built-in layout clears the selection, even on the same base`() {
        val writes = CustomLayoutSelection.forBuiltIn(TpsCascade.forKeyboardLayoutType("moe1", "moe1", "tl", "tl"))
        assertEquals("", writes[PreferenceKeys.SELECTED_CUSTOM_LAYOUT_ID])
        assertEquals("moe1", writes[PreferenceKeys.KEYBOARD_LAYOUT_TYPE])
    }

    @Test
    fun `applying selects the base through the TPS cascade, then the id`() {
        val writes = CustomLayoutSelection.forApply(layoutA, emptyMap(), currentLayout = "tps", currentInputMode = "tps", inputModeBeforeTps = "poj")
        assertEquals("moe1", writes[PreferenceKeys.KEYBOARD_LAYOUT_TYPE])
        assertEquals("poj", writes[PreferenceKeys.INPUT_MODE])
        assertEquals("a", writes[PreferenceKeys.SELECTED_CUSTOM_LAYOUT_ID])
    }

    @Test
    fun `switching between custom layouts on one base changes only the id`() {
        val writes = CustomLayoutSelection.forApply(layoutB, emptyMap(), currentLayout = "moe1", currentInputMode = "tl", inputModeBeforeTps = "tl")
        assertEquals("b", writes[PreferenceKeys.SELECTED_CUSTOM_LAYOUT_ID])
        assertFalse(writes.containsKey(PreferenceKeys.INPUT_MODE))
    }

    @Test
    fun `the TPS input mode leaves the selection to resume`() {
        assertFalse(TpsCascade.forInputMode("tps", "tl", "moe1", "moe1").containsKey(PreferenceKeys.SELECTED_CUSTOM_LAYOUT_ID))
        assertFalse(TpsCascade.forInputMode("tl", "tps", "tps", "moe1").containsKey(PreferenceKeys.SELECTED_CUSTOM_LAYOUT_ID))
    }

    @Test
    fun `saving writes the list and applies the layout, refusing past the cap`() {
        val edited = layoutA.copy(name = "Edited")
        val writes = CustomLayoutSelection.forSave(listOf(layoutA, layoutB), edited, emptyMap(), "moe1", "tl", "tl")!!
        assertEquals(CustomLayout.encodeList(listOf(edited, layoutB)), writes[PreferenceKeys.USER_LAYOUTS])
        assertEquals("a", writes[PreferenceKeys.SELECTED_CUSTOM_LAYOUT_ID])

        val full = (1..CustomLayout.MAX_CUSTOM_LAYOUTS).map { layoutA.copy(id = "$it") }
        assertNull(CustomLayoutSelection.forSave(full, layoutB, emptyMap(), "moe1", "tl", "tl"))
    }

    @Test
    fun `deleting the selected layout clears the selection and keeps the base`() {
        val writes = CustomLayoutSelection.forDelete(listOf(layoutA, layoutB), id = "a", selectedId = "a")
        assertEquals(CustomLayout.encodeList(listOf(layoutB)), writes[PreferenceKeys.USER_LAYOUTS])
        assertEquals("", writes[PreferenceKeys.SELECTED_CUSTOM_LAYOUT_ID])
        assertFalse(writes.containsKey(PreferenceKeys.KEYBOARD_LAYOUT_TYPE))

        assertFalse(CustomLayoutSelection.forDelete(listOf(layoutA, layoutB), id = "b", selectedId = "a").containsKey(PreferenceKeys.SELECTED_CUSTOM_LAYOUT_ID))
    }

    @Test
    fun `duplicating adds a renamed copy right after the original and leaves the selection alone`() {
        val writes = CustomLayoutSelection.forDuplicate(listOf(layoutA, layoutB), layoutA, name = "Copy")!!
        val saved = CustomLayout.decodeList(writes[PreferenceKeys.USER_LAYOUTS] as String, LayoutTestAssets.tables)
        assertEquals(listOf("a", saved[1].id, "b"), saved.map { it.id })
        assertNotEquals("a", saved[1].id)
        assertEquals(layoutA.copy(id = saved[1].id, name = "Copy"), saved[1])
        assertEquals(setOf(PreferenceKeys.USER_LAYOUTS), writes.keys)

        val full = (1..CustomLayout.MAX_CUSTOM_LAYOUTS).map { layoutA.copy(id = "$it") }
        assertNull(CustomLayoutSelection.forDuplicate(full, full.first(), name = "Copy"))
    }

    // forLayoutAfterMode — Layout-tab script card, mode first, one transaction

    @Test
    fun `a POJ card leaving TPS restores the pre-TPS layout, then the card's wins`() {
        // trace: forInputMode(poj, old=tps, layout tps) → layout phahTaigi, mode poj; then qwerty.
        val modeWrites = TpsCascade.forInputMode("poj", "tps", "tps", "phahTaigi")
        val writes = CustomLayoutSelection.forLayoutAfterMode("qwerty", modeWrites, "tps", "tps", "tl")
        assertEquals("qwerty", writes[PreferenceKeys.KEYBOARD_LAYOUT_TYPE])
        assertEquals("poj", writes[PreferenceKeys.INPUT_MODE])
    }

    @Test
    fun `a TL card from English switches the mode without TPS backups`() {
        val modeWrites = TpsCascade.forInputMode("tl", "english", "phahTaigi", "phahTaigi")
        val writes = CustomLayoutSelection.forLayoutAfterMode("moe1", modeWrites, "phahTaigi", "english", "tl")
        assertEquals("moe1", writes[PreferenceKeys.KEYBOARD_LAYOUT_TYPE])
        assertEquals("tl", writes[PreferenceKeys.INPUT_MODE])
        assertNull(writes[PreferenceKeys.INPUT_MODE_BEFORE_TPS])
        assertNull(writes[PreferenceKeys.LAYOUT_BEFORE_TPS])
    }

    @Test
    fun `the TPS card with the TPS mode enters TPS`() {
        // The preview generator's TPS job. trace: forInputMode(tps, old=poj, layout qwerty) saves
        // LAYOUT_BEFORE_TPS=qwerty, layout tps; the layout write is then idempotent.
        val modeWrites = TpsCascade.forInputMode("tps", "poj", "qwerty", "phahTaigi")
        val writes = CustomLayoutSelection.forLayoutAfterMode("tps", modeWrites, "qwerty", "poj", "tl")
        assertEquals("tps", writes[PreferenceKeys.KEYBOARD_LAYOUT_TYPE])
        assertEquals("tps", writes[PreferenceKeys.INPUT_MODE])
        assertEquals("qwerty", writes[PreferenceKeys.LAYOUT_BEFORE_TPS])
    }

    @Test
    fun `applying after mode writes runs the base cascade against the entered state`() {
        // Entering POJ from TPS restores phahTaigi; the base cascade then sees a non-TPS layout.
        val modeWrites = TpsCascade.forInputMode("poj", "tps", "tps", "phahTaigi")
        val writes = CustomLayoutSelection.forApply(layoutB, modeWrites, "tps", "tps", "tl")
        assertEquals("moe1", writes[PreferenceKeys.KEYBOARD_LAYOUT_TYPE])
        assertEquals("poj", writes[PreferenceKeys.INPUT_MODE])
        assertEquals("b", writes[PreferenceKeys.SELECTED_CUSTOM_LAYOUT_ID])
    }
}
