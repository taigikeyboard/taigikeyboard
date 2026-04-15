package com.siansiansu.taigikeyboard.util

import android.content.ComponentName
import android.content.Context
import android.content.pm.PackageManager

// Controls app launcher icon visibility via component enabled state
object LauncherIconUtils {
    private const val SETTINGS_ACTIVITY_NAME = "com.siansiansu.taigikeyboard.SettingsLauncherAlias"

    fun hideAppIcon(context: Context) {
        setLauncherIconEnabled(context, enabled = false)
    }

    fun showAppIcon(context: Context) {
        setLauncherIconEnabled(context, enabled = true)
    }

    private fun setLauncherIconEnabled(
        context: Context,
        enabled: Boolean,
    ) {
        val state =
            if (enabled) {
                PackageManager.COMPONENT_ENABLED_STATE_ENABLED
            } else {
                PackageManager.COMPONENT_ENABLED_STATE_DISABLED
            }
        context.packageManager.setComponentEnabledSetting(
            ComponentName(context, SETTINGS_ACTIVITY_NAME),
            state,
            PackageManager.DONT_KILL_APP,
        )
    }
}
