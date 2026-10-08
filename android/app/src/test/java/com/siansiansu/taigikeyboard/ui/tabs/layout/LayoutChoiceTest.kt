// Unit tests for the Layout tab's shelf grouping and card selection. Mirrors iOS LayoutChoiceTests.

package com.siansiansu.taigikeyboard.ui.tabs.layout

import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.i18n.generated.StringKey
import com.siansiansu.taigikeyboard.ime.text.layout.KeyboardLayoutOptions
import org.junit.Assert.assertEquals
import org.junit.Test

class LayoutChoiceTest {
    private fun choice(
        key: String,
        script: LayoutScript?,
    ): LayoutChoice =
        LayoutChoice(
            (KeyboardLayoutOptions.romanization + KeyboardLayoutOptions.phonetic).single { it.key == key },
            script,
        )

    @Test
    fun layoutShelves_splitLayoutsBySharedKeyTable() {
        assertEquals(
            listOf(
                StringKey.LAYOUT_COMMON_LAYOUTS_SECTION,
                StringKey.SETTINGS_TL_MODE,
                StringKey.SETTINGS_POJ_MODE,
                StringKey.SETTINGS_TPS_MODE,
            ),
            layoutShelves.map { it.titleKey },
        )
        assertEquals(
            listOf(
                listOf("phahTaigi/-"),
                listOf("qwerty/TL", "moe1/TL", "moe2/TL"),
                listOf("qwerty/POJ", "moe1/POJ", "moe2/POJ"),
                listOf("tps/-"),
            ),
            layoutShelves.map { shelf -> shelf.choices.map { "${it.option.key}/${it.script ?: "-"}" } },
        )
    }

    @Test
    fun previewRes_pojCardShowsPojTable() {
        assertEquals(R.drawable.layout_moe2_poj_preview, choice("moe2", LayoutScript.POJ).previewRes)
        assertEquals(R.drawable.layout_moe2_preview, choice("moe2", LayoutScript.TL).previewRes)
        assertEquals(R.drawable.layout_phahtaigi_preview, choice("phahTaigi", null).previewRes)
    }

    @Test
    fun isSelected_scriptCardsNeedTheirOwnMode() {
        val cases =
            listOf(
                Triple(choice("qwerty", LayoutScript.TL), "qwerty" to "tl", true),
                Triple(choice("qwerty", LayoutScript.TL), "qwerty" to "poj", false),
                Triple(choice("qwerty", LayoutScript.TL), "qwerty" to "english", false),
                Triple(choice("qwerty", LayoutScript.POJ), "qwerty" to "poj", true),
                Triple(choice("qwerty", LayoutScript.POJ), "moe1" to "poj", false),
                Triple(choice("phahTaigi", null), "phahTaigi" to "poj", true),
                Triple(choice("phahTaigi", null), "phahTaigi" to "english", true),
                Triple(choice("tps", null), "tps" to "tps", true),
            )
        for ((layoutChoice, state, expected) in cases) {
            assertEquals(
                "$layoutChoice with layout=${state.first} mode=${state.second}",
                expected,
                layoutChoice.isSelected(state.first, state.second),
            )
        }
    }
}
