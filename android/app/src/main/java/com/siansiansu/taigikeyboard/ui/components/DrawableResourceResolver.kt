package com.siansiansu.taigikeyboard.ui.components

import android.annotation.SuppressLint
import android.content.Context
import androidx.annotation.DrawableRes

// Resolves a drawable resource ID by name, returning fallback if not found.
@DrawableRes
@SuppressLint("DiscouragedApi")
fun resolveDrawableResId(
    context: Context,
    name: String,
    @DrawableRes fallback: Int = 0,
): Int {
    val resId = context.resources.getIdentifier(name, "drawable", context.packageName)
    return if (resId != 0) resId else fallback
}
