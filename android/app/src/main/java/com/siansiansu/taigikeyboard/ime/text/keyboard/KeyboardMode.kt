// Keyboard mode enum — 8 modes (characters / symbols / numeric / phone / clipboard, etc.),
// switched by LayoutManager; each mode maps to one layout JSON.

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
