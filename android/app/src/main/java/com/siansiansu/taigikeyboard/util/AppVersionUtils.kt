package com.siansiansu.taigikeyboard.util

import android.content.Context
import com.siansiansu.taigikeyboard.ime.core.PrefHelper

// Reads and tracks app version info for install/last-use preferences
object AppVersionUtils {
    const val DEFAULT_VERSION_RAW: String = "0.0.0"

    @Suppress("DEPRECATION")
    fun getRawVersionName(context: Context): String =
        try {
            context.packageManager.getPackageInfo(context.packageName, 0).versionName ?: "undefined"
        } catch (e: Exception) {
            "undefined"
        }

    fun updateVersionOnInstallAndLastUse(
        context: Context,
        prefs: PrefHelper,
    ) {
        val currentVersion = getRawVersionName(context)
        if (prefs.versionOnInstall == DEFAULT_VERSION_RAW) {
            prefs.versionOnInstall = currentVersion
        }
        prefs.versionLastUse = currentVersion
    }
}
