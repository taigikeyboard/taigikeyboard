package com.siansiansu.taigikeyboard.ime.text.layout

import com.siansiansu.taigikeyboard.ime.text.key.KeyCode
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File

/**
 * Pins the globe key's slot in the characters mod layouts: bottom-row index 1, right after
 * the symbols key, as on iOS (`TaigiLayouts.withGlobeKey`). The TPS mod carries none;
 * `LayoutManager` injects it at the same slot when TL / POJ use the TPS layout.
 */
class ModLayoutGlobeKeyTest {
    // Gradle runs unit tests with the module directory as the working directory
    private val modDir = File("src/main/assets/ime/text/characters/mod")
    private val modNames = modDir.listFiles { file -> file.extension == "json" }!!.map { it.nameWithoutExtension }

    private fun bottomRowCodes(name: String): List<Int> {
        val arrangement = JSONObject(File(modDir, "$name.json").readText()).getJSONArray("arrangement")
        val bottomRow = arrangement.getJSONArray(arrangement.length() - 1)
        return (0 until bottomRow.length()).map { bottomRow.getJSONObject(it).getInt("code") }
    }

    @Test
    fun `every non-TPS mod puts one globe right after the symbols key`() {
        assertTrue("mod assets not found", modNames.size > 1)
        for (name in modNames - "tps_halfwidth") {
            val codes = bottomRowCodes(name)
            assertEquals(name, KeyCode.VIEW_SYMBOLS, codes[0])
            assertEquals(name, KeyCode.LANGUAGE_SWITCH, codes[1])
            assertEquals(name, 1, codes.count { it == KeyCode.LANGUAGE_SWITCH })
        }
    }

    @Test
    fun `tps mod carries no globe`() {
        assertTrue(KeyCode.LANGUAGE_SWITCH !in bottomRowCodes("tps_halfwidth"))
    }
}
