package com.siansiansu.taigikeyboard.ime.theme

import android.content.res.Configuration

// A user theme renders light whatever the system night mode: the IME (TaigiKeyboard.lightContext)
// resolves resources under this copy.
fun Configuration.withNightModeOff(): Configuration = withNightMode(isNightMode = false)

// A copy of this configuration pinned to night ([isNightMode]) or day, whatever the system says.
fun Configuration.withNightMode(isNightMode: Boolean): Configuration =
    Configuration(this).apply {
        val nightBits = if (isNightMode) Configuration.UI_MODE_NIGHT_YES else Configuration.UI_MODE_NIGHT_NO
        uiMode = (uiMode and Configuration.UI_MODE_NIGHT_MASK.inv()) or nightBits
    }
