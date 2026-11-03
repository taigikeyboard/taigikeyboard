// Unit tests for the per-script layout memory write plans. Mirrors iOS CustomLayoutTests § Per-script memory.

package com.siansiansu.taigikeyboard.ime.settings

import com.siansiansu.taigikeyboard.ime.text.layout.LayoutScript
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutTestAssets
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Test

class LayoutScriptMemoryTest {
    private val tlLayout = LayoutTestAssets.unchanged("moe1", LayoutScript.TL, id = "tl")
    private val pojLayout = LayoutTestAssets.unchanged("moe2", LayoutScript.POJ, id = "poj")
    private val sharedLayout = LayoutTestAssets.unchanged("phahTaigi", LayoutScript.TL, id = "shared")
    private val universalLayout = LayoutTestAssets.unchanged("moe1", LayoutScript.TL, id = "universal").copy(isUniversal = true)
    private val tlOnSharedBase = LayoutTestAssets.unchanged("phahTaigi", LayoutScript.TL, id = "tlShared").copy(isUniversal = false)
    private val layouts = listOf(tlLayout, pojLayout, sharedLayout, universalLayout, tlOnSharedBase)

    private fun enter(
        newMode: String,
        oldMode: String,
        owner: String = "",
        liveLayout: String,
        liveCustomId: String = "",
        tl: String = "",
        poj: String = "",
    ) = LayoutScriptMemory.forInputMode(
        newMode,
        oldMode,
        owner,
        liveLayout,
        liveCustomId,
        remembered = { if (it == LayoutScript.TL) tl else poj },
        layouts = { layouts },
    )

    @Test
    fun `a script without memory shows the built-in of the same base and remembers the live one`() {
        val writes = enter("poj", "tl", liveLayout = "moe1", liveCustomId = "tl")
        assertEquals("moe1", writes[PreferenceKeys.KEYBOARD_LAYOUT_TYPE])
        assertEquals("", writes[PreferenceKeys.SELECTED_CUSTOM_LAYOUT_ID])
        assertEquals("moe1|tl", writes[PreferenceKeys.TL_LAYOUT_SELECTION])
        assertEquals("poj", writes[PreferenceKeys.LAYOUT_SCRIPT_OWNER])
    }

    @Test
    fun `entering a script brings back its own layout`() {
        val writes = enter("tl", "poj", owner = "poj", liveLayout = "moe2", liveCustomId = "poj", tl = "moe1|tl")
        assertEquals("moe1", writes[PreferenceKeys.KEYBOARD_LAYOUT_TYPE])
        assertEquals("tl", writes[PreferenceKeys.SELECTED_CUSTOM_LAYOUT_ID])
        assertEquals("moe2|poj", writes[PreferenceKeys.POJ_LAYOUT_SELECTION])
        assertFalse(writes.containsKey(PreferenceKeys.TL_LAYOUT_SELECTION))
    }

    @Test
    fun `the same script, English and TPS leave the selection alone`() {
        assertEquals(emptyMap<Any, Any>(), enter("poj", "tps", owner = "poj", liveLayout = "moe2", liveCustomId = "poj", tl = "moe1|tl"))
        val unstored = enter("tl", "tl", liveLayout = "moe1")
        assertEquals(setOf(PreferenceKeys.LAYOUT_SCRIPT_OWNER), unstored.keys)
        assertEquals("tl", unstored[PreferenceKeys.LAYOUT_SCRIPT_OWNER])
        assertEquals(emptyMap<Any, Any>(), enter("english", "tl", owner = "tl", liveLayout = "moe1"))
        assertEquals(emptyMap<Any, Any>(), enter("tps", "poj", owner = "poj", liveLayout = "moe2"))
    }

    @Test
    fun `TL through English to POJ swaps`() {
        val writes = enter("poj", "english", owner = "tl", liveLayout = "moe1", liveCustomId = "tl", poj = "moe2|poj")
        assertEquals("moe2", writes[PreferenceKeys.KEYBOARD_LAYOUT_TYPE])
        assertEquals("poj", writes[PreferenceKeys.SELECTED_CUSTOM_LAYOUT_ID])
    }

    @Test
    fun `a deleted remembered layout leaves its base`() {
        val writes = enter("tl", "poj", owner = "poj", liveLayout = "moe2", tl = "moe1|gone")
        assertEquals("moe1", writes[PreferenceKeys.KEYBOARD_LAYOUT_TYPE])
        assertEquals("", writes[PreferenceKeys.SELECTED_CUSTOM_LAYOUT_ID])
    }

    @Test
    fun `a shared-table layout follows into a script without memory`() {
        val writes = enter("poj", "tl", owner = "tl", liveLayout = "phahTaigi", liveCustomId = "shared")
        assertEquals("phahTaigi", writes[PreferenceKeys.KEYBOARD_LAYOUT_TYPE])
        assertEquals("shared", writes[PreferenceKeys.SELECTED_CUSTOM_LAYOUT_ID])
    }

    @Test
    fun `a universal layout follows into a script without memory`() {
        val writes = enter("poj", "tl", owner = "tl", liveLayout = "moe1", liveCustomId = "universal")
        assertEquals("moe1", writes[PreferenceKeys.KEYBOARD_LAYOUT_TYPE])
        assertEquals("universal", writes[PreferenceKeys.SELECTED_CUSTOM_LAYOUT_ID])
    }

    @Test
    fun `a one-script layout on a shared base stays out of the other script`() {
        val writes = enter("poj", "tl", owner = "tl", liveLayout = "phahTaigi", liveCustomId = "tlShared")
        assertEquals("phahTaigi", writes[PreferenceKeys.KEYBOARD_LAYOUT_TYPE])
        assertEquals("", writes[PreferenceKeys.SELECTED_CUSTOM_LAYOUT_ID])
    }

    @Test
    fun `a remembered layout of the other script is dropped to its base`() {
        // Pre-memory state: a TL layout left selected in POJ ends up in the POJ slot.
        val writes = enter("poj", "tl", owner = "tl", liveLayout = "moe2", poj = "moe1|tl")
        assertEquals("moe1", writes[PreferenceKeys.KEYBOARD_LAYOUT_TYPE])
        assertEquals("", writes[PreferenceKeys.SELECTED_CUSTOM_LAYOUT_ID])
    }

    @Test
    fun `a missing owner is the old mode's script`() {
        val writes = enter("tl", "poj", liveLayout = "moe2", liveCustomId = "poj")
        assertEquals("moe2|poj", writes[PreferenceKeys.POJ_LAYOUT_SELECTION])
    }

    @Test
    fun `selection storage round-trips and rejects unknown bases`() {
        assertEquals(LayoutSelection("moe2", ""), LayoutSelection.fromStorage("moe2"))
        assertEquals(LayoutSelection("moe2", "x"), LayoutSelection.fromStorage("moe2|x"))
        assertEquals("qwerty|x", LayoutSelection("qwerty", "x").storage)
        assertNull(LayoutSelection.fromStorage(""))
        assertNull(LayoutSelection.fromStorage("tps"))
        assertNull(LayoutSelection.fromStorage("a|b|c"))
    }
}
