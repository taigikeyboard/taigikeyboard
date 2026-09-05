// 一份計算完成的鍵盤佈局快照,提供唯讀 rows: List<List<KeyData>>。
// 外層列拓撲不可變;個別 KeyData 仍可變(LayoutManager.mergeLayouts 在建構後填 popup)。

package com.siansiansu.taigikeyboard.ime.text.keyboard

import com.siansiansu.taigikeyboard.ime.text.key.KeyData
import com.siansiansu.taigikeyboard.ime.text.layout.ComputedLayoutData

/**
 * Immutable view of a fully-computed keyboard layout, exposed as read-only
 * `rows: List<List<KeyData>>`.
 *
 * Row topology (the outer + inner List nesting) is immutable; individual
 * [KeyData] elements remain mutable because `LayoutManager.mergeLayouts`
 * populates `KeyData.popup` after construction (TODO: tighten alongside a
 * `KeyData` immutability refactor).
 */
data class KeyboardLayoutData(
    val mode: KeyboardMode,
    val name: String,
    val direction: String,
    val rows: List<List<KeyData>>,
) {
    companion object {
        fun from(computed: ComputedLayoutData): KeyboardLayoutData =
            KeyboardLayoutData(
                mode = computed.mode,
                name = computed.name,
                direction = computed.direction,
                rows = computed.arrangement.map { row -> row.toList() },
            )
    }
}
