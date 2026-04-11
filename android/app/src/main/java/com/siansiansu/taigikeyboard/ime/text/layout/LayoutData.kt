
package com.siansiansu.taigikeyboard.ime.text.layout

import com.siansiansu.taigikeyboard.ime.text.key.KeyData
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardMode
typealias LayoutDataArrangement = List<List<KeyData>>

data class LayoutData(
    val type: LayoutType,
    val name: String,
    val direction: String,
    val arrangement: LayoutDataArrangement = listOf(),
) {
    private fun getComputedLayoutDataArrangement(): ComputedLayoutDataArrangement {
        val ret = mutableListOf<MutableList<KeyData>>()
        for (row in arrangement) {
            val retRow = mutableListOf<KeyData>()
            for (keyData in row) {
                retRow.add(keyData)
            }
            ret.add(retRow)
        }
        return ret
    }

    fun toComputedLayoutData(keyboardMode: KeyboardMode): ComputedLayoutData =
        ComputedLayoutData(
            keyboardMode,
            name,
            direction,
            getComputedLayoutDataArrangement(),
        )
}

typealias ComputedLayoutDataArrangement = MutableList<MutableList<KeyData>>

data class ComputedLayoutData(
    val mode: KeyboardMode,
    val name: String,
    val direction: String,
    val arrangement: ComputedLayoutDataArrangement = mutableListOf(),
)
