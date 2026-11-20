
package com.siansiansu.taigikeyboard.ime.media.emoji

import android.annotation.SuppressLint
import androidx.compose.material.icons.Icons
import androidx.compose.ui.graphics.vector.ImageVector

enum class EmojiCategory {
    SMILEYS_EMOTION,
    PEOPLE_BODY,
    ANIMALS_NATURE,
    FOOD_DRINK,
    TRAVEL_PLACES,
    ACTIVITIES,
    OBJECTS,
    SYMBOLS,
    FLAGS,
    ;

    override fun toString(): String = super.toString().replace("_", " & ")

    fun icon(): ImageVector =
        when (this) {
            SMILEYS_EMOTION -> Icons.Default.EmojiEmotions
            PEOPLE_BODY -> Icons.Default.EmojiPeople
            ANIMALS_NATURE -> Icons.Default.EmojiNature
            FOOD_DRINK -> Icons.Default.EmojiFoodBeverage
            TRAVEL_PLACES -> Icons.Default.EmojiTransportation
            ACTIVITIES -> Icons.Default.EmojiEvents
            OBJECTS -> Icons.Default.EmojiObjects
            SYMBOLS -> Icons.Default.EmojiSymbols
            FLAGS -> Icons.Default.EmojiFlags
        }

    companion object {
        @SuppressLint("DefaultLocale")
        fun fromString(string: String): EmojiCategory = valueOf(string.replace(" & ", "_").uppercase())
    }
}
