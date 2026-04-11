package com.siansiansu.taigikeyboard.ime.text.key

import com.squareup.moshi.JsonClass

@JsonClass(generateAdapter = true)
data class KeyData(
    var code: Int,
    var label: String = "",
    var popup: MutableList<KeyData> = mutableListOf(),
    var type: KeyType = KeyType.CHARACTER,
    var variation: KeyVariation = KeyVariation.ALL
)
