package com.siansiansu.taigikeyboard.ime.theme

import android.content.Context
import android.util.TypedValue

// Resolves theme attribute colors
fun getColorFromAttr(
    context: Context,
    attrColor: Int,
): Int {
    val typedValue = TypedValue()
    context.theme.resolveAttribute(attrColor, typedValue, true)
    return typedValue.data
}
