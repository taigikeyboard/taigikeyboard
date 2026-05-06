package com.siansiansu.taigikeyboard.ime.core

import android.content.Context

// Reads and tracks app version info for install/last-use preferences
object AppVersionTracker {
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
