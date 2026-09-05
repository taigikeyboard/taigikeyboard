// 鍵盤模式列舉 — characters / symbols / numeric / phone / clipboard 等 8 種。
// 由 LayoutManager 驅動 mode 切換,各 mode 對應一份 layout JSON。

package com.siansiansu.taigikeyboard.ime.text.keyboard

enum class KeyboardMode {
    CHARACTERS,
    SYMBOLS,
    SYMBOLS2,
    NUMERIC,
    NUMERIC_ADVANCED,
    PHONE,
    PHONE2,
    CLIPBOARD,
}
