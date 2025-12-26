package com.siansiansu.taigikeyboard.localization

enum class DisplayLanguage {
    HANJI,
    POJ,
    TL
}

enum class InputMode(val value: String) {
    POJ("poj"),
    TL("tl");

    companion object {
        fun fromValue(value: String): InputMode {
            return entries.find { it.value == value } ?: POJ
        }
    }
}
