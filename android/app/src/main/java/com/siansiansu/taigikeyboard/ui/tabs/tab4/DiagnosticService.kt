package com.siansiansu.taigikeyboard.ui.tabs.tab4

import android.content.Context
import android.os.Build
import com.siansiansu.taigikeyboard.BuildConfig

private const val UNKNOWN_VERSION = "unknown"

// Snapshot of local device and app state for user-initiated bug reports.
// No data is transmitted — the user controls when and how to share.
data class DiagnosticInfo(
    val appVersion: String,
    val buildNumber: Int,
    val osVersion: String,
    val deviceModel: String,
) {
    fun formatted(): String =
        """
            |App: v$appVersion ($buildNumber)
            |OS: Android $osVersion (API ${Build.VERSION.SDK_INT})
            |Device: $deviceModel
        """.trimMargin().trim()
}

// Gathers local diagnostic information on demand.
object DiagnosticService {
    fun gather(context: Context): DiagnosticInfo {
        val appVersion =
            try {
                context.packageManager.getPackageInfo(context.packageName, 0).versionName ?: UNKNOWN_VERSION
            } catch (_: Exception) {
                UNKNOWN_VERSION
            }

        return DiagnosticInfo(
            appVersion = appVersion,
            buildNumber = BuildConfig.VERSION_CODE,
            osVersion = Build.VERSION.RELEASE,
            deviceModel = "${Build.MANUFACTURER} ${Build.MODEL}",
        )
    }
}
