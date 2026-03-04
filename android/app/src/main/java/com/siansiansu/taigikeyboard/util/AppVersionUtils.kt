
package com.siansiansu.taigikeyboard.util

import android.content.Context
import com.siansiansu.taigikeyboard.ime.core.PrefHelper

object AppVersionUtils {
    const val DEFAULT_VERSION_RAW: String = "0.0.0"

    fun getRawVersionName(context: Context): String {
        return try {
            context.packageManager.getPackageInfo(context.packageName, 0).versionName ?: "undefined"
        } catch (e: Exception) {
            "undefined"
        }
    }

    fun updateVersionOnInstallAndLastUse(context: Context, prefs: PrefHelper) {
        if (prefs.versionOnInstall == DEFAULT_VERSION_RAW) {
            prefs.versionOnInstall = getRawVersionName(context)
        }
        prefs.versionLastUse = getRawVersionName(context)
    }
}
