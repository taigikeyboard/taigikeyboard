// Unit tests for KeyboardLayoutOption.previewRes(inputMode). Mirrors iOS LayoutChoiceTests.

package com.siansiansu.taigikeyboard.ime.text.layout

import com.siansiansu.taigikeyboard.R
import org.junit.Assert.assertEquals
import org.junit.Test

class KeyboardLayoutOptionTest {
    private fun option(key: String): KeyboardLayoutOption = (KeyboardLayoutOptions.romanization + KeyboardLayoutOptions.phonetic).single { it.key == key }

    @Test
    fun previewRes_pojOnlyWhereTableDiffers() {
        assertEquals(R.drawable.layout_standard_poj_preview, option("qwerty").previewRes("poj"))
        assertEquals(R.drawable.layout_moe2_poj_preview, option("moe2").previewRes("poj"))
        assertEquals(R.drawable.layout_moe2_preview, option("moe2").previewRes("tl"))
        assertEquals(R.drawable.layout_moe1_preview, option("moe1").previewRes("english"))
        assertEquals(R.drawable.layout_phahtaigi_preview, option("phahTaigi").previewRes("poj"))
        assertEquals(R.drawable.layout_tps_preview, option("tps").previewRes("poj"))
    }
}
