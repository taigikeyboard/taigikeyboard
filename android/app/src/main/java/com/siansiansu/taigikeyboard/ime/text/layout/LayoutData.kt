// 中文: 從 layout JSON 反序列化得到的純資料 — type / name / direction / arrangement(每列每鍵)。
// 中文: getComputedLayoutDataArrangement() 把不可變列表轉成可變列表給 LayoutManager 後續注 popup。

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
