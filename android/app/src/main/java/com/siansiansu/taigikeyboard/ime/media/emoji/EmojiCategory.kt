
package com.siansiansu.taigikeyboard.ime.media.emoji

import android.annotation.SuppressLint
import androidx.compose.material.icons.Icons
import androidx.compose.ui.graphics.vector.ImageVector
import com.siansiansu.taigikeyboard.ui.components.EmojiEmotions
import com.siansiansu.taigikeyboard.ui.components.EmojiEvents
import com.siansiansu.taigikeyboard.ui.components.EmojiFlags
import com.siansiansu.taigikeyboard.ui.components.EmojiFoodBeverage
import com.siansiansu.taigikeyboard.ui.components.EmojiNature
import com.siansiansu.taigikeyboard.ui.components.EmojiObjects
import com.siansiansu.taigikeyboard.ui.components.EmojiPeople
import com.siansiansu.taigikeyboard.ui.components.EmojiSymbols
import com.siansiansu.taigikeyboard.ui.components.EmojiTransportation

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

    /**
     * 取得對應的 Material Icon
     */
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
