
package com.siansiansu.taigikeyboard.ime.popup

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Canvas
import android.graphics.drawable.Drawable
import androidx.core.content.ContextCompat.getDrawable
import androidx.core.graphics.BlendModeColorFilterCompat
import androidx.core.graphics.BlendModeCompat
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.util.*

@SuppressLint("ViewConstructor")
class KeyPopupExtendedSingleView(
    context: Context, var isActive: Boolean = false
) : androidx.appcompat.widget.AppCompatTextView(
    context, null, 0
) {

    var iconDrawable: Drawable? = null

    init {
        background = getDrawable(context, R.drawable.shape_rect_rounded)
    }

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)

        setBackgroundTintColor(this, when {
            isActive -> R.attr.key_popup_extended_bgColorActive
            else -> R.attr.key_popup_extended_bgColor
        })

        val drawable = iconDrawable
        val drawablePadding = (0.2f * measuredHeight).toInt()
        if (drawable != null) {
            var marginV = 0
            var marginH = 0
            if (measuredWidth > measuredHeight) {
                marginH = (measuredWidth - measuredHeight) / 2
            } else {
                marginV = (measuredHeight - measuredWidth) / 2
            }
            drawable.setBounds(
                marginH + drawablePadding,
                marginV + drawablePadding,
                measuredWidth - marginH - drawablePadding,
                measuredHeight - marginV - drawablePadding)
            drawable.colorFilter = BlendModeColorFilterCompat.createBlendModeColorFilterCompat(
                getColorFromAttr(context, R.attr.key_popup_fgColor),
                BlendModeCompat.SRC_ATOP
            )
            drawable.draw(canvas)
        }
    }
}
