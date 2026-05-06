package com.siansiansu.taigikeyboard.ime.theme

import android.content.Context
import android.content.res.ColorStateList
import android.util.TypedValue
import android.view.View

// Resolves theme attribute colors and applies background tints to Views
fun getColorFromAttr(
    context: Context,
    attrColor: Int,
): Int {
    val typedValue = TypedValue()
    context.theme.resolveAttribute(attrColor, typedValue, true)
    return typedValue.data
}

fun setBackgroundTintColor(
    view: View,
    colorAttr: Int,
) {
    view.backgroundTintList =
        ColorStateList.valueOf(
            getColorFromAttr(view.context, colorAttr),
        )
}
