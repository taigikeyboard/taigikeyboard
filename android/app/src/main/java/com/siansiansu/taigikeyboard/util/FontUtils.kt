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
        GEN_YO_MIN("genYoMin"),
        GEN_YO_GOTHIC("genYoGothic"),
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

    private fun getGenYoMinTypeface(context: Context): Typeface = loadTypeface(context, R.font.genyomin2tw_r)

    private fun getGenYoGothicTypeface(context: Context): Typeface = loadTypeface(context, R.font.genyogothic2tw_r)

    fun getTypefaceByType(
        fontType: String,
        context: Context,
    ): Typeface =
        when (fontType) {
            FontType.SYSTEM.value -> Typeface.DEFAULT

            FontType.OPEN_HUNINN.value -> getOpenHuninnTypeface(context)

            FontType.IANSUI.value -> getIansuiTypeface(context)

            FontType.GEN_YO_MIN.value -> getGenYoMinTypeface(context)

            FontType.GEN_YO_GOTHIC.value -> getGenYoGothicTypeface(context)

            // Default to OpenHuninn for unrecognized values from preferences
            else -> getOpenHuninnTypeface(context)
        }
}
