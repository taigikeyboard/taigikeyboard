// 中文: 應用版本追蹤器 — 紀錄 install 當下的版本與最後使用版本到 Pref。
// 中文: 由 Settings Activity 啟動時呼叫,持久化兩個版本值供日後比對使用。

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
