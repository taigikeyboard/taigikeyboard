package com.siansiansu.taigikeyboard.util

import android.content.Context
import android.graphics.Typeface
import androidx.annotation.FontRes
import androidx.core.content.res.ResourcesCompat
import com.siansiansu.taigikeyboard.R

// Loads custom typefaces (OpenHuninn, Iansui) with safe fallback to system default
object FontUtils {
    enum class FontType(
        val value: String,
    ) {
        SYSTEM("system"),
        OPEN_HUNINN("openHuninn"),
        IANSUI("iansui"),
    }

    private fun loadTypeface(
        context: Context,
        @FontRes fontResId: Int,
    ): Typeface =
        try {
            ResourcesCompat.getFont(context, fontResId) ?: Typeface.DEFAULT
        } catch (e: Exception) {
            Typeface.DEFAULT
        }

    private fun getOpenHuninnTypeface(context: Context): Typeface = loadTypeface(context, R.font.jf_openhuninn)

    private fun getIansuiTypeface(context: Context): Typeface = loadTypeface(context, R.font.iansui_regular)

    fun getTypefaceByType(
        fontType: String,
        context: Context,
    ): Typeface =
        when (fontType) {
            FontType.SYSTEM.value -> Typeface.DEFAULT

            FontType.OPEN_HUNINN.value -> getOpenHuninnTypeface(context)

            FontType.IANSUI.value -> getIansuiTypeface(context)

            // Default to OpenHuninn for unrecognized values from preferences
            else -> getOpenHuninnTypeface(context)
        }
}
